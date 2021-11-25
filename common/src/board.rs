use crate::math::{Pt2i, Pt2u, Vec2i, Vec2u};
use crate::tile::Kind;
use na::point;
use nalgebra as na;
use nalgebra::vector;
use itertools::{Itertools, chain, iproduct};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use getset::{CopyGetters, Getters};
use crate::{wrap_functions, impl_wrap_functions};

use std::fmt::Debug;
use std::hash::Hash;

pub trait Port: Clone + Debug + Eq + Hash + Serialize + for<'a> Deserialize<'a> {
    wrap_functions!(BasePort);
}

impl Port for (Pt2u, Vec2u) {
    impl_wrap_functions!(() BasePort, Pt2uVec2u);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BasePort {
    Pt2uVec2u((Pt2u, Vec2u))
}

pub trait TLoc: Clone + Debug + Eq + Hash + Serialize + for<'a> Deserialize<'a> {
    wrap_functions!(BaseTLoc);
}

impl TLoc for Pt2u {
    impl_wrap_functions!(() BaseTLoc, Pt2u);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BaseTLoc {
    Pt2u(Pt2u)
}

#[macro_export]
macro_rules! for_each_board {
    (internal ($dollar:tt) $path:ident $name:ident $ty:ident => $($body:tt)*) => {
        macro_rules! __mac {
            ($dollar(($dollar ($dollar $path:tt)*) :: $dollar $name:ident: $dollar $ty:ty,)*) => {$($body)*}
        }
        __mac! {
            ($crate::board::BaseBoard)::RectangleBoard: $crate::board::RectangleBoard,
        }
    };

    ($path:ident::$name:ident, $ty:ident => $($body:tt)*) => {
        $crate::for_each_board! {
            internal ($) $path $name $ty => $($body)*
        }
    };
}

for_each_board! {
    p::x, t =>
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum BaseBoard {
        $($x($t)),*
    }

    impl BaseBoard {
        /// The player ports around a tile location, in order
        pub fn port_locs(&self, port: &BasePort) -> Vec<BaseTLoc> {
            match self { $($($p)*::$x(s) => s.port_locs(
                <$t as Board>::Port::unwrap_base_ref(port)
            ).into_iter().map(|loc| loc.wrap_base()).collect()),* }
        }
    }

    $(
        impl $t {
            $crate::impl_wrap_functions!((pub) BaseBoard, $x);
        }
    )*
}

/// A board in the path game, parameterized by player location (port) type, tile location type, and tile kind type
pub trait Board: Clone + Debug + Serialize + for<'a> Deserialize<'a> {
    type TLoc: TLoc;
    type Port: Port;
    type Kind: Kind;
    type TileConfig: Clone + Debug;

    /// All the ports on the board, in no particular order
    fn all_ports(&self) -> Vec<Self::Port>;

    /// The ports on the boundary of the board, in no particular order
    fn boundary_ports(&self) -> Vec<Self::Port>;

    /// All the kinds of tiles used by the board
    fn all_kinds(&self) -> Vec<Self::Kind>;

    /// The kind of tile that goes in a specific location
    fn kind_at(&self, loc: &Self::TLoc) -> Self::Kind;

    /// The player ports around a tile location, in order
    fn loc_ports(&self, loc: &Self::TLoc) -> Vec<Self::Port>;

    /// The tile locations around a port, in no particular order
    fn port_locs(&self, port: &Self::Port) -> Vec<Self::TLoc>;

    /// Tile configuration for the board, used for generating tiles
    fn tile_config(&self) -> Self::TileConfig;
}

/// A tile config that just stores the number of ports per edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortsPerEdgeTileConfig(pub u32);

/// A rectangular board with square tiles.
#[derive(Clone, Debug, Serialize, Deserialize, CopyGetters)]
pub struct RectangleBoard {
    #[getset(get_copy = "pub")]
    width: u32,
    #[getset(get_copy = "pub")]
    height: u32,
    #[getset(get_copy = "pub")]
    ports_per_edge: u32
}

impl RectangleBoard {
    pub fn new(width: u32, height: u32, ports_per_edge: u32) -> Self {
        Self { width, height, ports_per_edge }
    }
}

impl Board for RectangleBoard {
    /// Coordinates of a tile
    type TLoc = Pt2u;
    /// Floored coordinates of a port, followed by fractional coordinates times `ports_per_edge + 1`
    type Port = (Pt2u, Vec2u);
    type Kind = ();
    type TileConfig = PortsPerEdgeTileConfig;

    fn all_ports(&self) -> Vec<Self::Port> {
        chain!(
            iproduct!(0..=self.height, 0..self.width, 1..=self.ports_per_edge).map(|(y, x, i)| (point![x, y], vector![i, 0])),
            iproduct!(0..=self.width, 0..self.height, 1..=self.ports_per_edge).map(|(x, y, i)| (point![x, y], vector![0, i]))
        ).collect_vec()
    }

    fn boundary_ports(&self) -> Vec<Self::Port> {
        chain!(
            iproduct!([0, self.height], 0..self.width, 1..=self.ports_per_edge).map(|(y, x, i)| (point![x, y], vector![i, 0])),
            iproduct!([0, self.width], 0..self.height, 1..=self.ports_per_edge).map(|(x, y, i)| (point![x, y], vector![0, i]))
        ).collect_vec()
    }

    fn all_kinds(&self) -> Vec<Self::Kind> {
        vec![()]
    }

    fn kind_at(&self, _: &Self::TLoc) -> Self::Kind {
    }

    fn loc_ports(&self, loc: &<Self as Board>::TLoc) -> Vec<<Self as Board>::Port> {
        chain!(
            (1..=self.ports_per_edge).map(|i| (*loc, vector![i, 0])),
            (1..=self.ports_per_edge).map(|i| (*loc + vector![1, 0], vector![0, i])),
            (1..=self.ports_per_edge).rev().map(|i| (*loc + vector![0, 1], vector![i, 0])),
            (1..=self.ports_per_edge).rev().map(|i| (*loc, vector![0, i]))
        ).collect_vec()
    }

    fn port_locs(&self, port: &Self::Port) -> Vec<Self::TLoc> {
        let p0 = na::convert::<_, Pt2i>(port.0);
        let p1 = p0 + if port.1[1] == 0 { vector![0, -1] } else { vector![-1, 0] };

        IntoIterator::into_iter([p0, p1])
            .filter(|vec| vec[0] >= 0 && vec[0] < self.width as i32 && vec[1] >= 0 && vec[1] < self.height as i32)
            .flat_map(na::try_convert)
            .collect_vec()
    }

    fn tile_config(&self) -> Self::TileConfig {
        PortsPerEdgeTileConfig(self.ports_per_edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::iproduct;

    #[test]
    fn test_rectangle_board_tile_ports() {
        let board = RectangleBoard::new(3, 2, 2);

        let ports = iproduct!(0..2, 0..3).map(|(y, x)| board.loc_ports(&point![x, y])).collect_vec();
        let expected = iproduct!(0..2, 0..3).map(|(y, x)| vec![
            (point![x + 0, y + 0], vector![1, 0]),
            (point![x + 0, y + 0], vector![2, 0]),
            (point![x + 1, y + 0], vector![0, 1]),
            (point![x + 1, y + 0], vector![0, 2]),
            (point![x + 0, y + 1], vector![2, 0]),
            (point![x + 0, y + 1], vector![1, 0]),
            (point![x + 0, y + 0], vector![0, 2]),
            (point![x + 0, y + 0], vector![0, 1]),
        ]).collect_vec();

        assert_eq!(ports, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_horz_sep() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs(&(point![2, 0], vector![0, 1]));
        let mut expected = vec![point![1, 0], point![2, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_vert_sep() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs(&(point![1, 1], vector![2, 0]));
        let mut expected = vec![point![1, 0], point![1, 1]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_left() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs(&(point![0, 0], vector![0, 1]));
        let mut expected = vec![point![0, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_right() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs(&(point![3, 0], vector![0, 1]));
        let mut expected = vec![point![2, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec.coords));
        assert_eq!(locs, expected);
    }
}