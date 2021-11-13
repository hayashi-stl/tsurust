use fnv::FnvHashMap;
use std::hash::Hash;
use std::fmt::Debug;

use crate::board::Board;
use crate::tile::Tile;

/// The state of the board
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
    /// Tile on tile location. None if there's no tile there
    pub fn tile_at(&self, loc: B::TLoc) -> Option<&T> {
        self.tiles.get(&loc)
    }
    
    /// Port that a player is on. None if the player is dead
    pub fn player_port(&self, player: u32) -> Option<B::Port> {
        self.players[player as usize].clone()
    }

    /// Player on port. None if there's no player there
    pub fn player_at(&self, port: B::Port) -> Option<u32> {
        self.players.iter().position(|p| p == &Some(port.clone())).map(|n| n as u32)
    }
}