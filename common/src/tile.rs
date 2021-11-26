use std::{collections::HashSet, fmt::Debug};
use std::hash::Hash;
use enum_dispatch::enum_dispatch;
use getset::CopyGetters;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::board::PortsPerEdgeTileConfig;
use crate::{wrap_functions, impl_wrap_functions};
use crate::WrapBase;

pub trait Kind: Clone + Debug + Eq + Ord + Hash + Serialize + for<'a> Deserialize<'a> {
    wrap_functions!(BaseKind);
}

impl Kind for () {
    impl_wrap_functions!(() BaseKind, Unit);
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BaseKind {
    Unit(())
}

/// A group action on a tile.
pub trait GAct: Clone + Debug + Eq + Ord + Hash + Serialize + for<'a> Deserialize<'a> {
    /// Compose this group action with another
    fn compose(&self, other: &Self) -> Self;

    wrap_functions!(BaseGAct);
}

/// Action on a cyclic group of size `size`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, CopyGetters)]
pub struct CycleGAct {
    #[getset(get_copy = "pub")]
    rotation: i32,
    #[getset(get_copy = "pub")]
    size: u32,
}

impl GAct for CycleGAct {
    fn compose(&self, other: &Self) -> Self {
        assert_eq!(self.size, other.size, "Cycle group sizes must equal");
        CycleGAct {
            rotation: (self.rotation + other.rotation).rem_euclid(self.size as i32),
            size: self.size
        }
    }

    impl_wrap_functions!(() BaseGAct, Cycle);
}

/// Action on a dihedral group on a cycle of size `size`
/// (possible reflection across an axis across element 0 of the cycle, followed by rotation)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, CopyGetters)]
pub struct DihedralGAct {
    #[getset(get_copy = "pub")]
    rotation: i32,
    #[getset(get_copy = "pub")]
    reflected: bool,
    #[getset(get_copy = "pub")]
    size: u32,
}

impl GAct for DihedralGAct {
    fn compose(&self, other: &Self) -> Self {
        assert_eq!(self.size, other.size, "Cycle group sizes must equal");
        DihedralGAct {
            rotation: (self.rotation * if other.reflected {-1} else {1} + other.rotation).rem_euclid(self.size as i32),
            reflected: self.reflected != other.reflected,
            size: self.size,
        }
    }

    impl_wrap_functions!(() BaseGAct, Dihedral);
}

#[macro_export]
macro_rules! for_each_gact {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::tile::BaseGAct)::Cycle: $crate::tile::CycleGAct,
            ($crate::tile::BaseGAct)::Dihedral: $crate::tile::DihedralGAct,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_gact! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_gact! {
    p::x, t =>

    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum BaseGAct {
        $($x($t)),*
    }

    impl BaseGAct {
        /// Compose this group action with another
        pub fn compose(&self, other: &Self) -> Self {
            match self { $($($p)*::$x(s) => s.compose(<$t>::unwrap_base_ref(other)).wrap_base()),* }
        }
    }
}

#[macro_export]
macro_rules! for_each_tile {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::tile::BaseTile)::RegularTile4: $crate::tile::RegularTile<4>,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_tile! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_tile! {
    p::x, t =>
    #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
    pub enum BaseTile {
        $($x($t)),*
    }

    impl BaseTile {
        /// The kind of the tile
        pub fn kind(&self) -> BaseKind {
            match self { $($($p)*::$x(s) => s.kind().clone().wrap_base()),* }
        }

        /// Rotate the tile `num_times` times clockwise.
        pub fn rotate(&self, num_times: i32) -> Self {
            match self { $($($p)*::$x(s) => s.rotate(num_times).wrap_base()),* }
        }

        /// Generate the identity group action.
        pub fn identity_action(&self) -> BaseGAct {
            match self { $($($p)*::$x(s) => s.identity_action().wrap_base()),* }
        }

        /// Generate a rotation group action that rotates `num_times` times clockwise.
        pub fn rotation_action(&self, num_times: i32) -> BaseGAct {
            match self { $($($p)*::$x(s) => s.rotation_action(num_times).wrap_base()),* }
        }

        /// Apply a group action to this tile.
        pub fn apply_action(&self, action: &BaseGAct) -> Self {
            match self { $($($p)*::$x(s) => s.apply_action(GAct::unwrap_base_ref(action)).wrap_base()),* }
        }
    }

    $($crate::impl_wrap_base!(BaseTile::$x($t)))*;
}

/// A tile in the path game, parameterized by kind
pub trait Tile: Clone + Debug + Eq + Ord + Hash + Serialize + for<'a> Deserialize<'a> {
    type Kind: Kind;
    type GAct: GAct;
    type TileConfig: Clone + Debug;

    /// All tiles of this type, in no particular order, but a deterministic order.
    /// Rotations count as separate tiles.
    fn all_including_rotations(config: Self::TileConfig) -> Vec<Self> where Self: Sized;

    /// All tiles of this type, in no particular order, but a deterministic order.
    /// Rotations do not count as separate tiles.
    fn all(config: Self::TileConfig) -> Vec<Self> where Self: Sized {
        let mut with_rotations = Self::all_including_rotations(config).into_iter().collect::<HashSet<_>>();

        let mut groups = vec![];
        while !with_rotations.is_empty() {
            let tile = with_rotations.iter().next().unwrap().clone();
            groups.push(tile.all_rotations().into_iter().map(|t| {
                with_rotations.remove(&t);
                t
            }).collect_vec());
        }

        groups.into_iter().map(|group| group.into_iter().min_by_key(|tile| tile.clone()).unwrap())
            .sorted()
            .collect_vec()
    }

    /// All rotations of this tile.
    fn all_rotations(&self) -> Vec<Self> where Self: Sized;

    /// The canonical orientation of this tile.
    fn canonical(&self) -> Self where Self: Sized {
        self.all_rotations().into_iter().min_by_key(|tile| tile.clone()).unwrap()
    }

    /// The kind of the tile
    fn kind(&self) -> &Self::Kind;

    /// The number of ports on this tile
    fn num_ports(&self) -> u32;

    /// Rotate the tile `num_times` times clockwise.
    fn rotate(&self, num_times: i32) -> Self;

    /// Generate the identity group action.
    fn identity_action(&self) -> Self::GAct;

    /// Generate a rotation group action that rotates `num_times` times clockwise.
    fn rotation_action(&self, num_times: i32) -> Self::GAct;

    /// Apply a group action to this tile.
    fn apply_action(&self, action: &Self::GAct) -> Self;

    /// The output port of some input port on the tile
    fn output(&self, input: u32) -> u32;

    /// Whether the tile is visible to whoever's has the reference
    fn visible(&self) -> bool;

    /// Set the visibility of this tile using the builder pattern
    fn with_visible(self, visible: bool) -> Self;

    /// Set the visibility of this tile
    fn set_visible(&mut self, visible: bool);
}

/// A regular-polygon-shaped tile with `EDGES` edges.
/// Parameterized on number of edges since boards can't support arbitary regular polygons.
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RegularTile<const EDGES: u32> {
    connections: Vec<u32>,
    visible: bool,
}

impl<const EDGES: u32> RegularTile<EDGES> {
    pub fn new(connections: Vec<u32>) -> Self {
        Self { connections, visible: true }
    }

    pub fn ports_per_edge(&self) -> u32 {
        self.connections.len() as u32 / EDGES
    }
}

impl<const EDGES: u32> Tile for RegularTile<EDGES> {
    type Kind = ();
    type GAct = CycleGAct;
    type TileConfig = PortsPerEdgeTileConfig;

    fn all_including_rotations(ports_per_edge: Self::TileConfig) -> Vec<Self> where Self: Sized {
        assert!(ports_per_edge.0 * EDGES % 2 == 0, "Tried to create {}-sided RegularTile with {} ports per edge, an odd number",
            EDGES, ports_per_edge.0);

        let num_ports = ports_per_edge.0 * EDGES;
        // Size of each iterator in the product. 
        let sizes = vec![1; num_ports as usize / 2].into_iter()
            .interleave((0..num_ports / 2).rev().map(|i| 2 * i + 1))
            .collect_vec();

        // pairing[i] is connected to pairing[i xor 1] when pairing is added to the pairing list
        let mut pairing = vec![0; num_ports as usize];
        let mut pairings = vec![];
        let mut numbers_left = vec![];
        for mut i in 0..sizes.iter().copied().product::<u32>() {
            numbers_left.extend(0..num_ports);

            for (j, size) in sizes.iter().enumerate() {
                pairing[j] = i % *size;
                i /= size;
            }
            pairing[sizes.len()..num_ports as usize].fill(0);

            for entry in &mut pairing {
                *entry = numbers_left.remove(*entry as usize);
            }

            pairings.push(pairing.clone());
        }

        pairings.into_iter().map(|pairing| {
            let mut connection = vec![0; pairing.len()];
            for (p0, p1) in pairing.iter().step_by(2).zip(pairing.iter().skip(1).step_by(2)) {
                connection[*p0 as usize] = *p1;
                connection[*p1 as usize] = *p0;
            }
            Self::new(connection)
        }).collect_vec()
    }

    fn all_rotations(&self) -> Vec<Self>
    where Self: Sized {
        (0..EDGES).map(|i| self.rotate(i as i32)).collect_vec()
    }

    fn kind(&self) -> &Self::Kind { &() }

    fn num_ports(&self) -> u32 {
        self.ports_per_edge() * EDGES
    }

    fn identity_action(&self) -> Self::GAct {
        Self::GAct {
            rotation: 0,
            size: EDGES
        }
    }

    fn rotation_action(&self, num_times: i32) -> Self::GAct {
        Self::GAct {
            rotation: num_times.rem_euclid(EDGES as i32),
            size: EDGES
        }
    }

    fn apply_action(&self, action: &Self::GAct) -> Self {
        self.rotate(action.rotation)
    }

    fn rotate(&self, num_times: i32) -> Self {
        let mut result = self.clone();
        let offset = (num_times * self.ports_per_edge() as i32).rem_euclid(self.num_ports() as i32);
        for i in 0..self.num_ports() as i32 {
            result.connections[i as usize] =
                (self.connections[(i - offset).rem_euclid(self.num_ports() as i32) as usize] as i32 + offset).rem_euclid(self.num_ports() as i32) as u32;
        }
        result
    }

    fn output(&self, input: u32) -> u32 {
        self.connections[input as usize]
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

#[cfg(test)]
mod tests {
    use crate::tile::Tile;
    use super::*;

    #[test]
    fn test_square_tile_rotate_ccw() {
        let tile = RegularTile::<4>::new(vec![2, 3, 0, 1, 7, 6, 5, 4]);
        let expected = RegularTile::<4>::new(vec![7, 6, 4, 5, 2, 3, 1, 0]);
        assert_eq!(tile.rotate(1), expected);
    }

    #[test]
    fn test_square_tile_rotate_cw() {
        let tile = RegularTile::<4>::new(vec![2, 3, 0, 1, 7, 6, 5, 4]);
        let expected = RegularTile::<4>::new(vec![6, 7, 5, 4, 3, 2, 0, 1]);
        assert_eq!(tile.rotate(-1), expected);
    }

    #[test]
    fn test_triangle_tile_all() {
        let all = RegularTile::<3>::all(PortsPerEdgeTileConfig(2));
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_square_tile_single_port_all() {
        let all = RegularTile::<4>::all(PortsPerEdgeTileConfig(1));
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_square_tile_all() {
        let all = RegularTile::<4>::all(PortsPerEdgeTileConfig(2));
        assert_eq!(all.len(), 35);
    }
}