use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::{cell::Cell, marker::PhantomData};
use std::fmt::Debug;
use std::hash::Hash;
use common::nalgebra as na;

use common::math::{Pt2, Vec2f, Vec3f, Vec3u, pt2};
use common::nalgebra::ComplexField;
use common::{board::{BaseBoard, BasePort, Board, RectangleBoard}, for_each_board, for_each_game, game::{BaseGame, Game, PathGame}, math::Vec2, tile::{RegularTile, Tile}};
use common::board::Port;
use common::tile::Kind;
use itertools::{Itertools, chain, iproduct};
use specs::prelude::*;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{DomParser, Element, MouseEvent, SupportedType, SvgElement, SvgGraphicsElement, SvgMatrix, SvgsvgElement};

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

trait SvgMatrixExt {
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

/// Transformation component. Sets transform of other objects
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Pt2,
}

impl Component for Transform {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Transform {
    pub fn new(position: Pt2) -> Self {
        Self { position }
    }
}

/// Sets transforms
pub struct TransformSystem {
    reader_id: ReaderId<ComponentEvent>,
    changed: BitSet,
}

impl TransformSystem {
    pub fn new(world: &World) -> Self {
        let mut storage = world.write_storage::<Transform>();
        Self {
            reader_id: storage.register_reader(),
            changed: BitSet::new(),
        }
    }
}

impl<'a> System<'a> for TransformSystem {
    type SystemData = (ReadStorage<'a, Transform>, ReadStorage<'a, SvgModel>);

    fn run(&mut self, (transforms, models): Self::SystemData) {
        self.changed.clear();

        for event in transforms.channel().read(&mut self.reader_id) {
            if let ComponentEvent::Modified(id) | ComponentEvent::Inserted(id) = event {
                self.changed.add(*id);
            }
        }

        for (transform, model, _) in (&transforms, &models, &self.changed).join() {
            let svg = document().get_element_by_id(&model.svg_id).unwrap();
            svg.set_attribute("transform", &format!("translate({}, {})", transform.position.x, transform.position.y))
                .expect("Cannot change transform");
        }
    }
}

/// Labels an entity with a port
#[derive(Clone, Debug)]
pub struct PortLabel(pub BasePort);

impl Component for PortLabel {
    type Storage = DenseVecStorage<Self>;
}

/// Rendering component
#[derive(Debug)]
pub struct SvgModel {
    /// Id of the corresponding svg element
    svg_id: String,
    order: i32,
    order_changed: bool,
}

impl Component for SvgModel {
    type Storage = DenseVecStorage<Self>;
}

impl SvgModel {
    pub const ORDER_BOARD: i32 = 0;
    pub const ORDER_PLAYER_TOKEN: i32 = 1;

    /// Adds an SVG to a parent node, taking a counter that is used for the id and increments.
    /// Also takes a rendering order.
    /// Then returns a `Model`.
    pub fn new(svg_elem: &SvgElement, order: i32, parent: &SvgElement, id: &mut u64) -> Self {
        svg_elem.set_id(&id.to_string());
        *id += 1;
        parent.append_child(&svg_elem).expect("Failed to add SVG");
        SvgModel { svg_id: svg_elem.id(), order, order_changed: true }
    }
}

impl Drop for SvgModel {
    /// Delete the SVG component
    fn drop(&mut self) {
        if let Some(element) = document().get_element_by_id(&self.svg_id) {
            element.remove();
        }
    }
}

/// Mouse input tracker for the SVG region where the board shows
#[derive(Debug)]
pub struct BoardInput {
    /// Position of the mouse, in board space
    position: Pt2,
    position_raw: Rc<Cell<Pt2>>,
    callback: Closure<dyn FnMut(MouseEvent)>,
}

impl BoardInput {
    /// Constructs a `BoardInput` that gets mouse events from a specific SVG graphics element
    pub fn new(elem: &SvgGraphicsElement) -> Self {
        let position_raw = Rc::new(Cell::new(Pt2::origin()));
        let position_clone = Rc::clone(&position_raw);
        
        let elem_clone = elem.clone();
        let mousemove_listener = Closure::wrap(Box::new(move |e: MouseEvent| {
            let position = elem_clone.get_screen_ctm()
                .expect("Missing SVG matrix")
                .inverse().expect("Cannot inverse SVG matrix")
                .transform(pt2(e.x() as f64, e.y() as f64));
            position_clone.set(position);
        }) as Box<dyn FnMut(MouseEvent)>);
        elem.add_event_listener_with_callback("mousemove", mousemove_listener.as_ref().unchecked_ref())
            .expect("Failed to add input callback");

        Self {
            position: Pt2::origin(),
            position_raw,
            callback: mousemove_listener,
        }
    }

    fn position(&self) -> Pt2 {
        self.position
    }
}

/// An SVG is used for collision
#[derive(Debug)]
pub struct Collider {
    hovered: bool,
    clicked: bool,
    hovered_raw: Rc<Cell<bool>>,
    clicked_raw: Rc<Cell<bool>>,
    mouseover_listener: Closure<dyn FnMut(MouseEvent)>,
    mouseout_listener: Closure<dyn FnMut(MouseEvent)>,
    click_listener: Closure<dyn FnMut(MouseEvent)>,
}

impl Component for Collider {
    type Storage = DenseVecStorage<Self>;
}

impl Collider {
    pub const ORDER_START_PORT: i32 = -(i32::MIN / 2) + 0;

    /// Constructs a collider.
    /// Takes an svg element to insert callbacks into
    pub fn new(svg_elem: &SvgElement) -> Self {
        let hovered_raw = Rc::new(Cell::new(false));
        let hovered_clone = Rc::clone(&hovered_raw);
        let mouseover_listener = Closure::wrap(Box::new(move |e: MouseEvent| {
            hovered_clone.set(true);
        }) as Box<dyn FnMut(MouseEvent)>);
        let hovered_clone = Rc::clone(&hovered_raw);
        let mouseout_listener = Closure::wrap(Box::new(move |e: MouseEvent| {
            hovered_clone.set(false);
        }) as Box<dyn FnMut(MouseEvent)>);

        svg_elem.add_event_listener_with_callback("mouseover", mouseover_listener.as_ref().unchecked_ref())
            .expect("Failed to add collider callback");
        svg_elem.add_event_listener_with_callback("mouseout", mouseout_listener.as_ref().unchecked_ref())
            .expect("Failed to add collider callback");

        let clicked_raw = Rc::new(Cell::new(false));
        let clicked_clone = Rc::clone(&clicked_raw);
        let click_listener = Closure::wrap(Box::new(move |e: MouseEvent| {
            clicked_clone.set(true);
        }) as Box<dyn FnMut(MouseEvent)>);

        svg_elem.add_event_listener_with_callback("click", click_listener.as_ref().unchecked_ref())
            .expect("Failed to add collider callback");

        Collider {
            hovered: false,
            clicked: false,
            hovered_raw,
            clicked_raw,
            mouseover_listener,
            mouseout_listener,
            click_listener,
        }
    }

    /// Whether the collider is being hovered over
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// Whether the collider is being clicked on this frame
    pub fn clicked(&self) -> bool {
        self.clicked
    }
}

/// Updates collider inputs
pub struct ColliderInputSystem;

impl<'a> System<'a> for ColliderInputSystem {
    // Option<Write<..>> is used even though the resource is strictly required
    // because BoardInput doesn't have a default
    type SystemData = (WriteStorage<'a, Collider>, Option<Write<'a, BoardInput>>);

    fn run(&mut self, (mut colliders, input): Self::SystemData) {
        for collider in (&mut colliders).join() {
            collider.hovered = collider.hovered_raw.get();
            collider.clicked = collider.clicked_raw.get();
            collider.clicked_raw.set(false);
        }

        let mut input = input.expect("Missing BoardInput");
        input.position = input.position_raw.get();
    }
}

/// Orders nodes to render
pub struct SvgOrderSystem;

impl<'a> System<'a> for SvgOrderSystem {
    type SystemData = WriteStorage<'a, SvgModel>;

    fn run(&mut self, mut models: Self::SystemData) {
        // Reorder nodes, since z-index isn't consistently supported
        let groups = (&mut models).join()
            .map(|m| (&m.svg_id, m.order, &mut m.order_changed))
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

/// A place where the player token can get added
#[derive(Clone, Copy, Debug, Default)]
pub struct TokenSlot;

impl Component for TokenSlot {
    type Storage = NullStorage<Self>;
}

/// The token that's being placed
#[derive(Clone, Copy, Debug, Default)]
pub struct TokenToPlace;

impl Component for TokenToPlace {
    type Storage = NullStorage<Self>;
}

pub struct PlaceTokenSystem {
    port_sender: Sender<BasePort>,
}

impl PlaceTokenSystem {
    pub fn new(port_sender: Sender<BasePort>) -> Self {
        Self { port_sender }
    }
}

#[derive(SystemData)]
pub struct PlaceTokenSystemData<'a> {
    tokens: ReadStorage<'a, TokenToPlace>,
    token_slots: ReadStorage<'a, TokenSlot>,
    colliders: ReadStorage<'a, Collider>,
    ports: ReadStorage<'a, PortLabel>,
    transforms: WriteStorage<'a, Transform>,
    input: Option<Read<'a, BoardInput>>,
}

impl<'a> System<'a> for PlaceTokenSystem {
    type SystemData = PlaceTokenSystemData<'a>;
    
    fn run(&mut self, mut data: Self::SystemData) {
        let position = (&data.token_slots, &data.colliders, &data.transforms).join()
            .flat_map(|(_, collider, transform)| {
                collider.hovered().then(|| transform.position)
            })
            .next();

        for (_, transform) in (&data.tokens, &mut data.transforms).join() {
            transform.position = if let Some(position) = position {
                position
            } else {
                data.input.as_ref().expect("Missing BoardInput").position()
            }
        }

        for (_, collider, port) in (&data.token_slots, &data.colliders, &data.ports).join() {
            if collider.clicked() {
                self.port_sender.send(port.0.clone()).expect("Port receiver is missing");
                break;
            }
        }
    }
}

/// Extension trait for Board, mainly for rendering since
/// the server should know nothing about rendering
pub trait BoardExt: Board {
    fn render(&self) -> SvgElement;

    fn port_position(&self, port: &Self::Port) -> Pt2;
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

    fn port_position(&self, port: &<Self as Board>::Port) -> Pt2 {
        port.0.cast::<f64>() + port.1.cast::<f64>() / (self.ports_per_edge() + 1) as f64
    }
}

/// Extension trait for BaseBoard, mainly for rendering since
/// the server should know nothing about rendering
pub trait BaseBoardExt {
    fn render(&self) -> SvgElement;
    
    fn port_position(&self, port: &BasePort) -> Pt2;
}

for_each_board! {
    x, t => 

    impl BaseBoardExt for BaseBoard {
        fn render(&self) -> SvgElement {
            match self {
                $($x(b) => b.render()),*
            }
        }

        fn port_position(&self, port: &BasePort) -> Pt2 {
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
    x, t => 

    impl BaseGameExt for BaseGame {
        fn start_ports_and_positions(&self) -> Vec<(BasePort, Pt2)> {
            match self {
                $($x(g) => g.start_ports_and_positions().into_iter()
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