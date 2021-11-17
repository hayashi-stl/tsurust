use fnv::FnvHashMap;
use itertools::Itertools;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{board::Board, game::Game, tile::{RegularTile, Tile}};

#[enum_dispatch]
pub trait GenericPlayerState {}

impl<T: Tile> GenericPlayerState for PlayerState<T> {}

#[enum_dispatch(GenericPlayerState)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BasePlayerState {
    RegularTile4(PlayerState<RegularTile<4>>)
}

#[enum_dispatch]
pub trait GenericPublicPlayerState {}

impl<T: Tile> GenericPublicPlayerState for PublicPlayerState<T> {}

#[enum_dispatch(GenericPublicPlayerState)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BasePublicPlayerState {
    RegularTile4(PublicPlayerState<RegularTile<4>>)
}

#[enum_dispatch]
pub trait GenericPlayerStateE {}

impl<T: Tile> GenericPlayerStateE for PlayerStateE<T> {}

#[enum_dispatch(GenericPlayerStateE)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BasePlayerStateE {
    RegularTile4(PlayerStateE<RegularTile<4>>)
}

/// The state of a player
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState<T: Tile> {
    #[serde(bound = "")]
    tiles: FnvHashMap<T::Kind, Vec<T>>
}

/// The state of a player visible to other players
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicPlayerState<T: Tile> { 
    num_tiles: FnvHashMap<T::Kind, u32>
}

/// Player state with the tiles either visible or hidden
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlayerStateE<T: Tile> {
    #[serde(bound = "")]
    Private(PlayerState<T>),
    #[serde(bound = "")]
    Public(PublicPlayerState<T>),
}

impl<T: Tile> PlayerState<T> {
    /// Construct a player state with the player holding 0 tiles
    pub fn new<G>(game: &G) -> Self where G: Game<Tile = T, Kind = T::Kind> {
        Self { tiles: game.board().all_kinds().into_iter().map(|kind| (kind, vec![])).collect() }
    }

    /// The tiles the player is holding, grouped by kind
    pub fn tiles(&self) -> &FnvHashMap<T::Kind, Vec<T>> {
        &self.tiles
    }

    /// Number of tiles of a specific kind that the player is holding
    pub fn num_tiles_by_kind(&self, kind: &T::Kind) -> u32 {
        self.tiles[kind].len() as u32
    }

    /// Adds a tile to the player's hand
    pub fn add_tile(&mut self, tile: T) {
        self.tiles.get_mut(&tile.kind()).expect("Every kind should have a tile list").push(tile)
    }

    /// Removes and returns a tile from the player's hand by kind and index.
    /// For now, assumes the index exists.
    pub fn remove_tile(&mut self, kind: &T::Kind, index: u32) -> T {
        self.tiles.get_mut(kind).expect("Every kind should have a tile list")
            .remove(index as usize)
    }

    /// Removes and returns all tiles from the player's hand, probably because the player is dead.
    pub fn remove_all_tiles(&mut self) -> Vec<T> {
        self.tiles.values_mut().flat_map(|v| std::mem::take(v)).collect_vec()
    }
}