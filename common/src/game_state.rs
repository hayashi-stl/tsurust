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
        tiles.sort_by_key(|tile| tile.kind().clone());
        let groups = tiles.into_iter().group_by(|tile| tile.kind().clone());
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
            let num_tiles = game.num_tiles_per_player(&kind);
            (0..num_players).cycle().take((num_tiles * num_players) as usize).map(|player| {
                state.deal_tile(player, &kind)
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
    pub fn next_tile(&mut self, kind: &G::Kind) -> Option<G::Tile> {
        self.tiles.get_mut(kind).expect("Each kind should have a list of tiles").pop_front()
    }

    /// Deals a tile of a specific kind to a specific player. Returns whether a tile was actually dealt
    pub fn deal_tile(&mut self, player: u32, kind: &G::Kind) -> bool {
        self.next_tile(kind).zip(self.player_states[player as usize].as_mut())
            .map(|(tile, state)| state.add_tile(tile))
            .is_some()
    }

    /// Place a player on some port.
    pub fn place_player(&mut self, player: u32, port: &G::Port) {
        self.board_state.place_player(player, port)
    }

    /// Place a tile on some location on the board. Assumes the location is empty and kinds match.
    pub fn place_tile(&mut self, tile: G::Tile, loc: &G::TLoc) {
        self.board_state.place_tile(tile, loc)
    }

    /// Have a player place a tile with some kind from some position in their hand to a location on the board.
    /// For now, assumes the player is alive.
    pub fn player_place_tile(&mut self, player: u32, kind: &G::Kind, index: u32, loc: &G::TLoc) {
        let tile = self.player_states[player as usize].as_mut().unwrap().remove_tile(kind, index);
        self.place_tile(tile, loc)
    }

    /// Move players that touch a tile along their respective paths until they face a dead end.
    /// Assumes the location has a tile on it.
    /// Returns a list of dead players.
    pub fn advance_players(&mut self, board: &G::Board, loc: &G::TLoc) -> Vec<u32> {
        self.board_state.advance_players(board, loc)
    }

    /// Give remaining tiles to players so that for each tile kind,
    /// each player has as close to the game-specified number of tiles of that kind as possible,
    /// each player has either *n* or *n* - 1 tiles for some *n*,
    /// and players with *n* tiles go before players with *n* - 1 tiles.
    /// Prioritize giving tiles to players with less tiles, then players whose turn is sooner, if this is impossible.
    /// 
    /// This is intended to be called before updating whose turn it is.
    fn redistribute_tiles(&mut self, game: &G) {
        for kind in game.board().all_kinds() {
            let num_tiles = game.num_tiles_per_player(&kind);
            let curr_player = self.curr_player();
            let num_players = self.num_players();
            let deal_tile_order = (0..num_tiles)
                .flat_map(|i| (0..num_players).map(move |j| ((j + curr_player + 1) % num_players, i)))
                .flat_map(|(player, i)| self.player_state(player)
                    .filter(|state| state.num_tiles_by_kind(&kind) <= i)
                    .map(|_| player))
                .collect_vec();

            for player in deal_tile_order {
                if !self.deal_tile(player, &kind) {
                    break;
                }
            }
        }
    }

    /// Removes tiles from dead players.
    /// Assumes the players were just alive
    pub fn handle_dead_players(&mut self, game: &G, players: Vec<u32>) {
        let tiles = players.into_iter().flat_map(|player| {
            let tiles = self.player_states[player as usize].as_mut().unwrap().remove_all_tiles();
            self.player_states[player as usize] = None;
            tiles
        }).collect_vec();

        for tile in tiles {
            self.tiles.get_mut(&tile.kind()).unwrap().push_back(tile);
        }
    }

    /// Can someone place their token on the board on port `port`?
    pub fn can_place_player(&mut self, game: &G, port: &G::Port) -> bool {
        self.board_state.player_at(port).is_none() && game.start_ports().contains(&port)
    }

    /// Have the current player take a turn by placing their token on the board on port `port`.
    /// The turn is processed and then advances to the next player.
    pub fn take_turn_placing_player(&mut self, game: &G, port: &G::Port) {
        self.place_player(self.curr_player(), port);
        // All players should still be alive
        self.curr_player = (self.curr_player + 1) % self.num_players();
    }

    /// Can `player` place a tile of kind `kind` from index `index` in their hand to location `loc`?
    pub fn can_place_tile(&mut self, game: &G, player: u32, kind: &G::Kind, index: u32, loc: &G::TLoc) -> bool {
        self.player_states[player as usize].as_ref().map_or(false, |state| index < state.num_tiles_by_kind(kind)) &&
            self.board_state.player_port(player).map_or(false, |port|
                game.board().port_locs(port).contains(loc)) &&
            self.board_state.tile_at(loc).is_none() &&
            kind == &game.board().kind_at(loc)
    }

    /// Have the current player take a turn by placing a tile of kind `kind` from index `index` in their hand to location `loc`.
    /// The turn is processed and then advances to the next player.
    pub fn take_turn_placing_tile(&mut self, game: &G, kind: &G::Kind, index: u32, loc: &G::TLoc) {
        self.player_place_tile(self.curr_player(), kind, index, loc);
        let dead = self.advance_players(game.board(), loc);
        let players_died = dead.len() > 0;
        self.handle_dead_players(game, dead);
        if players_died {
            self.redistribute_tiles(game);
        } else {
            self.deal_tile(self.curr_player, kind);
        }

        if let Some(next) = (0..self.num_players()).cycle().skip(self.curr_player() as usize + 1).take(self.num_players() as usize)
            .filter(|player| self.player_state(*player).is_some()).next()
        {
            self.curr_player = next;
        } else {
            unimplemented!("What to do when all players are dead")
        }
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