use std::sync::mpsc::{self, Receiver};

use common::{board::BasePort, game::{BaseGame, GenericGame}, game_state::BaseGameState, math::{Pt2, Vec2}, message::{Request, Response}, tile::Tile};
use itertools::Itertools;
use specs::{Builder, Dispatcher, DispatcherBuilder, Entity, World, WorldExt};
use wasm_bindgen::JsCast;
use web_sys::{Element, SvgElement};
use enum_dispatch::enum_dispatch;

use crate::{console_log, document, ecs::{BoardInput, ButtonAction, Collider, ColliderInputSystem, KeyLabel, KeyboardInput, KeyboardInputSystem, Model, PlaceTileSystem, PlaceTokenSystem, PlacedPort, PlacedTLoc, PortLabel, RunPlaceTileSystem, RunPlaceTokenSystem, RunSelectTileSystem, SelectTileSystem, SelectedTile, SvgOrderSystem, TLocLabel, TileLabel, TileSelect, TileSlot, TileToPlace, TokenSlot, TokenToPlace, Transform, TransformSystem}, render::{self, BaseBoardExt, BaseGameExt, BaseTileExt}};

mod app;
use app::{gameplay, AppStateT};

/// The game and state, including components such as collision and rendering
pub struct GameWorld {
    /// None if the state is being edited
    state: Option<app::State>,
    world: World,
    id_counter: u64,
    dispatcher: Dispatcher<'static, 'static>,
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
        world.insert(BoardInput::new(&document().get_element_by_id("svg_root").expect("Missing main panel svg")
            .dyn_into().expect("Not an <svg> element")));
        world.insert(KeyboardInput::new(&document().document_element().expect("Missing root element. What?!")));
        world.insert(RunPlaceTokenSystem(true));
        world.insert(RunSelectTileSystem(true));
        world.insert(RunPlaceTileSystem(true));
        world.insert(PlacedPort(None));
        world.insert(SelectedTile(0, None, None));
        world.insert(PlacedTLoc(None));

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
            .with(SvgOrderSystem, "svg_order", &[])
            .with(ColliderInputSystem, "collider_input", &[])
            .with(KeyboardInputSystem, "keyboard_input", &[])
            .with(PlaceTokenSystem, "place_token", &["collider_input", "keyboard_input"])
            .with(PlaceTileSystem, "place_tile", &["collider_input", "keyboard_input"])
            .with(TransformSystem::new(&world), "transform", &["place_token", "place_tile"])
            .with(SelectTileSystem, "select_tile", &["collider_input", "keyboard_input"])
            .build();

        Self {
            state: Some(app::EnterUsername.into()),
            world,
            id_counter: 0,
            dispatcher,
        }
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
            .dyn_into().unwrap()
    }

    /// Constructs a game world from a game and state.
    /// This is meant to be called with `self.state == None` and returns the new world state.
    pub fn set_game(&mut self, game: BaseGame, state: BaseGameState) -> app::Game {
        let board_svg = game.board().render();
        let start_ports = game.start_ports_and_positions().into_iter()
            .map(|(port, position)| {
                let svg = render::render_port_collider();
                self.world.create_entity()
                    .with(Transform::new(position))
                    .with(Model::new(
                        &svg,
                        Collider::ORDER_START_PORT,
                        &Self::svg_root(),
                        &mut self.id_counter
                    ))
                    .with(Collider::new(&svg))
                    .with(TokenSlot)
                    .with(PortLabel(port))
                    .build()
            })
            .collect_vec();
        let board_entity = self.world.create_entity()
            .with(Model::new(&board_svg, Model::ORDER_BOARD, &Self::svg_root(), &mut self.id_counter))
            .build();
        let token_entity = self.world.create_entity()
            .with(Transform::new(Pt2::origin()))
            .with(Model::new(
                &render::render_token(state.looker_expect(), state.num_players(), &mut self.id_counter),
                Model::ORDER_PLAYER_TOKEN, 
                &Self::svg_root(), &mut self.id_counter
            ))
            .with(TokenToPlace)
            .build();

        let num_players = state.num_players();
        let ports = (0..num_players)
            .map(|player| state.board_state().player_port(player))
            .collect_vec();

        let tile_hand_entities = state.player_state(state.looker_expect())
            .map_or(vec![], |state| state.tiles_vec())
            .into_iter()
            .flat_map(|(kind, tiles)| {
                tiles.into_iter().enumerate().map(move |(index, tile)| (kind.clone(), index as u32, tile))
            })
            .map(|(_, index, tile)| tile.create_hand_entity(
                index,
                &tile.identity_action(),
                &mut self.world,
                &mut self.id_counter,
            ))
            .collect_vec();

        let mut game_state = app::Game {
            game,
            state,
            board_entity,
            token_entities: vec![None; num_players as usize],
            tile_hand_entities, 
            board_tile_entities: vec![],
            gameplay_state: Some(gameplay::PlaceToken{ start_ports, token_entity }.into()),
        };

        // For spectators: add ports that have already been placed
        for (player, port) in ports.into_iter().enumerate() {
            if let Some(port) = port {
                game_state.set_token_position(self, player as u32, &port);
            }
        }

        game_state
    }

    pub fn update(&mut self) -> Vec<Request> {
        self.dispatcher.dispatch(&mut self.world);

        let mut requests = vec![];

        self.state = Some(self.state.take()
            .expect("State is missing")
            .update(self, &mut requests));

        requests
    }

    pub fn handle_response(&mut self, response: Response) -> Vec<Request> {
        let mut requests = vec![];

        if let Response::Usernames{ names } = response {
            let names_str = names.into_iter().join("\n\n");
            document().get_element_by_id("usernames").unwrap().set_inner_html(&names_str);
        } else {
            self.state = Some(self.state.take()
                .expect("State is missing")
                .handle_response(self, response, &mut requests));
        }

        requests
    }
}