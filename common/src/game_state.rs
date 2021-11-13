use std::collections::VecDeque;

use fnv::FnvHashMap;

use crate::{board_state::BoardState, game::Game, player_state::PlayerState};

/// The state of the game
pub struct GameState<G: Game> {
    board_state: BoardState<G::Board, G::Tile>,
    player_states: Vec<Option<PlayerState<G::Tile>>>,
    curr_player: u32,
    tiles: FnvHashMap<G::Kind, VecDeque<G::Tile>>
}

impl<G: Game> GameState<G> {
    /// The state of the game's board
    pub fn board_state(&self) -> &BoardState<G::Board, G::Tile> {
        &self.board_state
    }

    /// The state of a specific player. None if the player is dead.
    pub fn player_state(&self, player: u32) -> Option<&PlayerState<G::Tile>> {
        self.player_states[player as usize].as_ref()
    }

    /// Number of players in the game
    pub fn num_players(&self) -> u32 {
        self.player_states.len() as u32
    }

    /// Who's turn it is
    pub fn curr_player(&self) -> u32 {
        self.curr_player
    }

    /// Gets the next tile by kind and updates the state. None if there's no tiles left of that kind
    pub fn next_tile(&mut self, kind: G::Kind) -> Option<G::Tile> {
        self.tiles.get_mut(&kind).expect("Each kind should have a list of tiles").pop_front()
    }

    /// Deals a tile of a specific kind to a specific player. Returns whether a tile was actually dealt
    pub fn deal_tile(&mut self, player: u32, kind: G::Kind) -> bool {
        self.next_tile(kind).zip(self.player_states[player as usize].as_mut())
            .map(|(tile, state)| state.add_tile(tile))
            .is_some()
    }
}