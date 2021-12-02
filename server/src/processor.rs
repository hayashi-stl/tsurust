use std::net::SocketAddr;
use std::iter;

use async_std::sync::{Mutex, MutexGuard};
use common::{message::{Request, Response}, player_state::Looker};
use fnv::FnvHashMap;
use itertools::Itertools;
use log::*;

use crate::state::State;

/// Processes a request, and returns a list of responses to send to peers.
pub(crate) fn process_request(req: Request, requester: SocketAddr, state: &mut State) -> Vec<(SocketAddr, Response)> {
    match req {
        Request::SetUsername{ name } => {
            if state.set_username(requester, name.clone()) {
                let index = state.game_mut().add_player(requester, name.clone());
                if index.is_none() {
                    state.game_mut().add_spectator(requester, name.clone());
                }

                let usernames = state.game().players().iter().map(|player| player.username().clone())
                    .collect_vec();
                state.peers().iter().map(|(addr, peer)| (*addr, Response::Usernames{ names: usernames.clone() }))
                    .chain((state.game().started()).then(|| (requester, Response::State {
                        game: state.game().game().clone(),
                        state: state.game().state().as_ref().unwrap()
                            .visible_state(index.map_or(Looker::Spectator, |i| Looker::Player(i)))
                    })))
                    .chain(state.game().state().as_ref().map_or(false, |state| index == Some(state.turn_player()))
                        .then(|| (requester, Response::YourTurn)))
                    .collect()
            } else {
                vec![(requester, Response::Rejected)]
            }
        },

        Request::RemovePeer{ addr } => {
            if state.game().started() {
                // TODO: Handle a player quitting
                vec![]
            } else {
                if state.game_mut().remove_player(addr) {
                    let usernames = state.game().players().iter().map(|player| player.username().clone())
                        .collect_vec();
                    state.peers().iter().map(|(addr, peer)| (*addr, Response::Usernames{ names: usernames.clone() }))
                        .collect()
                } else {
                    state.game_mut().remove_spectator(addr);
                    vec![]
                }
            }
        },

        Request::StartGame => {
            let players_spectators = state.game().players_and_spectators().cloned().collect_vec();
            if !state.game().started() {
                state.game_mut().start();
                players_spectators.into_iter().enumerate().map(|(index, user)| {
                    (user.addr(), Response::State {
                        game: state.game().game().clone(),
                        state: state.game().state().as_ref().expect("Game should have started")
                            .visible_state(if (index as u32) < state.game().num_players() {
                                Looker::Player(index as u32)
                            } else {
                                Looker::Spectator
                            })
                    })
                }).collect()
            } else {
                vec![]
            }
        }

        Request::PlaceToken{ player, port } => {
            if let (game, Some(game_state)) = state.game_mut().game_and_state_mut() {
                if game_state.can_place_player(game, &port) {
                    game_state.place_player(player, &port);
                    let all_placed = game_state.all_players_placed();
                    let turn_player = game_state.turn_player();

                    state.game().players_and_spectators().into_iter()
                        .flat_map(|user| { vec![
                            Some((user.addr(), Response::PlacedToken { player, port: port.clone() })),
                            all_placed.then(|| (user.addr(), Response::AllPlacedTokens)),
                        ].into_iter().flatten()})
                        .chain(all_placed.then(|| (state.game().players()[turn_player as usize].addr(), Response::YourTurn)))
                        .collect()
                } else {
                    vec![(requester, Response::Rejected)]
                }
            } else {
                warn!("Game state is missing");
                vec![]
            }
        }

        Request::PlaceTile{ player, kind, index, action, loc } => {
            if let (game, Some(game_state)) = state.game_mut().game_and_state_mut() {
                if game_state.can_place_tile(game, player, &kind, index, &action, &loc) {
                    let result = game_state.take_turn_placing_tile(game, &kind, index, &action, &loc);
                    let turn_player = game_state.turn_player();
                    let game_over = result.game_over();

                    state.game().players_and_spectators().into_iter()
                        .map(|user| { 
                            (user.addr(), Response::PlacedTile { player, kind: kind.clone(), index: index as u32, action: action.clone(), loc: loc.clone() })
                        })
                        .chain((!game_over).then(|| (state.game().players()[turn_player as usize].addr(), Response::YourTurn)))
                        .collect()
                } else {
                    vec![(requester, Response::Rejected)]
                }
            } else {
                warn!("Game state is missing");
                vec![]
            }
        }
    }
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