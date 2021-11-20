use std::{collections::HashSet, fmt::Debug};
use std::hash::Hash;
use enum_dispatch::enum_dispatch;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::board::PortsPerEdgeTileConfig;
use crate::{wrap_functions, impl_wrap_functions};

#[enum_dispatch]
pub trait Kind: Serialize + for<'a> Deserialize<'a> {
    wrap_functions!(BaseKind);
}

impl Kind for () {
    impl_wrap_functions!(BaseKind, Unit);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BaseKind {
    Unit(())
}

pub trait GenericTile {}

impl<T: Tile> GenericTile for T {}

#[enum_dispatch(GenericTile)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BaseTile {
    RegularTile4(RegularTile<4>)
}

#[macro_export]
macro_rules! for_each_tile {
    (internal ($dollar:tt) $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar($dollar $name:path: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            $crate::tile::BaseTile::RegularTile4: $crate::tile::RegularTile::<4>,
        }
    };

    ($name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_tile! {
            internal ($) $name $ty => $($body)*
        }
    };
}

/// A tile in the path game, parameterized by kind
pub trait Tile: Clone + Eq + Ord + Hash + Serialize + for<'a> Deserialize<'a> {
    type Kind: Clone + Debug + Eq + Hash + Kind;
    type TileConfig: Clone + Debug;

    /// All tiles of this type, in no particular order.
    /// Rotations count as separate tiles.
    fn all_including_rotations(config: Self::TileConfig) -> Vec<Self> where Self: Sized;

    /// All tiles of this type, in no particular order.
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

        groups.into_iter().map(|group| group.into_iter().min_by_key(|tile| tile.clone()).unwrap()).collect_vec()
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

    /// Rotate the tile `num_times` times counterclockwise.
    fn rotate(&self, num_times: i32) -> Self;

    /// The output port of some input port on the tile
    fn output(&self, input: u32) -> u32;
}

/// A regular-polygon-shaped tile with `EDGES` edges
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RegularTile<const EDGES: u32> {
    connections: Vec<u32>
}

impl<const EDGES: u32> RegularTile<EDGES> {
    pub fn new(connections: Vec<u32>) -> Self {
        Self { connections }
    }

    fn ports_per_edge(&self) -> u32 {
        self.connections.len() as u32 / EDGES
    }
}

impl<const EDGES: u32> Tile for RegularTile<EDGES> {
    type Kind = ();
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