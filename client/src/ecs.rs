use std::cell::RefCell;
use std::collections::{HashSet};

use std::rc::Rc;

use std::{cell::Cell};
use std::fmt::Debug;

use common::game::GameId;
use common::{GameInstance};

use common::math::{Pt2, pt2};

use common::{board::{BasePort}};
use common::board::{BaseTLoc};
use common::tile::{BaseGAct, BaseKind, BaseTile};
use getset::{CopyGetters, Getters, MutGetters};
use itertools::{Itertools};
use specs::prelude::*;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{Element, KeyboardEvent, MouseEvent, SvgGraphicsElement};


use crate::render::{BaseTileExt, SvgMatrixExt, self};
use crate::{document};

/// Labels a game in the lobby with a GameInstance
#[derive(Clone, Debug)]
pub struct GameInstanceLabel(pub GameInstance);

impl Component for GameInstanceLabel {
    type Storage = DenseVecStorage<Self>;
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
    type SystemData = (ReadStorage<'a, Transform>, ReadStorage<'a, Model>);

    fn run(&mut self, (transforms, models): Self::SystemData) {
        self.changed.clear();

        for event in transforms.channel().read(&mut self.reader_id) {
            if let ComponentEvent::Modified(id) | ComponentEvent::Inserted(id) = event {
                self.changed.add(*id);
            }
        }

        for (transform, model, _) in (&transforms, &models, &self.changed).join() {
            let svg = document().get_element_by_id(&model.id).unwrap();
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

/// Labels an entity with a tile location
#[derive(Clone, Debug)]
pub struct TLocLabel(pub BaseTLoc);

impl Component for TLocLabel {
    type Storage = DenseVecStorage<Self>;
}

/// Labels an entity with a tile
/// 
/// Group actions are *not* preapplied to the tile.
#[derive(Clone, Debug)]
pub struct TileLabel(pub BaseTile);

impl Component for TileLabel {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Debug, Getters, MutGetters, CopyGetters)]
pub struct TileSelect {
    /// Whether this entity is a selected tile
    selected: bool,
    #[getset(get = "pub")]
    kind: BaseKind,
    #[getset(get_copy = "pub", get_mut = "pub")]
    index: u32,
    action: BaseGAct,
}

impl TileSelect {
    pub fn new(kind: BaseKind, index: u32, action: BaseGAct) -> Self {
        Self { selected: false, kind, index, action }
    }
}

impl Component for TileSelect {
    type Storage = DenseVecStorage<Self>;
}

/// Rendering component
#[derive(Debug)]
pub struct Model {
    /// Id of the corresponding element
    id: String,
    order: i32,
    order_changed: bool,
}

impl Component for Model {
    type Storage = DenseVecStorage<Self>;
}

impl Model {
    pub const ORDER_BOARD: i32 = 0;
    pub const ORDER_TILE: i32 = 1;
    pub const ORDER_PLAYER_TOKEN: i32 = 2;
    pub const ORDER_TILE_HOVER: i32 = 3;

    /// Adds an element to a parent node, taking a counter that is used for the id and increments.
    /// Also takes a rendering order.
    /// Then returns a `Model`.
    pub fn new(elem: &Element, order: i32, parent: &Element, id: &mut u64) -> Self {
        elem.set_id(&id.to_string());
        *id += 1;
        parent.append_child(&elem).expect("Failed to add element");
        Model { id: elem.id(), order, order_changed: true }
    }
}

impl Drop for Model {
    /// Delete the SVG component
    fn drop(&mut self) {
        if let Some(element) = document().get_element_by_id(&self.id) {
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

/// Keyboard input for the game
#[derive(Debug)]
pub struct KeyboardInput {
    keys_down_raw: Rc<RefCell<HashSet<String>>>,
    keys_down: HashSet<String>,
    keys_pressed: HashSet<String>,
    keydown_listener: Closure<dyn FnMut(KeyboardEvent)>,
    keyup_listener: Closure<dyn FnMut(KeyboardEvent)>,
}

impl KeyboardInput {
    /// Constructs a `KeyboardInput` that gets keyboard events from a specific element.
    pub fn new(elem: &Element) -> Self {
        let keys_down_raw = Rc::new(RefCell::new(HashSet::new()));
        let keys_clone = Rc::clone(&keys_down_raw);

        let keydown_listener = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            keys_clone.borrow_mut().insert(e.code());
        }) as Box<dyn FnMut(KeyboardEvent)>);
        let keys_clone = Rc::clone(&keys_down_raw);
        let keyup_listener = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            keys_clone.borrow_mut().remove(&e.code());
        }) as Box<dyn FnMut(KeyboardEvent)>);

        elem.add_event_listener_with_callback("keydown", keydown_listener.as_ref().unchecked_ref())
            .expect("Failed to add input callback");
        elem.add_event_listener_with_callback("keyup", keyup_listener.as_ref().unchecked_ref())
            .expect("Failed to add input callback");

        Self {
            keys_down_raw,
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keydown_listener,
            keyup_listener
        }
    }

    /// Whether a key is pressed
    pub fn pressed(&self, key: &str) -> bool {
        self.keys_pressed.contains(key)
    }
}

/// Updates keys_down and keys_pressed
pub struct KeyboardInputSystem;

impl<'a> System<'a> for KeyboardInputSystem {
    type SystemData = Option<Write<'a, KeyboardInput>>;

    fn run(&mut self, data: Self::SystemData) {
        let mut data = data.expect("Missing KeyboardInput");
        let keys_pressed = data.keys_down_raw.borrow().difference(&data.keys_down).cloned().collect();
        let keys_down = data.keys_down_raw.borrow().clone();
        data.keys_pressed = keys_pressed;
        data.keys_down = keys_down;
    }
}

/// Labels something with a keyboard key
#[derive(Clone, Debug)]
pub struct KeyLabel(pub String);

impl Component for KeyLabel {
    type Storage = HashMapStorage<Self>;
}

/// Group action performed by a button press
#[derive(Clone, Copy, Debug)]
pub enum ButtonAction {
    Rotation{ num_times: i32 }
}

impl ButtonAction {
    /// Generate the corresponding group action
    pub fn group_action(&self, tile: &BaseTile) -> BaseGAct {
        match self {
            Self::Rotation{ num_times } => tile.rotation_action(*num_times)
        }
    }
}

impl Component for ButtonAction {
    type Storage = HashMapStorage<Self>;
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
    pub const ORDER_START_PORT: i32 = -(i32::MIN / 2) + 1;
    pub const ORDER_TILE_LOC: i32 = -(i32::MIN / 2) + 0;

    /// Constructs a collider.
    /// Takes an element to insert callbacks into
    pub fn new(elem: &Element) -> Self {
        let hovered_raw = Rc::new(Cell::new(false));
        let hovered_clone = Rc::clone(&hovered_raw);
        let mouseover_listener = Closure::wrap(Box::new(move |_e: MouseEvent| {
            hovered_clone.set(true);
        }) as Box<dyn FnMut(MouseEvent)>);
        let hovered_clone = Rc::clone(&hovered_raw);
        let mouseout_listener = Closure::wrap(Box::new(move |_e: MouseEvent| {
            hovered_clone.set(false);
        }) as Box<dyn FnMut(MouseEvent)>);

        elem.add_event_listener_with_callback("mouseover", mouseover_listener.as_ref().unchecked_ref())
            .expect("Failed to add collider callback");
        elem.add_event_listener_with_callback("mouseout", mouseout_listener.as_ref().unchecked_ref())
            .expect("Failed to add collider callback");

        let clicked_raw = Rc::new(Cell::new(false));
        let clicked_clone = Rc::clone(&clicked_raw);
        let click_listener = Closure::wrap(Box::new(move |_e: MouseEvent| {
            clicked_clone.set(true);
        }) as Box<dyn FnMut(MouseEvent)>);

        elem.add_event_listener_with_callback("click", click_listener.as_ref().unchecked_ref())
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
    type SystemData = WriteStorage<'a, Model>;

    fn run(&mut self, mut models: Self::SystemData) {
        // Reorder nodes, since z-index isn't consistently supported
        let groups = (&mut models).join()
            .map(|m| (&m.id, m.order, &mut m.order_changed))
            .sorted_by_key(|(svg_id, _, _)| {
                document().get_element_by_id(svg_id).unwrap()
                    .parent_element().expect("SVG node parents should have ids for sorting purposes").id()
            })
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
            for (svg_id, _order, order_changed) in values {
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

/// The port a token was placed on
#[derive(Clone, Debug, Default)]
pub struct PlacedPort(pub Option<BasePort>);

#[derive(Clone, Copy, Debug, Default)]
pub struct RunPlaceTokenSystem(pub bool);

pub struct PlaceTokenSystem;

#[derive(SystemData)]
pub struct PlaceTokenSystemData<'a> {
    run: Read<'a, RunPlaceTokenSystem>,
    placed_port: Write<'a, PlacedPort>,
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
        if !data.run.0 { return }

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
                data.placed_port.0 = Some(port.0.clone());
                break;
            }
        }
    }
}

/// A place where the player token can get added
#[derive(Clone, Copy, Debug, Default)]
pub struct TileSlot;

impl Component for TileSlot {
    type Storage = NullStorage<Self>;
}

/// The token that's being placed
#[derive(Clone, Copy, Debug, Default)]
pub struct TileToPlace;

impl Component for TileToPlace {
    type Storage = NullStorage<Self>;
}

/// The location a tile was placed on
#[derive(Clone, Debug, Default)]
pub struct PlacedTLoc(pub Option<BaseTLoc>);

#[derive(Clone, Copy, Debug, Default)]
pub struct RunPlaceTileSystem(pub bool);

pub struct PlaceTileSystem;

#[derive(SystemData)]
pub struct PlaceTileSystemData<'a> {
    run: Read<'a, RunPlaceTileSystem>,
    placed_loc: Write<'a, PlacedTLoc>,
    tiles: ReadStorage<'a, TileToPlace>,
    tile_slots: ReadStorage<'a, TileSlot>,
    colliders: ReadStorage<'a, Collider>,
    locs: ReadStorage<'a, TLocLabel>,
    transforms: WriteStorage<'a, Transform>,
    input: Option<Read<'a, BoardInput>>,
}

impl<'a> System<'a> for PlaceTileSystem {
    type SystemData = PlaceTileSystemData<'a>;
    
    fn run(&mut self, mut data: Self::SystemData) {
        if !data.run.0 { return }

        let position = (&data.tile_slots, &data.colliders, &data.transforms).join()
            .flat_map(|(_, collider, transform)| {
                collider.hovered().then(|| transform.position)
            })
            .next();

        for (_, transform) in (&data.tiles, &mut data.transforms).join() {
            transform.position = if let Some(position) = position {
                position
            } else {
                data.input.as_ref().expect("Missing BoardInput").position()
            }
        }

        for (_, collider, loc) in (&data.tile_slots, &data.colliders, &data.locs).join() {
            if collider.clicked() {
                data.placed_loc.0 = Some(loc.0.clone());
                break;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RunSelectTileSystem(pub bool);

pub struct SelectTileSystem;

/// The tile that's currently selected, paired with its index and group action
/// into the list of tiles the player has of the same kind.
/// 
/// The group is *not* preapplied to the tile.
#[derive(Clone, Debug, Default)]
pub struct SelectedTile(pub u32, pub Option<BaseGAct>, pub Option<BaseTile>);

#[derive(SystemData)]
pub struct SelectTileSystemData<'a> {
    run: Read<'a, RunSelectTileSystem>,
    selected_tile: Write<'a, SelectedTile>,
    keyboard_input: Option<Read<'a, KeyboardInput>>,
    models: ReadStorage<'a, Model>,
    colliders: ReadStorage<'a, Collider>,
    tiles: ReadStorage<'a, TileLabel>,
    tile_selects: WriteStorage<'a, TileSelect>,
    button_actions: ReadStorage<'a, ButtonAction>,
    key_labels: ReadStorage<'a, KeyLabel>,
}

impl<'a> System<'a> for SelectTileSystem {
    type SystemData = SelectTileSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        if !data.run.0 { return; }

        // Edit group action if necessary
        let selected_tile = &mut *data.selected_tile;
        let keyboard_input = data.keyboard_input.expect("Missing KeyboardInput");
        if let (Some(action), Some(tile)) = (&mut selected_tile.1, &selected_tile.2) {
            for (collider, button_action, key) in (&data.colliders, &data.button_actions, &data.key_labels).join() {
                if collider.clicked() || keyboard_input.pressed(&key.0) {
                    *action = action.compose(&button_action.group_action(tile));
                }
            }
        }

        for (model, tile_select, tile) in (&data.models, &mut data.tile_selects, &data.tiles).join() {
            let elem = document().get_element_by_id(&model.id).expect("Missing model element");

            // Replace rendered tile if necessary
            if tile_select.selected {
                if let Some(action) = data.selected_tile.1.clone() {
                    if action != tile_select.action {
                        let old = elem.first_child().expect("Expected a tile svg");
                        let new = render::parse_svg(&tile.0.apply_action(&action).render());
                        elem.replace_child(&new, &old).expect("Failed to replace tile svg");
                        tile_select.action = action;
                    }
                }
            }
        }

        // Only do something when the selection is modified
        if (&data.colliders, &data.tile_selects).join().all(|(c, _)| !c.clicked()) {
            return;
        }

        let mut found_selected = false;

        for (collider, tile, tile_select) in (&data.colliders, &data.tiles, &mut data.tile_selects).join() {
            if found_selected {
                tile_select.selected = false;
                continue;
            }

            tile_select.selected = collider.clicked();
            if collider.clicked() {
                found_selected = true;
                data.selected_tile.0 = tile_select.index;
                data.selected_tile.1 = Some(tile_select.action.clone());
                data.selected_tile.2 = Some(tile.0.clone());
            }
        }

        // Update selection visualization
        for (model, tile_select, _tile) in (&data.models, &mut data.tile_selects, &data.tiles).join() {
            let elem = document().get_element_by_id(&model.id).expect("Missing model element");
            elem.set_attribute(
                "class", 
                if tile_select.selected { "bottom-tile tile-selected" } else { "bottom-tile tile-unselected" }
            ).expect("Cannot set tile select style");
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RunSelectGameSystem(pub bool);

pub struct SelectGameSystem;

/// The game id that's been clicked, if any
#[derive(Clone, Debug, Default)]
pub struct SelectedGame(pub Option<GameId>);

#[derive(SystemData)]
pub struct SelectGameSystemData<'a> {
    run: Read<'a, RunSelectGameSystem>,
    selected_game: Write<'a, SelectedGame>,
    colliders: ReadStorage<'a, Collider>,
    games: ReadStorage<'a, GameInstanceLabel>,
}

impl<'a> System<'a> for SelectGameSystem {
    type SystemData = SelectGameSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        if !data.run.0 { return; }

        for (game, collider) in (&data.games, &data.colliders).join() {
            if collider.clicked() {
                data.selected_game.0 = Some(game.0.id());
                break;
            }
        }
    }
}