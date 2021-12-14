use std::{net::SocketAddr, collections::VecDeque};
use std::iter;

use async_std::sync::{Mutex, MutexGuard};
use common::{message::{Request, Response}, player_state::Looker, board::{RectangleBoard, Board, BasePort, BaseTLoc}, game::{PathGame, GameId}, WrapBase, tile::{BaseKind, BaseGAct}};
use fnv::FnvHashMap;
use itertools::{Itertools, chain};
use log::*;

use crate::state::State;

/// A request for which a simple action is done.
/// This can generate more `ElementaryRequest`s as well as responses.
#[derive(Clone, Debug)]
pub enum ElementaryRequest {
    SetUsername{ username: String },
    JoinLobby,
    /// Elementary only. Does not send a response.
    LeaveLobby,
    CreateGame,
    JoinGame{ id: GameId },
    /// Elementary only. Does not send a response.
    LeaveGame{ id: GameId },
    /// Elementary only. Does not send a response.
    LeaveGames,
    /// Elementary only. Notifies the players of the game that the list of players changed.
    NotifyChangePlayers{ id: GameId },
    /// Elementary only. Notifies the lobby that a game changed.
    NotifyChangeGame{ id: GameId },
    StartGame{ id: GameId },
    PlaceToken{ id: GameId, player: u32, port: BasePort },
    PlaceTile{ id: GameId, player: u32, kind: BaseKind, index: u32, action: BaseGAct, loc: BaseTLoc },
}

impl ElementaryRequest {
    fn vec_from_request(req: Request) -> Vec<Self> {
        match req {
            Request::SetUsername{ username } => vec![Self::SetUsername{ username }],
            Request::JoinLobby => vec![Self::LeaveGames, Self::JoinLobby],
            Request::CreateGame => vec![Self::CreateGame],
            Request::JoinGame{ id } => vec![Self::LeaveLobby, Self::JoinGame{ id }],
            Request::StartGame{ id } => vec![Self::StartGame{ id }],
            Request::PlaceToken{ id, player, port } => vec![Self::PlaceToken{ id, player, port }],
            Request::PlaceTile{ id, player, kind, index, action, loc } =>
                vec![Self::PlaceTile{ id, player, kind, index, action, loc }],
            Request::RemovePeer => vec![Self::LeaveGames, Self::LeaveLobby],
        }
    }
}

/// Processes a request, and returns a list of responses to send to peers.
pub(crate) fn process_request(req: Request, requester: SocketAddr, state: &mut State) -> Vec<(SocketAddr, Response)> {
    let elem_req = ElementaryRequest::vec_from_request(req);

    let mut to_process = elem_req.into_iter().collect::<VecDeque<_>>();
    let mut responses = vec![];
    while let Some(req) = to_process.pop_front() {
        responses.extend(match req {
            ElementaryRequest::SetUsername{ username: name } => {
                if state.set_username(requester, name.clone()) {
                    to_process.push_back(ElementaryRequest::JoinLobby);
                    vec![]
                } else {
                    vec![(requester, Response::RejectedUsername)]
                }
            },

            ElementaryRequest::CreateGame => {
                let board = RectangleBoard::new(6, 6, 2);
                let start_ports = board.boundary_ports();
                let game = PathGame::new(
                    RectangleBoard::new(6, 6, 2),
                    start_ports,
                    [((), 3)],
                ).wrap_base();
                
                let game = state.add_game(game).to_common();
                to_process.push_back(ElementaryRequest::NotifyChangeGame{ id: game.id() });
                vec![]
            }

            ElementaryRequest::JoinGame{ id } => {
                let username = state.peer(requester).expect("Peer doesn't exist").username().clone();

                if let Some(game) = state.game_mut(id) {
                    let index = game.add_player(requester, username.clone());
                    if index.is_none() {
                        game.add_spectator(requester, username);
                    }

                    if index.is_some() {
                        to_process.extend([
                            ElementaryRequest::NotifyChangePlayers{ id },
                            ElementaryRequest::NotifyChangeGame{ id },
                        ])
                    }
                    [
                        Some((requester, Response::JoinedGame{ game: game.to_common() })),
                        game.state().as_ref().map_or(false, |state| index == Some(state.turn_player()))
                            .then(|| (requester, Response::YourTurn{ id }))
                    ].into_iter().flatten().collect()
                } else { vec![(requester, Response::Rejected{ id })] }
            }

            ElementaryRequest::LeaveGame{ id } => {
                if let Some(game) = state.game_mut(id) {
                    if game.remove_player(requester) {
                        to_process.push_back(ElementaryRequest::NotifyChangePlayers{ id });
                        vec![]
                    } else {
                        game.remove_spectator(requester);
                        vec![]
                    }
                } else { vec![] }
            }

            ElementaryRequest::LeaveGames => {
                to_process.extend(state.games().iter().map(|game| ElementaryRequest::LeaveGame{ id: game.id() }));
                vec![]
            }

            ElementaryRequest::JoinLobby => {
                let username = state.peer(requester).expect("Peer doesn't exist").username().clone();
                state.add_to_lobby(username, requester);
                let games = state.games().iter().map(|game| game.to_common()).collect();
                vec![(requester, Response::JoinedLobby{ games })]
            }

            ElementaryRequest::LeaveLobby => {
                state.remove_from_lobby_by_addr(requester);
                vec![]
            }

            ElementaryRequest::NotifyChangeGame{ id } => {
                // This can be proven to work without relying on the user input being good
                let game = state.game(id).expect("NotifyChangeGame requested on nonexistent game");

                state.lobby().iter().map(|(_, addr)|
                    (*addr, Response::ChangedGame{ game: game.to_common() })
                ).collect()
            }

            ElementaryRequest::NotifyChangePlayers{ id } => {
                // This can be proven to work without relying on the user input being good
                let game = state.game(id).expect("NotifyChangePlayers requested on nonexistent game");

                let usernames = game.players().iter().map(|player| player.username().clone())
                    .collect_vec();
                game.players_and_spectators().map(|player|
                    (player.addr(), Response::ChangedPlayers{ id, names: usernames.clone() })
                ).collect()
            }

            ElementaryRequest::StartGame{ id } => {
                if let Some(game) = state.game_mut(id) {
                    let players_spectators = game.players_and_spectators().cloned().collect_vec();
                    if !game.started() {
                        game.start();
                        let game = state.game(id).unwrap(); // no more need for the mutable borrow

                        to_process.push_back(ElementaryRequest::NotifyChangeGame{ id });

                        let game_state = game.state().as_ref()
                            .expect("Game started, there should be a state");
                        players_spectators.into_iter().enumerate().map(|(index, user)| {
                            let this_state = game_state.visible_state(if (index as u32) < game.num_players() {
                                    Looker::Player(index as u32)
                                } else {
                                    Looker::Spectator
                                });
                            (user.addr(), Response::StartedGame { id, state: this_state })
                        })
                        .chain(state.lobby().values().map(|addr| (
                            *addr, Response::ChangedGame{ game: game.to_common() }
                        )))
                        .collect()
                    } else { vec![(requester, Response::Rejected{ id })] }
                } else { vec![(requester, Response::Rejected{ id })] }
            }

            ElementaryRequest::PlaceToken{ id, player, port } => {
                if let Some(inst) = state.game_mut(id) {
                    if let (game, Some(game_state)) = inst.game_and_state_mut() {
                        if game_state.can_place_player(game, &port) {
                            game_state.place_player(player, &port);
                            let all_placed = game_state.all_players_placed();
                            let turn_player = game_state.turn_player();

                            inst.players_and_spectators().into_iter()
                                .flat_map(|user| { vec![
                                    Some((user.addr(), Response::PlacedToken { id, player, port: port.clone() })),
                                    all_placed.then(|| (user.addr(), Response::AllPlacedTokens{ id })),
                                ].into_iter().flatten()})
                                .chain(all_placed.then(|| (inst.players()[turn_player as usize].addr(), Response::YourTurn{ id })))
                                .collect()
                        } else {
                            vec![(requester, Response::Rejected{ id })]
                        }
                    } else {
                        warn!("Game state is missing");
                        vec![(requester, Response::Rejected{ id })]
                    }
                } else { vec![(requester, Response::Rejected{ id })] }
            }

            ElementaryRequest::PlaceTile{ id, player, kind, index, action, loc } => {
                if let Some(inst) = state.game_mut(id) {
                    if let (game, Some(game_state)) = inst.game_and_state_mut() {
                        if game_state.can_place_tile(game, player, &kind, index, &action, &loc) {
                            let result = game_state.take_turn_placing_tile(game, &kind, index, &action, &loc);
                            let turn_player = game_state.turn_player();
                            let game_over = result.game_over();
                            
                            if game_over {
                                to_process.push_back(ElementaryRequest::NotifyChangeGame{ id });
                            }

                            inst.players_and_spectators().into_iter()
                                .map(|user| { 
                                    (user.addr(), Response::PlacedTile {
                                        id, player, kind: kind.clone(), index: index as u32, action: action.clone(), loc: loc.clone()
                                    })
                                })
                                .chain((!game_over).then(|| (inst.players()[turn_player as usize].addr(), Response::YourTurn{ id })))
                                .collect()
                        } else {
                            vec![(requester, Response::Rejected{ id })]
                        }
                    } else {
                        warn!("Game state is missing");
                        vec![(requester, Response::Rejected{ id })]
                    }
                } else { vec![(requester, Response::Rejected{ id })] }
            }
        })
    }

    responses
}

/// Processes and responds to a request.
pub(crate) async fn respond_to_request(req: Request, requester: SocketAddr, state: &Mutex<State>) {
    info!("Received request from {}: {:?}", requester, req);
    let mut state = state.lock().await;
    
    let responses = process_request(req, requester, &mut state);
    for (addr, resp) in responses {
        if let Some(peer) = state.peer(addr) {
            if let Err(resp) = peer.tx().unbounded_send(resp) {
                warn!("Failed to send response to {}: {:?}", addr, resp);
            }
        } else {
            warn!("Failed to send response to {}: peer was disconnected, attempted response: {:?}", addr, resp);
        }
    }
}