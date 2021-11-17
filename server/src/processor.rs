use std::net::SocketAddr;

use async_std::sync::{Mutex, MutexGuard};
use common::message::{Request, Response};
use fnv::FnvHashMap;
use itertools::Itertools;
use log::*;

use crate::state::State;

/// Processes a request, and returns a map from peers to responses to send to those peers.
pub(crate) fn process_request(req: Request, requester: SocketAddr, state: &mut State) -> FnvHashMap<SocketAddr, Vec<Response>> {
    match req {
        Request::SetUsername{ name } => {
            state.set_username(requester, name.clone());
            state.game_mut().add_player(requester, name);

            let usernames = state.game().players().iter().map(|player| player.username().clone())
                .collect_vec();
            state.peers().iter().map(|(addr, peer)| {
                (*addr, vec![Response::Usernames{ names: usernames.clone() }])
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
                } else { FnvHashMap::default() }
            }
        },

        Request::StartGame => {
            state.game_mut().start();
            FnvHashMap::default()
        }

        _ => FnvHashMap::default(),
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