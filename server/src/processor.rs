use std::net::SocketAddr;
use std::iter;

use async_std::sync::{Mutex, MutexGuard};
use common::{message::{Request, Response}, player_state::Looker};
use fnv::FnvHashMap;
use itertools::Itertools;
use log::*;

use crate::state::State;

/// Processes a request, and returns a map from peers to responses to send to those peers.
pub(crate) fn process_request(req: Request, requester: SocketAddr, state: &mut State) -> FnvHashMap<SocketAddr, Vec<Response>> {
    match req {
        Request::SetUsername{ name } => {
            state.set_username(requester, name.clone());
            let added = state.game_mut().add_player(requester, name.clone());
            if !added {
                state.game_mut().add_spectator(requester, name);
            }

            let usernames = state.game().players().iter().map(|player| player.username().clone())
                .collect_vec();
            state.peers().iter().map(|(addr, peer)| {
                (*addr, vec![
                    Some(Response::Usernames{ names: usernames.clone() }),
                    (!added && state.game().started()).then(|| Response::State {
                        game: state.game().game().clone(),
                        state: state.game().state().as_ref().expect("Game should have started").visible_state(Looker::Spectator)
                    })
                ].into_iter().flatten().collect())
            }).collect()
        },

        Request::RemovePeer{ addr } => {
            if state.game().started() {
                // TODO: Handle a player quitting
                FnvHashMap::default()
            } else {
                if state.game_mut().remove_player(addr) {
                    let usernames = state.game().players().iter().map(|player| player.username().clone())
                        .collect_vec();
                    state.peers().iter().map(|(addr, peer)| {
                        (*addr, vec![Response::Usernames{ names: usernames.clone() }])
                    }).collect()
                } else {
                    state.game_mut().remove_spectator(addr);
                    FnvHashMap::default()
                }
            }
        },

        Request::StartGame => {
            let players_spectators = state.game().players_and_spectators().cloned().collect_vec();
            if !state.game().started() {
                state.game_mut().start();
                state.peers().iter().map(|(addr, _)| {(*addr,
                    if let Some(index) = players_spectators.iter().position(|p| p.addr() == addr) { vec![
                        Response::State {
                            game: state.game().game().clone(),
                            state: state.game().state().as_ref().expect("Game should have started")
                                .visible_state(if (index as u32) < state.game().num_players() {
                                    Looker::Player(index as u32)
                                } else {
                                    Looker::Spectator
                                })
                        }
                    ]} else { vec![] }
                )}).collect()
            } else {
                FnvHashMap::default()
            }
        }

        Request::PlaceToken{ player, port } => {
            if let (game, Some(game_state)) = state.game_mut().game_and_state_mut() {
                if game_state.can_place_player(game, &port) {
                    game_state.place_player(player, &port);
                    let all_placed = game_state.all_players_placed();
                    let turn_player = game_state.turn_player();

                    state.peers().iter().map(|(addr, _)| {(*addr,
                        if let Some(index) = state.game().players_and_spectators().position(|p| p.addr() == addr) { vec![
                            Some(Response::PlacedToken { player, port: port.clone() }),
                            all_placed.then(|| Response::AllPlacedTokens),
                            (all_placed && turn_player == index as u32).then(|| Response::YourTurn),
                        ].into_iter().flatten().collect()} else { vec![] }
                    )}).collect()
                } else {
                    iter::once((requester, vec![Response::Rejected])).collect()
                }
            } else {
                warn!("Game state is missing");
                FnvHashMap::default()
            }
        }

        Request::PlaceTile{ player, kind, index, action, loc } => {
            if let (game, Some(game_state)) = state.game_mut().game_and_state_mut() {
                if game_state.can_place_tile(game, player, &kind, index, &action, &loc) {
                    let result = game_state.take_turn_placing_tile(game, &kind, index, &action, &loc);
                    let turn_player = game_state.turn_player();
                    let game_over = result.winners().len() > 0;

                    state.peers().iter().map(|(addr, _)| {(*addr,
                        if let Some(i) = state.game().players_and_spectators().position(|p| p.addr() == addr) { vec![
                            Some(Response::PlacedTile { player, kind: kind.clone(), index, action: action.clone(), loc: loc.clone() }),
                            (turn_player == i as u32 && !game_over).then(|| Response::YourTurn),
                        ].into_iter().flatten().collect()} else { vec![] }
                    )}).collect()
                } else {
                    iter::once((requester, vec![Response::Rejected])).collect()
                }
            } else {
                warn!("Game state is missing");
                FnvHashMap::default()
            }
        }
    }
}

/// Processes and responds to a request.
pub(crate) async fn respond_to_request(req: Request, requester: SocketAddr, state: &Mutex<State>) {
    info!("Received request from {}: {:?}", requester, req);
    let mut state = state.lock().await;
    
    let responses = process_request(req, requester, &mut state);
    for (addr, responses) in responses {
        for resp in responses {
            state.peer(addr).tx().unbounded_send(resp).ok();
        }
    }
}