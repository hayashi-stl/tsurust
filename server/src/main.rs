pub mod processor;
pub mod game;
pub mod state;

use std::{collections::HashMap, sync::Arc};

use async_std::{net::{SocketAddr, TcpListener, TcpStream}, sync::Mutex};
use async_tungstenite::{accept_async, tungstenite::{Error, Message, Result}};
use common::{game::BaseGame, game_state::BaseGameState, message::{Request, Response}};
use fnv::FnvHashMap;
use futures::{StreamExt, future::{self, Either}, pin_mut, prelude::*};
use futures::channel::mpsc::{self, UnboundedSender};
use log::*;

use crate::{processor::{process_request, respond_to_request}, state::State};

async fn accept_connection(peer: SocketAddr, stream: TcpStream, state: Arc<Mutex<State>>) {
    if let Err(e) = handle_connection(peer, stream, Arc::clone(&state)).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {}
            error => error!("Error processing connection: {}", error),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream, state: Arc<Mutex<State>>) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect(&format!("Failed to accept {}", peer));
    info!("New web socket connection: {}", peer);
    let (mut sink, mut stream) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded();
    {
        let mut state = state.lock().await;
        state.add_peer(peer, tx);
    }
    info!("Starting game with {}", peer);

    let stream_loop = async {
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            if let Message::Binary(msg) = msg {
                match bincode::deserialize::<Request>(&msg) {
                    Ok(req) => respond_to_request(req, peer, &state).await,
                    Err(err) => error!("Invalid request from {}: {:?}", peer, err),
                }
            }
        }

        Ok(())
    };

    // Actually sends the responses
    let receive_loop = async {
        while let Some(resp) = rx.next().await {
            match sink.send(bincode::serialize(&resp).unwrap().into()).await {
                Ok(_) => info!("Sent response to {}: {:?}", peer, resp),
                Err(err) => error!("Error sending response to {}: {:?}, error: {}", peer, resp, err),
            }
        }
        Ok(())
    };

    pin_mut!(stream_loop, receive_loop);
    let result = match future::select(stream_loop, receive_loop).await {
        Either::Left(result) => result.0,
        Either::Right(result) => result.0,
    };
    info!("{} disconnected", peer);
    state.lock().await.remove_peer(peer);
    respond_to_request(Request::RemovePeer{ addr: peer }, peer, &state).await;
    result
}

async fn run() {
    env_logger::builder().filter_level(log::LevelFilter::Info).parse_default_env().init();

    let state = Arc::new(Mutex::new(State::new()));

    info!("Attempting to listen to {}", common::HOST_ADDRESS);
    let listener = TcpListener::bind(common::HOST_ADDRESS).await
        .expect(&format!("Can't listen to {}", common::HOST_ADDRESS));
    info!("Listening on {}", common::HOST_ADDRESS);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("Connected streams should have a peer address");
        info!("Peer address {}", peer);

        async_std::task::spawn(accept_connection(peer, stream, Arc::clone(&state)));
    }
}

fn main() {
    async_std::task::block_on(run());
}