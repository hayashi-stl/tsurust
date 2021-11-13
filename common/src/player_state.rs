use fnv::FnvHashMap;

use crate::tile::Tile;

/// The state of a player
pub struct PlayerState<T: Tile> {
    tiles: FnvHashMap<T::Kind, Vec<T>>
}

impl<T: Tile> PlayerState<T> {
    /// The tiles the player is holding, grouped by kind
    pub fn tiles(&self) -> &FnvHashMap<T::Kind, Vec<T>> {
        &self.tiles
    }

    /// Adds a tile to the player's hand
    pub fn add_tile(&mut self, tile: T) {
        self.tiles.get_mut(&tile.kind()).expect("Every kind should have a tile list").push(tile)
    }
}