use std::{net::SocketAddr, collections::{HashMap}};

use common::{message::Response};
use common::game::{GameId, BaseGame};

use fnv::FnvHashMap;
use futures::channel::mpsc::UnboundedSender;
use getset::{Getters, MutGetters};

use crate::game::{GameInstance};

type PeerMap = FnvHashMap<SocketAddr, Peer>;

#[derive(Debug, Getters, MutGetters)]
pub struct Peer {
    #[getset(get = "pub")]
    username: String,
    #[getset(get = "pub")]
    tx: UnboundedSender<Response>,
}

impl Peer {
}

#[derive(Debug, Getters, MutGetters)]
pub struct State {
    #[getset(get = "pub")]
    peers: PeerMap,
    /// Maps usernames to addresses
    inv_peers: HashMap<String, SocketAddr>,
    #[getset(get = "pub", get_mut = "pub")]
    games: Vec<GameInstance>,
    /// Map of players outside any game to their addresses
    #[getset(get = "pub")]
    lobby: HashMap<String, SocketAddr>,
    id_counter: u32,
}

impl State {
    pub fn new() -> Self {
        Self {
            peers: FnvHashMap::default(),
            inv_peers: HashMap::default(),
            games: vec![],
            lobby: HashMap::default(),
            id_counter: 0,
        }
    }

    pub fn add_to_lobby(&mut self, username: String, addr: SocketAddr) {
        self.lobby.insert(username, addr);
    }

    pub fn remove_from_lobby(&mut self, username: &str) {
        self.lobby.remove(username);
    }

    pub fn remove_from_lobby_by_addr(&mut self, addr: SocketAddr) {
        if let Some(peer) = self.peers.get(&addr) {
            self.lobby.remove(peer.username());
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

    pub fn peers_and_games_mut(&mut self) -> (&PeerMap, &mut [GameInstance]) {
        (&self.peers, &mut self.games)
    }

    /// Adds a game to the list and returns a reference to it.
    pub fn add_game(&mut self, game: BaseGame) -> &GameInstance {
        let id = GameId(self.id_counter);
        self.id_counter += 1;
        self.games.push(GameInstance::new(id, game));
        self.games.last().unwrap()
    }

    fn game_index(&self, id: GameId) -> Option<usize> {
        self.games.binary_search_by_key(&id, |game| game.id()).ok()
    }

    /// Gets a game by id, if it exists
    pub fn game(&self, id: GameId) -> Option<&GameInstance> {
        self.game_index(id).map(|i| &self.games[i])
    }

    /// Gets a game mutably by id, if it exists
    pub fn game_mut(&mut self, id: GameId) -> Option<&mut GameInstance> {
        self.game_index(id).map(|i| &mut self.games[i])
    }
}