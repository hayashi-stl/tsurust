use std::net::SocketAddr;

use common::{game::{BaseGame, GameId}, game_state::BaseGameState};
use getset::{Getters, CopyGetters};

#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct Player {
    #[getset(get_copy = "pub")]
    addr: SocketAddr,
    #[getset(get = "pub")]
    username: String,
}

#[derive(Debug, Getters, CopyGetters)]
pub struct GameInstance {
    #[getset(get_copy = "pub")]
    id: GameId,
    #[getset(get = "pub")]
    game: BaseGame,
    /// None if the game hasn't started
    #[getset(get = "pub")]
    state: Option<BaseGameState>,
    /// stores address and username
    #[getset(get = "pub")]
    players: Vec<Player>, 
    #[getset(get = "pub")]
    spectators: Vec<Player>,
}

impl GameInstance {
    pub fn new(id: GameId, game: BaseGame) -> Self {
        Self {
            id,
            game,
            state: None,
            players: vec![],
            spectators: vec![]
        }
    }

    pub fn to_common(&self) -> common::GameInstance {
        common::GameInstance::new(
            self.id,
            self.game.clone(),
            self.state.clone(),
            self.players.iter().map(|player| player.username().clone()).collect(),
        )
    }

    /// Whether the game has started
    pub fn started(&self) -> bool {
        self.state.is_some()
    }

    /// Adds a player to the game by address and username, replacing the address
    /// if the username is already in the game. Does not add new players if the game has started.
    /// Returns the player's index if they got added or their address got replaced.
    pub fn add_player(&mut self, addr: SocketAddr, username: String) -> Option<u32> {
        if let Some((index, player)) = self.players.iter_mut().enumerate()
            .find(|(_i, player)| player.username == username)
        {
            player.addr = addr;
            Some(index as u32)
        } else if !self.started() {
            self.players.push(Player { addr, username });
            Some(self.players.len() as u32 - 1)
        } else { None }
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

    /// Adds a spectator to the game by address and username, replacing the address if the
    /// username already exists.
    pub fn add_spectator(&mut self, addr: SocketAddr, username: String) {
        if let Some((_index, spectator)) = self.spectators.iter_mut().enumerate()
            .find(|(_i, spectator)| spectator.username == username)
        {
            spectator.addr = addr;
        } else {
            self.spectators.push(Player { addr, username })
        }
    }

    /// Removes a spectator from the game. Does nothing if they weren't in the game.
    pub fn remove_spectator(&mut self, addr: SocketAddr) {
        if let Some(pos) = self.spectators.iter().position(|player| player.addr == addr) {
            self.spectators.remove(pos);
        }
    }

    pub fn num_players(&self) -> u32 {
        self.players.len() as u32
    }

    /// Start the game. Adding players is not allowed afterward.
    pub fn start(&mut self) {
        self.state = Some(self.game.new_state(self.players.len() as u32));
    }

    /// Gets the state mutably
    pub fn state_mut(&mut self) -> Option<&mut BaseGameState> {
        self.state.as_mut()
    }

    /// Gets the game immutably and the state mutably
    pub fn game_and_state_mut(&mut self) -> (&BaseGame, Option<&mut BaseGameState>) {
        (&self.game, self.state.as_mut())
    }

    /// Iterates over all players and spectators.
    /// Players come first.
    pub fn players_and_spectators(&self) -> impl Iterator<Item = &Player> + Clone {
        self.players().iter().chain(self.spectators())
    }
}