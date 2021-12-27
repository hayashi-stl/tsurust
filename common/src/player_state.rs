use fnv::FnvHashMap;
use getset::Getters;
use itertools::Itertools;

use serde::{Deserialize, Serialize};
use strum_macros::EnumDiscriminants;

use crate::{board::Board, game::Game, tile::{Tile}};
use crate::tile::{BaseKind, BaseTile, Kind};
use crate::WrapBase;

#[macro_export]
macro_rules! for_each_player_state {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::player_state::BasePlayerState)::RegularTile4: $crate::player_state::PlayerState<$crate::tile::RegularTile<4>>,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_player_state! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_player_state! {
    p::x, t =>
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum BasePlayerState {
        $($x($t)),*
    }

    impl BasePlayerState {
        pub fn tiles_vec(&self) -> Vec<(BaseKind, Vec<BaseTile>)> {
            match self {
                $($($p)*::$x(s) => s.tiles_vec().into_iter()
                    .map(|(k, v)| (
                        k.clone().wrap_base(),
                        v.into_iter().map(|tile| tile.clone().wrap_base()).collect_vec()
                    ))
                    .collect_vec()),*
            }
        }
    }

    $($crate::impl_wrap_base!(BasePlayerState::$x($t)))*;
}

/// Someone that looks at the game
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumDiscriminants, Serialize, Deserialize)]
#[strum_discriminants(name(LookerTag))]
pub enum Looker {
    Server,
    Player(u32),
    Spectator,
}

impl Looker {
    pub fn tag(self) -> LookerTag {
        self.into()
    }
}

/// The state of a player
#[derive(Clone, Debug, Serialize, Deserialize, Getters)]
pub struct PlayerState<T: Tile> {
    #[serde(bound = "")]
    #[getset(get = "pub")]
    tiles: FnvHashMap<T::Kind, Vec<T>>
}

impl<T: Tile> PlayerState<T> {
    /// Construct a player state with the player holding 0 tiles
    pub fn new<G>(game: &G) -> Self where G: Game<Tile = T, Kind = T::Kind> {
        Self { tiles: game.board().all_kinds().into_iter().map(|kind| (kind, vec![])).collect() }
    }

    /// Whether the player has any tiles
    pub fn has_tiles(&self) -> bool {
        self.tiles.iter().any(|(_, v)| !v.is_empty())
    }

    /// The tiles the player is holding, grouped by kind, with the kinds sorted
    pub fn tiles_vec(&self) -> Vec<(&T::Kind, &[T])> {
        self.tiles.iter().map(|(k, v)| (k, v.as_slice()))
            .sorted_by_key(|(k, _)| *k)
            .collect_vec()
    }

    /// Number of tiles of a specific kind that the player is holding
    pub fn num_tiles_by_kind(&self, kind: &T::Kind) -> u32 {
        self.tiles[kind].len() as u32
    }

    /// Adds a tile to the player's hand
    pub fn add_tile(&mut self, tile: T) {
        self.tiles.get_mut(tile.kind()).expect("Every kind should have a tile list").push(tile)
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

    /// Returns the state of `player` visible to `looker`
    pub fn visible_state(&self, player: u32, looker: Looker) -> PlayerState<T> {
        let mut result = self.clone();
        for tile in result.tiles.values_mut().into_iter().flatten() {
            tile.set_visible(looker.tag() != LookerTag::Player || looker == Looker::Player(player));
        }
        result
    }
}