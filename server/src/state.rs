use std::net::SocketAddr;

use common::{board::{Board, RectangleBoard}, game::PathGame, message::Response};
use fnv::FnvHashMap;
use futures::channel::mpsc::UnboundedSender;
use getset::{Getters, MutGetters};

use crate::game::GameInstance;


type PeerMap = FnvHashMap<SocketAddr, Peer>;

#[derive(Debug, Getters, MutGetters)]
pub(crate) struct Peer {
    #[getset(get = "pub")]
    username: String,
    #[getset(get = "pub")]
    tx: UnboundedSender<Response>,
}

impl Peer {
}

#[derive(Debug, Getters, MutGetters)]
pub(crate) struct State {
    #[getset(get = "pub")]
    peers: PeerMap,
    #[getset(get = "pub", get_mut = "pub")]
    game: GameInstance,
}

impl State {
    pub fn new() -> Self {
        let board = RectangleBoard::new(6, 6, 2);
        let start_ports = board.boundary_ports();
        let game = PathGame::new(
            RectangleBoard::new(6, 6, 2),
            start_ports,
            [((), 3)],
        );

        Self {
            peers: FnvHashMap::default(),
            game: GameInstance::new(game.into())
        }
    }

    /// Add a peer with a placeholder username
    pub fn add_peer(&mut self, addr: SocketAddr, tx: UnboundedSender<Response>) {
        self.peers.insert(addr, Peer { username: "???".to_owned(), tx });
    }
    
    /// Removes a peer
    pub fn remove_peer(&mut self, addr: SocketAddr) {
        self.peers.remove(&addr);
    }
    
    /// Set the username of a peer, assuming it exists.
    pub fn set_username(&mut self, addr: SocketAddr, username: String) {
        self.peers.get_mut(&addr)
            .expect("Expected peer to exist")
            .username = username;
    }

    /// Get the peer, assuming it exists.
    pub fn peer(&self, addr: SocketAddr) -> &Peer {
        &self.peers[&addr]
    }
}