use std::marker::PhantomData;
use std::fmt::Debug;
use std::hash::Hash;

use common::{board::{BaseBoard, BasePort, Board, RectangleBoard}, for_each_board, for_each_game, game::{BaseGame, Game, PathGame}, math::Vec2, tile::{RegularTile, Tile}};
use common::board::Port;
use common::tile::Kind;
use enum_dispatch::enum_dispatch;
use itertools::{Itertools, chain, iproduct};
use specs::{Component, DenseVecStorage, Join, ReadStorage, System, VecStorage, WriteStorage};
use wasm_bindgen::JsCast;
use web_sys::{DomParser, Element, SupportedType, SvgElement};

use crate::{SVG_NS, console_log, document};

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

/// Rendering component
pub struct Model {
    /// Id of the corresponding svg element
    svg_id: String,
    order: i32,
    order_changed: bool,
}

impl Component for Model {
    type Storage = DenseVecStorage<Self>;
}

impl Model {
    pub const ORDER_BOARD: i32 = 0;

    /// Adds an SVG to a parent node, taking a counter that is used for the id and increments.
    /// Also takes a rendering order.
    /// Then returns a `Model`.
    pub fn new(svg_elem: &SvgElement, order: i32, parent: &SvgElement, id: &mut u64) -> Self {
        svg_elem.set_id(&id.to_string());
        *id += 1;
        parent.append_child(&svg_elem).expect("Failed to add SVG");
        Model { svg_id: svg_elem.id(), order, order_changed: true }
    }
}

impl Drop for Model {
    /// Delete the SVG component
    fn drop(&mut self) {
        if let Some(element) = document().get_element_by_id(&self.svg_id) {
            element.remove();
        }
    }
}

/// An SVG is used for collision
pub struct Collider {
    /// id of the corresponding svg element
    svg_id: String,
    order: i32,
    order_changed: bool,
    hovered: bool,
    hovered_raw: bool,
}

impl Component for Collider {
    type Storage = DenseVecStorage<Self>;
}

impl Collider {
    pub const ORDER_START_PORT: i32 = 0;

    pub fn new(svg_elem: &SvgElement, order: i32, parent: &SvgElement, id: &mut u64) -> Self {
        svg_elem.set_id(&id.to_string());
        *id += 1;
        parent.append_child(&svg_elem).expect("Failed to add SVG");
        Collider {
            svg_id: svg_elem.id(),
            order: order - i32::MIN / 2,
            order_changed: true,
            hovered: false,
            hovered_raw: false,
        }
    }

    /// Whether the collider is being hovered over
    pub fn hovered(&self) -> bool {
        self.hovered
    }
}

impl Drop for Collider {
    /// Delete the SVG component
    fn drop(&mut self) {
        if let Some(element) = document().get_element_by_id(&self.svg_id) {
            element.remove();
        }
    }
}

/// Updates collider inputs
pub struct ColliderInputSystem;

impl<'a> System<'a> for ColliderInputSystem {
    type SystemData = WriteStorage<'a, Collider>;

    fn run(&mut self, mut data: Self::SystemData) {
        for collider in (&mut data).join() {
            collider.hovered = collider.hovered_raw;
        }
    }
}

/// Orders nodes to render
pub struct SvgOrderSystem;

impl<'a> System<'a> for SvgOrderSystem {
    type SystemData = (WriteStorage<'a, Model>, WriteStorage<'a, Collider>);

    fn run(&mut self, (mut models, mut colliders): Self::SystemData) {
        // Reorder nodes, since z-index isn't consistently supported
        let groups = chain!(
            (&mut models).join().map(|m| (&m.svg_id, m.order, &mut m.order_changed)),
            (&mut colliders).join().map(|c| (&c.svg_id, c.order, &mut c.order_changed)),
            )
            .group_by(|(svg_id, _, _)| {
                document().get_element_by_id(svg_id).unwrap()
                    .parent_element().expect("SVG node parents should have ids for sorting purposes").id()
            });

        for (parent_id, group) in groups.into_iter() {
            let mut values = group.collect_vec();
            // Sort only if some node changed order
            if values.iter().all(|(_, _, order_changed)| !**order_changed) {
                continue;
            }

            values.sort_by_key(|(_, order, _)| *order);
            let parent = document().get_element_by_id(&parent_id).expect("SVG node unexpectedly removed");
            for (svg_id, _, order_changed) in values {
                let elem = document().get_element_by_id(svg_id).expect("SVG node unexpectedly removed");
                let node = parent.remove_child(&elem).expect("Failed to reorder");
                parent.append_child(&node).expect("Failed to reorder");
                *order_changed = false;
            }
        }
    }
}

/// Extension trait for Board, mainly for rendering since
/// the server should know nothing about rendering
pub trait BoardExt: Board {
    fn render(&self) -> SvgElement;

    fn port_position(&self, port: &Self::Port) -> Vec2;
}

impl BoardExt for RectangleBoard {
    fn render(&self) -> SvgElement {
        let svg_str = format!(r##"<g xmlns="{}" fill="#ffd090" stroke="#806048">"##, SVG_NS) +
            &chain!(
                iproduct!(0..self.height(), 0..self.width()).map(|(y, x)|
                    format!(r##"<rect x="{}" y="{}" stroke-width="0.04" width="1" height="1"/>"##, x, y)),
                self.boundary_ports().into_iter().map(|(min, d)| {
                    let v = self.port_position(&(min, d));
                    let dx = if d.x == 0 { 0.1 } else { 0.0 };
                    let dy = if d.y == 0 { 0.1 } else { 0.0 };
                    format!(r##"<line x1="{}" x2="{}" y1="{}" y2="{}" stroke-width="0.05"/>"##, v.x - dx, v.x + dx, v.y - dy, v.y + dy)
                })
            )
                .join("") +
            r##"</g>"##;

        parse_svg(&svg_str)
    }

    fn port_position(&self, port: &<Self as Board>::Port) -> Vec2 {
        port.0.cast::<f64>() + port.1.cast::<f64>() / (self.ports_per_edge() + 1) as f64
    }
}

/// Extension trait for BaseBoard, mainly for rendering since
/// the server should know nothing about rendering
pub trait BaseBoardExt {
    fn render(&self) -> SvgElement;
    
    fn port_position(&self, port: &BasePort) -> Vec2;
}

for_each_board! {
    x, t => 

    impl BaseBoardExt for BaseBoard {
        fn render(&self) -> SvgElement {
            match self {
                $($x(b) => b.render()),*
            }
        }

        fn port_position(&self, port: &BasePort) -> Vec2 {
            match self {
                $($x(b) => b.port_position(<$t as Board>::Port::unwrap_base_ref(port))),*
            }
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
    fn start_ports_and_positions(&self) -> Vec<(Self::Port, Vec2)> {
        self.start_ports().into_iter()
            .map(|port| (port.clone(), self.board().port_position(&port)))
            .collect()
    }

    /// Colliders for starting ports
    fn start_port_colliders(&self) -> Vec<SvgElement> {
        self.start_ports_and_positions().into_iter()
            .map(|(port, pos)| {
                let svg_str = format!(concat!(
                    r##"<g xmlns="{0}" data-port="{1}" data-x="{2}" data-y="{3}" fill="#ff40d0">"##,
                    r##"<circle cx="{2}" cy="{3}" r="0.15"/>"##,
                    r##"</g>"##,
                ), SVG_NS, html_escape::encode_double_quoted_attribute(&serde_json::to_string(&port.wrap_base()).unwrap()), pos.x, pos.y);
                parse_svg(&svg_str)
            })
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
    fn start_ports_and_positions(&self) -> Vec<(BasePort, Vec2)>;

    fn start_port_colliders(&self) -> Vec<SvgElement>;
}

for_each_game! {
    x, t => 

    impl BaseGameExt for BaseGame {
        fn start_ports_and_positions(&self) -> Vec<(BasePort, Vec2)> {
            match self {
                $($x(g) => g.start_ports_and_positions().into_iter()
                    .map(|(port, pos)| (port.wrap_base(), pos))
                    .collect()),*
            }
        }

        fn start_port_colliders(&self) -> Vec<SvgElement> {
            match self {
                $($x(g) => g.start_port_colliders()),*
            }
        }
    }
}
