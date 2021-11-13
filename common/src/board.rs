use crate::math::{Vec2i, Vec2u};
use nalgebra as na;
use nalgebra::vector;
use itertools::{Itertools, chain, iproduct};

use std::fmt::Debug;
use std::hash::Hash;

/// A board in the path game, parameterized by player location (port) type, tile location type, and tile kind type
pub trait Board {
    type TLoc: Clone + Debug + Eq + Hash;
    type Port: Clone + Debug + Eq + Hash;
    type Kind: Clone + Debug + Eq + Hash;
    type TileConfig: Clone + Debug;

    /// All the ports on the board, in no particular order
    fn all_ports(&self) -> Vec<Self::Port>;

    /// The ports on the boundary of the board, in no particular order
    fn boundary_ports(&self) -> Vec<Self::Port>;

    /// All the kinds of tiles used by the board
    fn all_kinds(&self) -> Vec<Self::Kind>;

    /// The kind of tile that goes in a specific location
    fn kind_at(&self, loc: Self::TLoc) -> Self::Kind;

    /// The player ports around a tile location, in order
    fn loc_ports(&self, loc: Self::TLoc) -> Vec<Self::Port>;

    /// The tile locations around a port, in no particular order
    fn port_locs(&self, port: Self::Port) -> Vec<Self::TLoc>;

    /// Tile configuration for the board, used for generating tiles
    fn tile_config(&self) -> Self::TileConfig;
}

/// A tile config that just stores the number of ports per edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortsPerEdgeTileConfig(pub u32);

/// A rectangular board with square tiles.
#[derive(Clone, Debug)]
pub struct RectangleBoard {
    width: u32,
    height: u32,
    ports_per_edge: u32
}

impl RectangleBoard {
    pub fn new(width: u32, height: u32, ports_per_edge: u32) -> Self {
        Self { width, height, ports_per_edge }
    }
}

impl Board for RectangleBoard {
    /// Coordinates of a tile
    type TLoc = Vec2u;
    /// Floored coordinates of a port, followed by fractional coordinates times `ports_per_edge + 1`
    type Port = (Vec2u, Vec2u);
    type Kind = ();
    type TileConfig = PortsPerEdgeTileConfig;

    fn all_ports(&self) -> Vec<Self::Port> {
        chain!(
            iproduct!(0..=self.height, 0..self.width, 1..=self.ports_per_edge).map(|(y, x, i)| (vector![x, y], vector![i, 0])),
            iproduct!(0..=self.width, 0..self.height, 1..=self.ports_per_edge).map(|(x, y, i)| (vector![x, y], vector![0, i]))
        ).collect_vec()
    }

    fn boundary_ports(&self) -> Vec<Self::Port> {
        chain!(
            iproduct!([0, self.height], 0..self.width, 1..=self.ports_per_edge).map(|(y, x, i)| (vector![x, y], vector![i, 0])),
            iproduct!([0, self.width], 0..self.height, 1..=self.ports_per_edge).map(|(x, y, i)| (vector![x, y], vector![0, i]))
        ).collect_vec()
    }

    fn all_kinds(&self) -> Vec<Self::Kind> {
        vec![()]
    }

    fn kind_at(&self, _: Self::TLoc) -> Self::Kind {
    }

    fn loc_ports(&self, loc: <Self as Board>::TLoc) -> Vec<<Self as Board>::Port> {
        chain!(
            (1..=self.ports_per_edge).map(|i| (loc, vector![i, 0])),
            (1..=self.ports_per_edge).map(|i| (loc + vector![1, 0], vector![0, i])),
            (1..=self.ports_per_edge).rev().map(|i| (loc + vector![0, 1], vector![i, 0])),
            (1..=self.ports_per_edge).rev().map(|i| (loc, vector![0, i]))
        ).collect_vec()
    }

    fn port_locs(&self, port: Self::Port) -> Vec<Self::TLoc> {
        let p0 = na::convert::<_, Vec2i>(port.0);
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

        let ports = iproduct!(0..2, 0..3).map(|(y, x)| board.loc_ports(vector![x, y])).collect_vec();
        let expected = iproduct!(0..2, 0..3).map(|(y, x)| vec![
            (vector![x + 0, y + 0], vector![1, 0]),
            (vector![x + 0, y + 0], vector![2, 0]),
            (vector![x + 1, y + 0], vector![0, 1]),
            (vector![x + 1, y + 0], vector![0, 2]),
            (vector![x + 0, y + 1], vector![2, 0]),
            (vector![x + 0, y + 1], vector![1, 0]),
            (vector![x + 0, y + 0], vector![0, 2]),
            (vector![x + 0, y + 0], vector![0, 1]),
        ]).collect_vec();

        assert_eq!(ports, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_horz_sep() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs((vector![2, 0], vector![0, 1]));
        let mut expected = vec![vector![1, 0], vector![2, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_vert_sep() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs((vector![1, 1], vector![2, 0]));
        let mut expected = vec![vector![1, 0], vector![1, 1]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_left() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs((vector![0, 0], vector![0, 1]));
        let mut expected = vec![vector![0, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        assert_eq!(locs, expected);
    }

    #[test]
    fn test_rectangle_board_port_tiles_right() {
        let board = RectangleBoard::new(3, 2, 2);
        let mut locs = board.port_locs((vector![3, 0], vector![0, 1]));
        let mut expected = vec![vector![2, 0]];
        locs.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        expected.sort_by_key(|vec| *AsRef::<[u32; 2]>::as_ref(&vec));
        assert_eq!(locs, expected);
    }
}