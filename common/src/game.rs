use std::marker::PhantomData;
use std::fmt::Debug;
use std::hash::Hash;

use crate::{board::Board, tile::Tile};

pub trait Game {
    type TLoc: Clone + Debug + Eq + Hash;
    type Port: Clone + Debug + Eq + Hash;
    type Kind: Clone + Debug + Eq + Hash;
    type TileConfig: Clone + Debug;
    type Board: Clone + Debug + Board<TLoc = Self::TLoc, Port = Self::Port, Kind = Self::Kind, TileConfig = Self::TileConfig>;
    type Tile: Clone + Debug + Tile<Kind = Self::Kind, TileConfig = Self::TileConfig>;

    /// The game's board
    fn board(&self) -> &Self::Board;

    /// The set of tiles the game uses
    fn all_tiles(&self) -> Vec<Self::Tile> {
        Self::Tile::all(self.board().tile_config())
    }
}

/// A definition for a path game
#[derive(Clone, Debug)]
pub struct PathGame<B: Board, T> {
    board: B,
    start_ports: Vec<<B as Board>::Port>,
    tiles_per_player: u32,
    phantom: PhantomData<T>,
}

impl<K, C, B, T> PathGame<B, T>
where
    K: Clone + Debug + Eq + Hash,
    C: Clone + Debug,
    B: Clone + Debug + Board<Kind = K, TileConfig = C>,
    T: Clone + Debug + Tile<Kind = K, TileConfig = C>
{
    pub fn new(board: B, start_ports: Vec<<B as Board>::Port>, tiles_per_player: u32) -> Self {
        Self {
            board,
            start_ports,
            tiles_per_player,
            phantom: PhantomData,
        }
    }
}

impl<K, C, B, T> Game for PathGame<B, T>
where
    K: Clone + Debug + Eq + Hash,
    C: Clone + Debug,
    B: Clone + Debug + Board<Kind = K, TileConfig = C>,
    T: Clone + Debug + Tile<Kind = K, TileConfig = C>
{
    type TLoc = B::TLoc;
    type Port = B::Port;
    type Kind = B::Kind;
    type TileConfig = B::TileConfig;
    type Board = B;
    type Tile = T;

    fn board(&self) -> &Self::Board {
        &self.board
    }
}