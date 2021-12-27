

use common::{message::{Request, Response}};
use specs::{Builder, Dispatcher, DispatcherBuilder, Entity, World, WorldExt};
use wasm_bindgen::JsCast;
use web_sys::{Element, SvgElement};


use crate::{document, ecs::{BoardInput, ButtonAction, Collider, ColliderInputSystem, KeyLabel, KeyboardInput, KeyboardInputSystem, Model, PlaceTileSystem, PlaceTokenSystem, PlacedPort, PlacedTLoc, PortLabel, RunPlaceTileSystem, RunPlaceTokenSystem, RunSelectTileSystem, SelectTileSystem, SelectedTile, SvgOrderSystem, TLocLabel, TileLabel, TileSelect, TileSlot, TileToPlace, TokenSlot, TokenToPlace, Transform, TransformSystem, GameInstanceLabel, RunSelectGameSystem, SelectGameSystem, SelectedGame}};

mod app;
use app::{AppStateT};

/// The game and state, including components such as collision and rendering
pub struct GameWorld {
    /// None if the state is being edited
    state: Option<app::State>,
    world: World,
    id_counter: u64,
    start_game_entity: Entity,
    leave_game_entity: Entity,
    dispatcher: Dispatcher<'static, 'static>,
    render_dispatcher: Dispatcher<'static, 'static>,
}

impl GameWorld {
    /// Constructs a game world
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<Model>();
        world.register::<Collider>();
        world.register::<TokenSlot>();
        world.register::<TokenToPlace>();
        world.register::<TileSlot>();
        world.register::<TileToPlace>();
        world.register::<Transform>();
        world.register::<PortLabel>();
        world.register::<TileLabel>();
        world.register::<TLocLabel>();
        world.register::<TileSelect>();
        world.register::<ButtonAction>();
        world.register::<KeyLabel>();
        world.register::<GameInstanceLabel>();
        world.insert(BoardInput::new(&document().get_element_by_id("svg_root").expect("Missing main panel svg")
            .dyn_into().expect("Not an <svg> element")));
        world.insert(KeyboardInput::new(&document().document_element().expect("Missing root element. What?!")));
        world.insert(RunPlaceTokenSystem(true));
        world.insert(RunSelectTileSystem(true));
        world.insert(RunPlaceTileSystem(true));
        world.insert(RunSelectGameSystem(true));
        world.insert(PlacedPort(None));
        world.insert(SelectedTile(0, None, None));
        world.insert(PlacedTLoc(None));
        world.insert(SelectedGame(None));

        world.create_entity()
            .with(Collider::new(&document().get_element_by_id("rotate_ccw").expect("Missing rotate ccw button")))
            .with(ButtonAction::Rotation{ num_times: -1 })
            .with(KeyLabel("KeyE".to_owned()))
            .build();

        world.create_entity()
            .with(Collider::new(&document().get_element_by_id("rotate_cw").expect("Missing rotate cw button")))
            .with(ButtonAction::Rotation{ num_times: 1 })
            .with(KeyLabel("KeyR".to_owned()))
            .build();

        let dispatcher = DispatcherBuilder::new()
            .with(ColliderInputSystem, "collider_input", &[])
            .with(KeyboardInputSystem, "keyboard_input", &[])
            .with(PlaceTokenSystem, "place_token", &["collider_input", "keyboard_input"])
            .with(PlaceTileSystem, "place_tile", &["collider_input", "keyboard_input"])
            .with(SelectTileSystem, "select_tile", &["collider_input", "keyboard_input"])
            .with(SelectGameSystem, "select_game", &["collider_input", "keyboard_input"])
            .build();

        let render_dispatcher = DispatcherBuilder::new()
            .with(SvgOrderSystem, "svg_order", &[])
            .with(TransformSystem::new(&world), "transform", &[])
            .build();

        let start_game_entity = world.create_entity()
            .with(Collider::new(&document().get_element_by_id("start_game").unwrap()))
            .build();

        let leave_game_entity = world.create_entity()
            .with(Collider::new(&document().get_element_by_id("leave_game").unwrap()))
            .build();

        Self {
            state: Some(app::EnterUsername::default().into()),
            world,
            id_counter: 0,
            start_game_entity,
            leave_game_entity,
            dispatcher,
            render_dispatcher,
        }
    }

    pub fn game_panel() -> Element {
        web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("game_panel").unwrap()
    }

    pub fn svg_root() -> SvgElement {
        web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("svg_root").unwrap()
            .dyn_into().unwrap()
    }

    pub fn bottom_panel() -> Element {
        web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("bottom_panel").unwrap()
    }

    pub fn update(&mut self) -> Vec<Request> {
        self.dispatcher.dispatch(&mut self.world);

        let mut requests = vec![];

        self.state = Some(self.state.take()
            .expect("State is missing")
            .update(self, &mut requests));

        self.render_dispatcher.dispatch(&mut self.world);

        requests
    }

    pub fn handle_response(&mut self, response: Response) -> Vec<Request> {
        let mut requests = vec![];

        self.state = Some(self.state.take()
            .expect("State is missing")
            .handle_response(self, response, &mut requests));

        requests
    }
}