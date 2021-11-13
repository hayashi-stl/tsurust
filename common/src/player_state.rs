use fnv::FnvHashMap;

use crate::{board::Board, game::Game, tile::Tile};

/// The state of a player
#[derive(Clone, Debug)]
pub struct PlayerState<T: Tile> {
    tiles: FnvHashMap<T::Kind, Vec<T>>
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

    /// Adds a tile to the player's hand
    pub fn add_tile(&mut self, tile: T) {
        self.tiles.get_mut(&tile.kind()).expect("Every kind should have a tile list").push(tile)
    }

    /// Removes and returns a tile from the player's hand by kind and index.
    /// For now, assumes the index exists.
    pub fn remove_tile(&mut self, kind: T::Kind, index: u32) -> T {
        self.tiles.get_mut(&kind).expect("Every kind should have a tile list")
            .remove(index as usize)
    }
}