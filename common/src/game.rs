use std::marker::PhantomData;
use std::fmt::Debug;
use std::hash::Hash;
use enum_dispatch::enum_dispatch;
use fnv::FnvHashMap;

use crate::{board::{Board, Port, RectangleBoard, TLoc}, game_state::GameState, tile::{Kind, RegularTile, Tile}};
use crate::game_state::BaseGameState;

#[enum_dispatch]
pub trait GenericGame {
    fn new_state(&self, num_players: u32) -> BaseGameState;
}

impl<G> GenericGame for G
where
    G: Game,
    BaseGameState: From<GameState<G>>,
{
    fn new_state(&self, num_players: u32) -> BaseGameState {
        GameState::new(self, num_players).into()
    }
}

#[enum_dispatch(GenericGame)]
pub enum BaseGame {
    Normal(PathGame<RectangleBoard, RegularTile<4>>)
}

pub trait Game {
    type TLoc: Clone + Debug + Eq + Hash + TLoc;
    type Port: Clone + Debug + Eq + Hash + Port;
    type Kind: Clone + Debug + Eq + Ord + Hash + Kind;
    type TileConfig: Clone + Debug;
    type Board: Clone + Debug + Board<TLoc = Self::TLoc, Port = Self::Port, Kind = Self::Kind, TileConfig = Self::TileConfig>;
    type Tile: Clone + Debug + Tile<Kind = Self::Kind, TileConfig = Self::TileConfig>;

    /// The game's board
    fn board(&self) -> &Self::Board;

    /// All the ports that players can start at
    fn start_ports(&self) -> Vec<Self::Port>;

    /// The set of tiles the game uses
    fn all_tiles(&self) -> Vec<Self::Tile> {
        Self::Tile::all(self.board().tile_config())
    }

    /// Tiles of some kind that a player starts with
    fn num_tiles_per_player(&self, kind: &Self::Kind) -> u32;
}

/// A definition for a path game
#[derive(Clone, Debug)]
pub struct PathGame<B: Board, T> {
    board: B,
    start_ports: Vec<<B as Board>::Port>,
    tiles_per_player: FnvHashMap<<B as Board>::Kind, u32>,
    phantom: PhantomData<T>,
}

impl<K, C, B, T> PathGame<B, T>
where
    K: Clone + Debug + Eq + Ord + Hash + Kind,
    C: Clone + Debug,
    B: Clone + Debug + Board<Kind = K, TileConfig = C>,
    T: Clone + Debug + Tile<Kind = K, TileConfig = C>
{
    pub fn new<I: IntoIterator<Item = (B::Kind, u32)>>(
        board: B, start_ports: Vec<<B as Board>::Port>, tiles_per_player: I) -> Self {
        Self {
            board,
            start_ports,
            tiles_per_player: tiles_per_player.into_iter().collect(),
            phantom: PhantomData,
        }
    }
}

impl<K, C, B, T> Game for PathGame<B, T>
where
    K: Clone + Debug + Eq + Ord + Hash + Kind,
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

    fn start_ports(&self) -> Vec<Self::Port> {
        self.start_ports.clone()
    }

    fn num_tiles_per_player(&self, kind: &Self::Kind) -> u32 {
        self.tiles_per_player[kind]
    }
}