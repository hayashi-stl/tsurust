use std::collections::VecDeque;

use fnv::FnvHashMap;
use getset::{CopyGetters, Getters};
use itertools::Itertools;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};


use crate::{board::{BasePort, BaseTLoc, Board, TLoc}, board_state::BoardState, game::{Game}, pcg64, player_state::{Looker, PlayerState}, tile::{BaseKind, Tile, Kind}};
use crate::tile::{BaseTile, GAct, BaseGAct};
use crate::board_state::BaseBoardState;
use crate::board::Port;
use crate::player_state::{BasePlayerState, LookerTag};
use crate::game::BaseGame;
use crate::WrapBase;

#[macro_export]
macro_rules! for_each_game_state {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::game_state::BaseGameState)::Normal: $crate::game_state::GameState<
                $crate::game::PathGame<$crate::board::RectangleBoard, $crate::tile::RegularTile<4>>
            >,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_game_state! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_game_state! {
    p::x, t =>
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum BaseGameState {
        $($x($t)),*
    }

    impl BaseGameState {
        pub fn visible_state(&self, looker: Looker) -> BaseGameState {
            match self { $($($p)*::$x(s) => s.visible_state(looker).wrap_base()),* }
        }

        /// Can someone place their token on the board on port `port`?
        pub fn can_place_player(&mut self, game: &BaseGame, port: &BasePort) -> bool {
            match self { $($($p)*::$x(s) => s.can_place_player(
                <$t as GameStateT>::Game::unwrap_base_ref(game),
                <<$t as GameStateT>::Game as Game>::Port::unwrap_base_ref(port),
            )),* }
        }

        /// Can `player` place a tile of kind `kind` from index `index` in their hand transformed by group action `action` to location `loc`?
        pub fn can_place_tile(&mut self, game: &BaseGame, player: u32, kind: &BaseKind, index: u32, action: &BaseGAct, loc: &BaseTLoc) -> bool {
            match self { $($($p)*::$x(s) => s.can_place_tile(
                <$t as GameStateT>::Game::unwrap_base_ref(game),
                player,
                <<$t as GameStateT>::Game as Game>::Kind::unwrap_base_ref(kind),
                index,
                <<$t as GameStateT>::Game as Game>::GAct::unwrap_base_ref(action),
                <<$t as GameStateT>::Game as Game>::TLoc::unwrap_base_ref(loc),
            )),* }
        }

        /// The player looking at this state, or None if no specific person
        pub fn looker(&self) -> Looker {
            match self { $($($p)*::$x(s) => s.looker()),* }
        }

        /// Gets the looker as a player expectantly. Should only be called by clients.
        pub fn player_expect(&self) -> u32 {
            match self { $($($p)*::$x(s) => if let Looker::Player(player) = s.looker() {player} else {panic!("Should be a player'")}),* }
        }

        /// Whether this looker is a player instead of a spectator
        pub fn is_player(&self) -> bool {
            match self { $($($p)*::$x(s) => s.looker().tag() == LookerTag::Player),* }
        }

        pub fn num_players(&self) -> u32 {
            match self { $($($p)*::$x(s) => s.player_states.len() as u32),* }
        }

        pub fn board_state(&self) -> BaseBoardState {
            match self { $($($p)*::$x(s) => s.board_state().clone().wrap_base()),* }
        }

        pub fn player_state(&self, player: u32) -> Option<BasePlayerState> {
            match self { $($($p)*::$x(s) => s.player_state(player).map(|state| state.clone().wrap_base())),* }
        }

        /// Whether the game is over
        pub fn game_over(&self) -> bool {
            match self { $($($p)*::$x(s) => s.game_over()),* }
        }

        /// Whether some player won the game
        pub fn won(&self, player: u32) -> bool {
            match self { $($($p)*::$x(s) => s.winners().contains(&player)),* }
        }

        /// Number of tiles left of each kind in the draw pile
        pub fn num_tiles_left_by_kind(&self) -> Vec<(BaseKind, u32)> {
            match self { $($($p)*::$x(s) => 
                s.num_tiles_left_by_kind().into_iter()
                    .map(|(kind, num)| (kind.wrap_base(), num))
                    .collect()
            ),* }
        }

        /// The tile at the top of the draw pile of some kind.
        /// None if there're no tiles of that kind left.
        pub fn top_tile_left_of_kind(&self, kind: &BaseKind) -> Option<BaseTile> {
            match self { $($($p)*::$x(s) => 
                s.top_tile_left_of_kind(<<$t as GameStateT>::Game as Game>::Kind::unwrap_base_ref(kind))
                    .map(|tile| tile.clone().wrap_base())
            ),* }
        }

        /// Whose turn it is
        pub fn turn_player(&self) -> u32 {
            match self { $($($p)*::$x(s) => s.turn_player()),* }
        }

        /// Whether all players placed their tokens
        pub fn all_players_placed(&self) -> bool {
            match self { $($($p)*::$x(s) => s.all_players_placed()),* }
        }

        pub fn place_player(&mut self, player: u32, port: &BasePort) {
            match self { $($($p)*::$x(s) => s.place_player(player, Port::unwrap_base_ref(port))),* }
        }

        /// Have the current player take a turn by placing a tile of kind `kind` from index `index` in their hand
        /// transformed by group action `action` to location `loc`.
        /// The turn is processed and then advances to the next player.
        pub fn take_turn_placing_tile(&mut self, game: &BaseGame, kind: &BaseKind, index: u32, action: &BaseGAct, loc: &BaseTLoc) -> BaseTurnResult {
            match self { $($($p)*::$x(s) => {
                let res = s.take_turn_placing_tile(
                    <$t as GameStateT>::Game::unwrap_base_ref(game),
                    Kind::unwrap_base_ref(kind),
                    index,
                    GAct::unwrap_base_ref(action),
                    TLoc::unwrap_base_ref(loc),
                );
                BaseTurnResult {
                    tile_placer: res.tile_placer,
                    tile_placed: (res.tile_placed.0, res.tile_placed.1.wrap_base()),
                    tile_loc: res.tile_loc.wrap_base(),
                    player_ports: res.player_ports.into_iter().map(|p| p.wrap_base()).collect(),
                    dead_players: res.dead_players,
                    num_tiles_left: res.num_tiles_left.into_iter().map(|(k, n)| (k.wrap_base(), n)).collect(),
                    drawn_tiles: res.drawn_tiles.into_iter().map(|(p, i, t)| (p, i, t.wrap_base())).collect(),
                    game_over: res.game_over,
                }
            }),* }
        }
    }

    $($crate::impl_wrap_base!(BaseGameState::$x($t)))*;
}

/// This trait is just to make the macro work
pub trait GameStateT {
    type Game: Game;
}

impl<G: Game> GameStateT for GameState<G> {
    type Game = G;
}

/// The state of the game
#[derive(Clone, Debug, Getters, CopyGetters, Serialize, Deserialize)]
pub struct GameState<G: Game> {
    #[getset(get = "pub")]
    board_state: BoardState<G::Board, G::Tile>,
    player_states: Vec<Option<PlayerState<G::Tile>>>,
    #[getset(get_copy = "pub")]
    looker: Looker,
    turn_player: u32,
    tiles: FnvHashMap<G::Kind, VecDeque<G::Tile>>,
    #[getset(get = "pub")]
    winners: Vec<u32>,
}

impl<G: Game> GameState<G> {
    /// Construct a new state from a game
    pub fn new(game: &G, num_players: u32) -> Self {
        let mut tiles = game.all_tiles();
        // TODO: Shuffle tiles first
        tiles.sort_by_key(|tile| tile.kind().clone());
        let groups = tiles.into_iter().group_by(|tile| tile.kind().clone());
        let mut tiles = groups.into_iter().map(|(kind, tiles)|
            (kind, tiles.map(|t| t.with_visible(false)).collect::<VecDeque<_>>())).collect::<FnvHashMap<_, _>>();
        for tiles in tiles.values_mut() {
            tiles.make_contiguous().shuffle(&mut pcg64!("Generating tiles for game"));
        }

        let mut state = Self {
            board_state: BoardState::new(game, num_players),
            player_states: vec![Some(PlayerState::new(game)); num_players as usize],
            looker: Looker::Server,
            turn_player: 0,
            tiles,
            winners: vec![],
        };

        // deal tiles
        for kind in game.board().all_kinds() {
            let num_tiles = game.num_tiles_per_player(&kind);
            (0..num_players).cycle().take((num_tiles * num_players) as usize).map(|player| {
                state.deal_tile(player, &kind)
            }).all(|b| b.is_some());
        }

        state
    }

    /// The state of a specific player. None if the player is dead.
    pub fn player_state(&self, player: u32) -> Option<&PlayerState<G::Tile>> {
        self.player_states[player as usize].as_ref()
    }

    /// The state of the game visible to `looker`.
    /// `looker` is None for spectators.
    pub fn visible_state(&self, looker: Looker) -> GameState<G> {
        GameState {
            board_state: self.board_state().clone(),
            player_states: self.player_states.iter().enumerate().map(|(player, maybe_state)|
                maybe_state.as_ref().map(|state| state.visible_state(player as u32, looker)))
                .collect_vec(),
            looker,
            turn_player: self.turn_player,
            tiles: self.tiles.iter().map(|(kind, tiles)|
                (kind.clone(), tiles.iter().map(|t| t.clone().with_visible(false)).collect()))
                .collect(),
            winners: self.winners.clone(),
        }
    }

    /// Number of players in the game
    pub fn num_players(&self) -> u32 {
        self.player_states.len() as u32
    }

    /// Who's turn it is
    pub fn turn_player(&self) -> u32 {
        self.turn_player
    }

    /// Gets the next tile by kind and updates the state. None if there's no tiles left of that kind
    pub fn next_tile(&mut self, kind: &G::Kind) -> Option<G::Tile> {
        self.tiles.get_mut(kind).expect("Each kind should have a list of tiles").pop_front()
    }

    /// Deals a tile of a specific kind to a specific player. Returns the tile dealt and index into the player's hand if one was dealt.
    pub fn deal_tile(&mut self, player: u32, kind: &G::Kind) -> Option<(u32, G::Tile)> {
        self.next_tile(kind).zip(self.player_states[player as usize].as_mut())
            .map(|(mut tile, state)| {
                tile.set_visible(self.looker.tag() != LookerTag::Player || self.looker == Looker::Player(player));
                state.add_tile(tile.clone());
                (state.num_tiles_by_kind(kind) as u32 - 1, tile)
            })
    }

    /// Place a player on some port.
    pub fn place_player(&mut self, player: u32, port: &G::Port) {
        self.board_state.place_player(player, port)
    }

    /// Place a tile on some location on the board. Assumes the location is empty and kinds match.
    pub fn place_tile(&mut self, tile: G::Tile, loc: &G::TLoc) {
        self.board_state.place_tile(tile, loc)
    }

    /// Have a player place a tile with some kind from some position in their hand, transformed by a group action, to a location on the board.
    /// For now, assumes the player is alive.
    /// Returns the tile placed.
    pub fn player_place_tile(&mut self, player: u32, kind: &G::Kind, index: u32, action: &G::GAct, loc: &G::TLoc) -> G::Tile {
        let tile = self.player_states[player as usize].as_mut().unwrap().remove_tile(kind, index)
            .with_visible(true)
            .apply_action(action);
        self.place_tile(tile.clone(), loc);
        tile
    }

    /// Whether all players placed their tokens
    pub fn all_players_placed(&self) -> bool {
        self.board_state().all_players_placed()
    }

    /// Number of tiles left of each kind in the draw pile
    pub fn num_tiles_left_by_kind(&self) -> Vec<(&G::Kind, u32)> {
        self.tiles.iter()
            .map(|(kind, tiles)| (kind, tiles.len() as u32))
            .collect()
    }

    /// The tile at the top of the draw pile of some kind.
    /// None if there're no tiles of that kind left.
    pub fn top_tile_left_of_kind(&self, kind: &G::Kind) -> Option<&G::Tile> {
        self.tiles.get(kind)
            .and_then(|tiles| tiles.iter().next())
    }

    /// Whether the game is over
    pub fn game_over(&self) -> bool {
        !self.winners.is_empty()
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
    /// 
    /// Returns a list of tiles added to player's hands in the form (player, index, tile)
    fn redistribute_tiles(&mut self, game: &G) -> Vec<(u32, u32, G::Tile)> {
        let mut new_tiles = vec![];

        for kind in game.board().all_kinds() {
            let num_tiles = game.num_tiles_per_player(&kind);
            let turn_player = self.turn_player();
            let num_players = self.num_players();
            let deal_tile_order = (0..num_tiles)
                .flat_map(|i| (0..num_players).map(move |j| ((j + turn_player + 1) % num_players, i)))
                .flat_map(|(player, i)| self.player_state(player)
                    .filter(|state| state.num_tiles_by_kind(&kind) <= i)
                    .map(|_| player))
                .collect_vec();

            for player in deal_tile_order {
                if let Some((index, tile)) = self.deal_tile(player, &kind) {
                    new_tiles.push((player, index, tile));
                } else {
                    break;
                }
            }
        }

        new_tiles
    }

    /// Removes tiles from dead players.
    /// Assumes the players were just alive
    pub fn handle_dead_players(&mut self, _game: &G, players: &[u32]) {
        let tiles = players.iter().flat_map(|player| {
            let tiles = self.player_states[*player as usize].as_mut().unwrap().remove_all_tiles();
            self.player_states[*player as usize] = None;
            tiles
        }).collect_vec();

        for mut tile in tiles {
            tile.set_visible(false);
            self.tiles.get_mut(tile.kind()).unwrap().push_back(tile);
        }
    }

    /// Can someone place their token on the board on port `port`?
    pub fn can_place_player(&mut self, game: &G, port: &G::Port) -> bool {
        self.board_state.player_at(port).is_none() && game.start_ports().contains(port)
    }

    /// Have the current player take a turn by placing their token on the board on port `port`.
    /// The turn is processed and then advances to the next player.
    pub fn take_turn_placing_player(&mut self, _game: &G, port: &G::Port) {
        self.place_player(self.turn_player(), port);
        // All players should still be alive
        self.turn_player = (self.turn_player + 1) % self.num_players();
    }

    /// Can `player` place a tile of kind `kind` from index `index` in their hand transformed by group action `action` to location `loc`?
    pub fn can_place_tile(&mut self, game: &G, player: u32, kind: &G::Kind, index: u32, _action: &G::GAct, loc: &G::TLoc) -> bool {
        self.player_states[player as usize].as_ref().map_or(false, |state| index < state.num_tiles_by_kind(kind)) &&
            self.board_state.player_port(player).map_or(false, |port|
                game.board().port_locs(port).contains(loc)) &&
            self.board_state.tile_at(loc).is_none() &&
            kind == &game.board().kind_at(loc)
            // TODO: In the original game, there's also the condition that a player can't kill themselves with a tile
            // if they have a move that doesn't do that. Figure out if this should be checked here.
    }

    /// Have the current player take a turn by placing a tile of kind `kind` from index `index` in their hand
    /// transformed by group action `action` to location `loc`.
    /// The turn is processed and then advances to the next player.
    pub fn take_turn_placing_tile(&mut self, game: &G, kind: &G::Kind, index: u32, action: &G::GAct, loc: &G::TLoc) -> TurnResult<G> {
        let tile_placer = self.turn_player;

        let tile_placed = self.player_place_tile(self.turn_player(), kind, index, action, loc);
        let dead = self.advance_players(game.board(), loc);
        let players_died = !dead.is_empty();
        self.handle_dead_players(game, &dead);
        let drawn_tiles = if players_died {
            self.redistribute_tiles(game)
        } else {
            self.deal_tile(self.turn_player, kind).map(|(index, tile)| (self.turn_player, index, tile)).into_iter().collect()
        };

        let mut all_dead = false;
        if let Some(next) = (0..self.num_players()).cycle().skip(self.turn_player() as usize + 1).take(self.num_players() as usize)
            .find(|player| self.player_state(*player).is_some())
        {
            self.turn_player = next;
        } else {
            // Every player died, so the last ones that remained won
            all_dead = true;
            self.winners = dead.clone();
        }

        let player_ports = (0..self.num_players())
            .map(|player| self.board_state().player_port(player).expect("Players should have placed ports").clone())
            .collect();
        let num_tiles_left = self.tiles.iter()
            .map(|(kind, tiles)| (kind.clone(), tiles.len() as u32))
            .collect();

        if !all_dead {
            let mut remaining = (0..self.num_players())
                .filter(|player| self.player_state(*player).is_some());
            if let (Some(winner), None) = (remaining.next(), remaining.next()) {
                // Unique player remaning, game is over
                self.winners = vec![winner];
            } else if self.player_states.iter()
                .flat_map(|maybe| maybe.as_ref())
                .all(|state| !state.has_tiles())
            {
                // If everyone's out of tiles, the game's over
                self.winners = (0..self.num_players())
                    .filter(|player| self.player_state(*player).is_some())
                    .collect();
            }
        }

        TurnResult {
            tile_placer,
            tile_placed: (index, tile_placed),
            tile_loc: loc.clone(),
            player_ports,
            dead_players: dead,
            num_tiles_left,
            drawn_tiles,
            game_over: !self.winners.is_empty()
        }
    }
}

/// The stuff that happened during a turn
#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct TurnResult<G: Game> {
    /// The player who placed the tile
    #[getset(get_copy = "pub")]
    tile_placer: u32,
    /// index and tile placed
    #[getset(get = "pub")]
    tile_placed: (u32, G::Tile),
    /// Where the tile was placed
    #[getset(get = "pub")]
    tile_loc: G::TLoc,
    /// New locations of players, indexed by player
    #[getset(get = "pub")]
    player_ports: Vec<G::Port>,
    /// Which players died
    #[getset(get = "pub")]
    dead_players: Vec<u32>,
    /// New number of tiles per kind in the draw pile
    #[getset(get = "pub")]
    num_tiles_left: Vec<(G::Kind, u32)>,
    /// New tiles drawn by players in (player, index, tile) format
    #[getset(get = "pub")]
    drawn_tiles: Vec<(u32, u32, G::Tile)>,
    /// Whether the game is over
    #[getset(get = "pub")]
    game_over: bool,
}

/// The stuff that happened during a turn
#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct BaseTurnResult {
    /// The player who placed the tile
    #[getset(get_copy = "pub")]
    tile_placer: u32,
    /// index and tile placed
    #[getset(get = "pub")]
    tile_placed: (u32, BaseTile),
    /// Where the tile was placed
    #[getset(get = "pub")]
    tile_loc: BaseTLoc,
    /// New locations of players, indexed by player
    #[getset(get = "pub")]
    player_ports: Vec<BasePort>,
    /// Which players died
    #[getset(get = "pub")]
    dead_players: Vec<u32>,
    /// New number of tiles per kind in the draw pile
    #[getset(get = "pub")]
    num_tiles_left: Vec<(BaseKind, u32)>,
    /// New tiles drawn by players in (player, index, tile) format
    #[getset(get = "pub")]
    drawn_tiles: Vec<(u32, u32, BaseTile)>,
    /// Whether the game is over
    #[getset(get_copy = "pub")]
    game_over: bool,
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