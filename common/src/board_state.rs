use fnv::FnvHashMap;
use itertools::Itertools;
use log::debug;
use std::hash::Hash;
use std::fmt::Debug;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::board::{BasePort, Board, RectangleBoard, Port, BaseTLoc, TLoc};
use crate::game::Game;
use crate::tile::{RegularTile, Tile, BaseTile};
use crate::WrapBase;

#[macro_export]
macro_rules! for_each_board_state {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::board_state::BaseBoardState)::Normal: $crate::board_state::BoardState<
                $crate::board::RectangleBoard, $crate::tile::RegularTile<4>
            >,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_board_state! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_board_state! {
    p::x, t =>
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum BaseBoardState {
        $($x($t)),*
    }

    impl BaseBoardState {
        pub fn player_port(&self, player: u32) -> Option<BasePort> {
            match self { $($($p)*::$x(s) => s.player_port(player).map(|port| port.clone().wrap_base())),* }
        }

        /// All the tiles on the board
        pub fn tiles_vec(&self) -> Vec<(BaseTLoc, BaseTile)> {
            match self { $($($p)*::$x(s) => 
                s.tiles_vec().into_iter()
                    .map(|(k, v)| (k.clone().wrap_base(), v.clone().wrap_base()))
                    .collect()
            ),* }
        }
    }

    $($crate::impl_wrap_base!(BaseBoardState::$x($t)))*;
}

/// The state of the board
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardState<B: Board, T> {
    tiles: FnvHashMap<B::TLoc, T>,
    players: Vec<Option<B::Port>>,
}

impl<K, C, B, T> BoardState<B, T>
where
    K: Clone + Debug + Eq + Hash,
    C: Clone + Debug,
    B: Clone + Debug + Board<Kind = K, TileConfig = C>,
    T: Clone + Debug + Tile<Kind = K, TileConfig = C>
{
    /// Construct a board state from a game
    pub fn new<G>(_game: &G, num_players: u32) -> Self where G: Game<Board = B, Tile = T> {
        Self {
            tiles: FnvHashMap::default(),
            players: vec![None; num_players as usize],
        }
    }

    /// All the tiles on the board
    pub fn tiles_vec(&self) -> Vec<(&B::TLoc, &T)> {
        self.tiles.iter().collect()
    }

    /// Tile on tile location. None if there's no tile there
    pub fn tile_at(&self, loc: &B::TLoc) -> Option<&T> {
        self.tiles.get(loc)
    }

    /// Whether all players placed their tokens
    pub fn all_players_placed(&self) -> bool {
        self.players.iter().all(|port| port.is_some())
    }
    
    /// Port that a player is on. None if the player hasn't placed their token yet
    pub fn player_port(&self, player: u32) -> Option<&B::Port> {
        self.players[player as usize].as_ref()
    }

    /// Player on port. None if there's no player there
    pub fn player_at(&self, port: &B::Port) -> Option<u32> {
        self.players.iter().position(|p| p.as_ref() == Some(port)).map(|n| n as u32)
    }

    /// Place a player token on some port.
    pub fn place_player(&mut self, player: u32, port: &B::Port) {
        self.players[player as usize] = Some(port.clone());
    }

    /// Place a tile on some location. Assumes the location is empty and kinds match.
    pub fn place_tile(&mut self, tile: T, loc: &B::TLoc) {
        self.tiles.insert(loc.clone(), tile);
    }

    /// Move players that touch a tile along their respective paths until they face a dead end.
    /// Assumes the location has a tile on it.
    /// Returns a list of newly dead players.
    pub fn advance_players(&mut self, board: &B, loc: &B::TLoc) -> Vec<u32> {
        // Contains tuples of player and tile location to move through.
        // If the tile location is None, the player is done moving.
        let mut to_advance = (0..self.players.len())
            .flat_map(|i| self.player_port(i as u32)
                .and_then(|p| board.loc_ports(loc).into_iter().position(|q| p == &q))
                .map(|_| (i as u32, Some(loc.clone()))))
            .collect_vec();

        let mut dead = vec![];

        while !to_advance.iter_mut().map(|(player, maybe_loc)| {
                if let Some(loc) = maybe_loc {
                    // Move player
                    let port_in = self.player_port(*player).unwrap();
                    let input = board.loc_ports(loc).into_iter().position(|p| &p == port_in).unwrap() as u32;
                    let tile = self.tile_at(loc).unwrap().clone();
                    let output = tile.output(input);
                    let port_out = board.loc_ports(loc)[output as usize].clone();
                    self.players[*player as usize] = Some(port_out.clone());

                    // Figure out if they can move again
                    // TODO: What if there's a choice?
                    *maybe_loc = board.port_locs(&port_out).into_iter().find(|l| l != loc);
                    if maybe_loc.is_none() {
                        dead.push(*player);
                    }
                    *maybe_loc = maybe_loc.clone().filter(|l| self.tile_at(l).is_some());
                    maybe_loc.is_none()
                } else {
                    true
                }
                // Don't use `all` to avoid short-circuiting
            }).fold(true, |b1, b2| b1 && b2)
        {}

        dead
    }
}