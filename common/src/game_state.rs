use std::collections::VecDeque;

use fnv::FnvHashMap;
use itertools::Itertools;

use crate::{board::Board, board_state::BoardState, game::Game, player_state::PlayerState, tile::Tile};

/// The state of the game
#[derive(Clone, Debug)]
pub struct GameState<G: Game> {
    board_state: BoardState<G::Board, G::Tile>,
    player_states: Vec<Option<PlayerState<G::Tile>>>,
    curr_player: u32,
    tiles: FnvHashMap<G::Kind, VecDeque<G::Tile>>
}

impl<G: Game> GameState<G> {
    /// Construct a new state from a game
    pub fn new(game: &G, num_players: u32) -> Self {
        let mut tiles = game.all_tiles();
        // TODO: Shuffle tiles first
        tiles.sort_by_key(|tile| tile.kind());
        let groups = tiles.into_iter().group_by(|tile| tile.kind());
        let tiles = groups.into_iter().map(|(kind, tiles)|
            (kind, tiles.collect::<VecDeque<_>>())).collect::<FnvHashMap<_, _>>();

        let mut state = Self {
            board_state: BoardState::new(game, num_players),
            player_states: vec![Some(PlayerState::new(game)); num_players as usize],
            curr_player: 0,
            tiles,
        };

        // deal tiles
        for kind in game.board().all_kinds() {
            let num_tiles = game.num_tiles_per_player(kind.clone());
            (0..num_players).cycle().take((num_tiles * num_players) as usize).map(|player| {
                state.deal_tile(player, kind.clone())
            }).all(|b| b);
        }

        state
    }

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

    /// Place a tile on some location on the board. Assumes the location is empty and kinds match.
    pub fn place_tile(&mut self, tile: G::Tile, loc: G::TLoc) {
        self.board_state.place_tile(loc, tile)
    }

    /// Have a player place a tile with some kind from some position in their hand to a location on the board.
    /// For now, assumes the player is alive.
    pub fn player_place_tile(&mut self, player: u32, kind: G::Kind, index: u32, loc: G::TLoc) {
        let tile = self.player_states[player as usize].as_mut().unwrap().remove_tile(kind, index);
        self.place_tile(tile, loc)
    }

    /// Move players that touch a tile along their respective paths until they face a dead end.
    /// Assumes the location has a tile on it.
    pub fn advance_players(&mut self, board: &G::Board, loc: G::TLoc) {
        self.board_state.advance_players(board, loc);
    }
}

#[cfg(test)]
mod tests {
    use crate::{board::RectangleBoard, game::PathGame, tile::RegularTile};

    use super::*;

    #[test]
    fn test_game_state_new() {
        let board = RectangleBoard::new(6, 6, 2);
        let start_ports = board.boundary_ports();
        let game = PathGame::<_, RegularTile<4>>::new(board, start_ports, [((), 3)]);
        let state = GameState::new(&game, 4);

        for player in 0..state.num_players() {
            let tiles = state.player_state(player).unwrap().tiles();
            assert_eq!(tiles[&()].len(), 3);
            assert_eq!(state.board_state().player_port(player), None);
        }
    }
}