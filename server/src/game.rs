use std::net::SocketAddr;

use common::{game::{BaseGame, GenericGame}, game_state::BaseGameState};
use getset::Getters;

#[derive(Debug, Getters)]
pub(crate) struct Player {
    #[getset(get = "pub")]
    addr: SocketAddr,
    #[getset(get = "pub")]
    username: String,
}

#[derive(Debug, Getters)]
pub(crate) struct GameInstance {
    #[getset(get = "pub")]
    game: BaseGame,
    /// None if the game hasn't started
    #[getset(get = "pub")]
    state: Option<BaseGameState>,
    /// stores address and username
    #[getset(get = "pub")]
    players: Vec<Player>, 
}

impl GameInstance {
    pub fn new(game: BaseGame) -> Self {
        Self {
            game,
            state: None,
            players: vec![]
        }
    }

    /// Whether the game has started
    pub fn started(&self) -> bool {
        self.state.is_some()
    }

    /// Adds a player to the game by address and username. Does nothing if the game has started.
    /// Returns whether the player got added.
    pub fn add_player(&mut self, addr: SocketAddr, username: String) -> bool {
        if !self.started() {
            self.players.push(Player { addr, username });
            true
        } else { false }
    }

    /// Removes a player from the game. Returns whether the player was in the game.
    /// TODO: If the game has started, kill the player token.
    pub fn remove_player(&mut self, addr: SocketAddr) -> bool {
        if !self.started() {
            if let Some(pos) = self.players.iter().position(|player| player.addr == addr) {
                self.players.remove(pos);
                true
            } else { false }
        } else { false }
    }

    /// Start the game. Adding players is not allowed afterward.
    pub fn start(&mut self) {
        self.state = Some(self.game.new_state(self.players.len() as u32));
    }
}