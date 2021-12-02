use std::{net::SocketAddr, collections::HashMap};

use common::{board::{Board, RectangleBoard}, game::PathGame, message::Response};
use common::WrapBase;
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
    /// Maps usernames to addresses
    inv_peers: HashMap<String, SocketAddr>,
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
            [((), 11)],
        );

        Self {
            peers: FnvHashMap::default(),
            inv_peers: HashMap::default(),
            game: GameInstance::new(game.wrap_base())
        }
    }

    /// Add a peer with a placeholder username
    pub fn add_peer(&mut self, addr: SocketAddr, tx: UnboundedSender<Response>) {
        self.peers.insert(addr, Peer { username: "???".to_owned(), tx });
    }
    
    /// Removes a peer
    pub fn remove_peer(&mut self, addr: SocketAddr) {
        if let Some(username) = self.peers.get(&addr).map(|peer| peer.username()) {
            self.inv_peers.remove(username);
        }
        self.peers.remove(&addr);
    }
    
    /// Set the username of a peer, assuming it exists.
    /// Returns false instead if the username is not unique.
    pub fn set_username(&mut self, addr: SocketAddr, username: String) -> bool {
        if self.inv_peers.contains_key(&username) {
            false
        } else {
            self.peers.get_mut(&addr)
                .expect("Expected peer to exist")
                .username = username.clone();
            self.inv_peers.insert(username, addr);
            true
        }
    }

    /// Get the peer, if it exists.
    pub fn peer(&self, addr: SocketAddr) -> Option<&Peer> {
        self.peers.get(&addr)
    }
}