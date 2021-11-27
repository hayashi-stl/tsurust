use std::f64::consts::TAU;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::{cell::Cell, marker::PhantomData};
use std::fmt::Debug;
use std::hash::Hash;
use common::{for_each_tile, nalgebra, nalgebra as na};

use common::math::{Mtx2, Pt2, Vec2f, Vec3f, Vec3u, pt2};
use common::nalgebra::{ComplexField, vector};
use common::{board::{BaseBoard, BasePort, Board, RectangleBoard}, for_each_board, for_each_game, game::{BaseGame, Game, PathGame}, math::Vec2, tile::{RegularTile, Tile}};
use common::board::{BaseTLoc, Port, TLoc};
use common::tile::{BaseGAct, BaseKind, BaseTile, Kind};
use getset::{CopyGetters, Getters, MutGetters};
use itertools::{Itertools, chain, iproduct, izip};
use specs::prelude::*;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{DomParser, Element, MouseEvent, SupportedType, SvgElement, SvgGraphicsElement, SvgMatrix, SvgsvgElement};

use crate::ecs::{Collider, Model, TLocLabel, TileSlot, Transform, TileLabel, TileSelect, TileToPlace};
use crate::game::GameWorld;
use crate::{SVG_NS, add_event_listener, console_log, document};

//fn create_svg_element<S: JsCast>(name: &str) -> S {
//    web_sys::window().unwrap().document().unwrap().create_element_ns(Some("http://www.w3.org/2000/svg"), name)
//        .expect("SVG element could not be created")
//        .dyn_into()
//        .expect("Wrong type specified")
//}

fn parse_svg(svg_str: &str) -> SvgElement {
    let svg = DomParser::new().unwrap().parse_from_string(&svg_str, SupportedType::ImageSvgXml)
        .expect("SVG could not be created");
    svg.document_element().expect("SVG doesn't have an element")
        .dyn_into().expect("SVG is not an SVG")
}

pub trait SvgMatrixExt {
    /// Transforms a position with this matrix
    fn transform(&self, position: Pt2) -> Pt2;
}

impl SvgMatrixExt for SvgMatrix {
    fn transform(&self, position: Pt2) -> Pt2 {
        pt2(
            self.a() as f64 * position.x + self.c() as f64 * position.y + self.e() as f64,
            self.b() as f64 * position.x + self.d() as f64 * position.y + self.f() as f64,
        )
    }
}


/// Extension trait for Board, mainly for rendering since
/// the server should know nothing about rendering
pub trait BoardExt: Board {
    fn render(&self) -> SvgElement;

    fn port_position(&self, port: &Self::Port) -> Pt2;

    fn loc_position(&self, loc: &Self::TLoc) -> Pt2;

    /// Render the collider for a specific tile location.
    fn render_collider(&self, loc: &Self::TLoc) -> SvgElement;

    /// Creates an entity (mainly for collision detection) at a specific tile location.
    fn create_loc_collider_entity(&self, loc: &Self::TLoc, world: &mut World, id_counter: &mut u64) -> Entity;
}

impl BoardExt for RectangleBoard {
    fn render(&self) -> SvgElement {
        let svg_str = format!(r##"<g xmlns="{}" class="rectangular-board">"##, SVG_NS) +
            &chain!(
                iproduct!(0..self.height(), 0..self.width()).map(|(y, x)|
                    format!(r##"<rect x="{}" y="{}" width="1" height="1"/>"##, x, y)),
                self.boundary_ports().into_iter().map(|(min, d)| {
                    let v = self.port_position(&(min, d));
                    let dx = if d.x == 0 { 0.1 } else { 0.0 };
                    let dy = if d.y == 0 { 0.1 } else { 0.0 };
                    format!(r##"<line x1="{}" x2="{}" y1="{}" y2="{}" class="rectangular-board-notch"/>"##, v.x - dx, v.x + dx, v.y - dy, v.y + dy)
                })
            )
                .join("") +
            r##"</g>"##;

        parse_svg(&svg_str)
    }

    fn port_position(&self, port: &<Self as Board>::Port) -> Pt2 {
        port.0.cast::<f64>() + port.1.cast::<f64>() / (self.ports_per_edge() + 1) as f64
    }

    fn loc_position(&self, loc: &Self::TLoc) -> Pt2 {
        loc.cast() + vector![0.5, 0.5]
    }

    fn render_collider(&self, loc: &Self::TLoc) -> SvgElement {
        let svg_str = format!(concat!(
            r##"<g xmlns="{}" fill="transparent">"##,
            r##"<rect x="-0.5" y="-0.5" width="1" height="1"/>"##,
            r##"</g>"##
        ), SVG_NS);
        parse_svg(&svg_str)
    }

    fn create_loc_collider_entity(&self, loc: &Self::TLoc, world: &mut World, id_counter: &mut u64) -> Entity {
        let svg = self.render_collider(loc);
        world.create_entity()
            .with(Model::new(&svg, Collider::ORDER_TILE_LOC, &GameWorld::svg_root(), id_counter))
            .with(Collider::new(&svg))
            .with(Transform::new(self.loc_position(loc)))
            .with(TLocLabel(loc.clone().wrap_base()))
            .with(TileSlot)
            .build()
    }
}

/// Extension trait for BaseBoard, mainly for rendering since
/// the server should know nothing about rendering
pub trait BaseBoardExt {
    fn render(&self) -> SvgElement;
    
    fn port_position(&self, port: &BasePort) -> Pt2;

    fn loc_position(&self, loc: &BaseTLoc) -> Pt2;

    /// Creates an entity (mainly for collision detection) at a specific tile location.
    fn create_loc_collider_entity(&self, loc: &BaseTLoc, world: &mut World, id_counter: &mut u64) -> Entity;
}

for_each_board! {
    p::x, t => 

    impl BaseBoardExt for BaseBoard {
        fn render(&self) -> SvgElement {
            match self {
                $($($p)*::$x(b) => b.render()),*
            }
        }

        fn port_position(&self, port: &BasePort) -> Pt2 {
            match self {
                $($($p)*::$x(b) => b.port_position(<$t as Board>::Port::unwrap_base_ref(port))),*
            }
        }

        fn loc_position(&self, loc: &BaseTLoc) -> Pt2 {
            match self {
                $($($p)*::$x(b) => b.loc_position(<$t as Board>::TLoc::unwrap_base_ref(loc))),*
            }
        }

        fn create_loc_collider_entity(&self, loc: &BaseTLoc, world: &mut World, id_counter: &mut u64) -> Entity {
            match self {
                $($($p)*::$x(b) => b.create_loc_collider_entity(
                    <$t as Board>::TLoc::unwrap_base_ref(loc),
                    world,
                    id_counter
                )),*
            }
        }
    }
}

/// Gets the point vectors of a `n`-sided regular polygon with unit side length,
/// centered at the origin, and rotated so there are 2 points with minimum y coordinate.
fn regular_polygon_points(n: u32) -> Vec<Vec2> {
    let radius = 0.5 / (TAU / (2.0 * n as f64)).sin();
    (0..n).map(|i| {
        let angle = TAU * (-0.25 + (-0.5 + i as f64) / n as f64);
        let (sin, cos) = angle.sin_cos();
        vector![cos * radius, sin * radius]
    }).collect_vec()
}

/// Gets the SVG string that draws a `n`-sided regular polygon with unit side length,
/// centered at the origin, and rotated so there are 2 points with minimum y coordinate.
fn regular_polygon_svg_str(n: u32) -> String {
    let poly_str = regular_polygon_points(n).into_iter()
        .map(|vec| format!("{},{}", vec.x, vec.y))
        .join(" ");
    format!(r##"<polygon points="{}"/>"##, poly_str)
}

/// Extension trait for Tile, mainly for rendering since
/// the server should know nothing about rendering
pub trait TileExt: Tile {
    fn render(&self) -> SvgElement;
}

impl<const EDGES: u32> TileExt for RegularTile<EDGES> {
    fn render(&self) -> SvgElement {
        if self.visible() {
            let connections = (0..self.num_ports()).map(|i| self.output(i)).collect_vec();
            let mut covered = vec![false; connections.len()];
            let poly_pts = regular_polygon_points(EDGES);
            let pts_normals = poly_pts.into_iter()
                .circular_tuple_windows()
                .flat_map(|(p0, p1)| {
                    let normal = vector![-p1.y + p0.y, p1.x - p0.x];
                    let ports_per_edge = self.ports_per_edge();
                    (0..ports_per_edge).map(move |i|
                        (p0 + (p1 - p0) * (i + 1) as f64 / (ports_per_edge + 1) as f64, normal)
                    )
                })
                .collect_vec();

            let curviness = 0.25;
            let path_str = izip!(0..self.num_ports(), connections)
                .map(|(s, t)| {
                    let p0 = pts_normals[s as usize].0;
                    let p1 = pts_normals[s as usize].0 + pts_normals[s as usize].1 * curviness;
                    let p2 = pts_normals[t as usize].0 + pts_normals[t as usize].1 * curviness;
                    let p3 = pts_normals[t as usize].0;
                    format!(concat!(
                        r##"<path class="regular-tile-path-outer" d="M {0},{1} C {2},{3} {4},{5} {6},{7}"/>"##,
                        r##"<path class="regular-tile-path-inner" d="M {0},{1} C {2},{3} {4},{5} {6},{7}"/>"##,
                    ), p0.x, p0.y, p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)
                })
                .join("");

            let poly_str = regular_polygon_svg_str(EDGES);
            let svg_str = format!(concat!(
                r##"<g xmlns="{}" class="regular-tile-visible">"##,
                "{}{}",
                r##"</g>"##,
            ), SVG_NS, poly_str, path_str);
            parse_svg(&svg_str)
        } else {
            let poly_str = regular_polygon_svg_str(EDGES);
            let svg_str = format!(concat!(
                r##"<g xmlns="{}" class="regular-tile-hidden">"##,
                r##"{}"##,
                r##"</g>"##,
            ), SVG_NS, poly_str);
            parse_svg(&svg_str)
        }
    }
}

/// Extension trait for BaseTile, mainly for rendering since
/// the server should know nothing about rendering
pub trait BaseTileExt {
    fn render(&self) -> SvgElement;

    fn create_hand_entity(&self, index: u32, action: &BaseGAct, world: &mut World, id_counter: &mut u64) -> Entity;

    fn create_board_entity_common<'a>(&self, world: &'a mut World, id_counter: &mut u64) -> EntityBuilder<'a>;

    fn create_to_place_entity(&self, action: &BaseGAct, transform: Transform, world: &mut World, id_counter: &mut u64) -> Entity;

    fn create_on_board_entity(&self, board: &BaseBoard, loc: &BaseTLoc, world: &mut World, id_counter: &mut u64) -> Entity;
}

for_each_tile! {
    p::x, t => 

    impl BaseTileExt for BaseTile {
        fn render(&self) -> SvgElement {
            match self { $($($p)*::$x(b) => b.render()),* }
        }

        fn create_hand_entity(&self, index: u32, action: &BaseGAct, world: &mut World, id_counter: &mut u64) -> Entity {
            match self { $($($p)*::$x(b) => {
                let svg = self.apply_action(action).render();
                let wrapper = wrap_svg(&svg.dyn_into().unwrap(), 128);
                wrapper.set_attribute("class", "tile-unselected").expect("Cannot set tile select class");
                world.create_entity()
                    .with(TileLabel(self.clone()))
                    .with(Model::new(&wrapper, 0, &GameWorld::bottom_panel(), id_counter))
                    .with(Collider::new(&wrapper))
                    .with(TileSelect::new(self.kind(), index, action.clone()))
                    .build()
            }),* }
        }

        fn create_board_entity_common<'a>(&self, world: &'a mut World, id_counter: &mut u64) -> EntityBuilder<'a> {
            match self { $($($p)*::$x(b) => {
                world.create_entity()
                    .with(TileLabel(self.clone()))
            }),* }
        }

        fn create_to_place_entity(&self, action: &BaseGAct, transform: Transform, world: &mut World, id_counter: &mut u64) -> Entity {
            match self { $($($p)*::$x(b) => {
                let svg = self.apply_action(action).render();
                self.create_board_entity_common(world, id_counter)
                    .with(Model::new(&svg, Model::ORDER_TILE_HOVER, &GameWorld::svg_root(), id_counter))
                    .with(TileToPlace)
                    .with(transform)
                    .build()
            }),* }
        }

        fn create_on_board_entity(&self, board: &BaseBoard, loc: &BaseTLoc, world: &mut World, id_counter: &mut u64) -> Entity {
            match self { $($($p)*::$x(b) => {
                let svg = self.render();
                self.create_board_entity_common(world, id_counter)
                    .with(Model::new(&svg, Model::ORDER_TILE, &GameWorld::svg_root(), id_counter))
                    .with(Transform::new(board.loc_position(loc)))
                    .build()
            }),* }
        }
    }
}

/// Extension trait for Game, mainly for rendering since
/// the server should know nothing about rendering
pub trait GameExt: Game
where
    Self::Board: BoardExt
{
    /// Starting ports and their positions
    fn start_ports_and_positions(&self) -> Vec<(Self::Port, Pt2)> {
        self.start_ports().into_iter()
            .map(|port| (port.clone(), self.board().port_position(&port)))
            .collect()
    }
}

impl<K, C, B, T> GameExt for PathGame<B, T>
where
    K: Clone + Debug + Eq + Ord + Hash + Kind,
    C: Clone + Debug,
    B: Clone + Debug + Board<Kind = K, TileConfig = C> + BoardExt,
    T: Clone + Debug + Tile<Kind = K, TileConfig = C>
{}

/// Extension trait for BaseGame, mainly for rendering since
/// the server should know nothing about rendering
pub trait BaseGameExt {
    fn start_ports_and_positions(&self) -> Vec<(BasePort, Pt2)>;
}

for_each_game! {
    p::x, t => 

    impl BaseGameExt for BaseGame {
        fn start_ports_and_positions(&self) -> Vec<(BasePort, Pt2)> {
            match self {
                $($($p)*::$x(g) => g.start_ports_and_positions().into_iter()
                    .map(|(port, pos)| (port.wrap_base(), pos))
                    .collect()),*
            }
        }
    }
}

/// Renders a port collider, used for detecting whether the mouse is hovering over a port
pub fn render_port_collider() -> SvgElement {
    let svg_str = format!(concat!(
        r##"<g xmlns="{0}" fill="transparent">"##,
        r##"<circle r="0.167"/>"##,
        r##"</g>"##,
    ), SVG_NS);
    parse_svg(&svg_str)
}

fn hsv_to_rgb(mut h: f32, s: f32, v: f32) -> Vec3f {
    h *= 6.0;
    let vec = Vec3f::from([
        ((h - 3.0).abs() - 1.0).clamp(0.0, 1.0),
        (-(h - 2.0).abs() + 2.0).clamp(0.0, 1.0),
        (-(h - 4.0).abs() + 2.0).clamp(0.0, 1.0),
    ]);
    (Vec3f::from([1.0, 1.0, 1.0]) * (1.0 - s) + vec * s) * v
}

/// Renders a player token, given the player index and the number of players.
pub fn render_token(index: u32, num_players: u32, id_counter: &mut u64) -> SvgElement {
    let color = hsv_to_rgb(index as f32 / num_players as f32, 1.0, 1.0);
    let darker = color * 3.0 / 4.0;
    let color: Vec3u = na::try_convert(color * 255.0).expect("Color conversion failed");
    let darker: Vec3u = na::try_convert(darker * 255.0).expect("Color conversion failed");
    let svg_str = format!(concat!(
        r##"<g xmlns="{0}" transform="translate(0, 0)">"##,
        r##"<defs>"##,
        r##"<radialGradient id="g{7}">"##,
        r##"<stop offset="0%" stop-color="#{1:02x}{2:02x}{3:02x}"/>"##,
        r##"<stop offset="100%" stop-color="#{4:02x}{5:02x}{6:02x}"/>"##,
        r##"</radialGradient>"##,
        r##"</defs>"##,
        r##"<circle r="0.1" fill="url('#g{7}')"/>"##,
        r##"</g>"##
    ), SVG_NS, color.x, color.y, color.z, darker.x, darker.y, darker.z, {*id_counter += 1; *id_counter - 1});
    parse_svg(&svg_str)
}

/// Wraps the SVG in an `<svg>` element of a specific size.
/// The viewport is set so the svg fits snugly inside.
pub fn wrap_svg(svg: &SvgGraphicsElement, size: u32) -> SvgElement {
    let bbox = svg.get_b_box().expect("Cannot get bounding box");
    let wrapper_str = format!(concat!(
        r##"<svg xmlns="{0}" width="{1}" height="{1}" viewBox="{2} {3} {4} {5}">"##,
        r##"</svg>"##
    ), SVG_NS, size, -0.5, -0.5, 1, 1);//bbox.x(), bbox.y(), bbox.width(), bbox.height());
    let wrapper = parse_svg(&wrapper_str);
    wrapper.append_child(svg).expect("Cannot wrap svg");
    wrapper
}