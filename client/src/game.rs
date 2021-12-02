use std::sync::mpsc::{self, Receiver};

use common::{board::BasePort, game::{BaseGame, GenericGame}, game_state::BaseGameState, math::{Pt2, Vec2}, message::{Request, Response}, player_state::Looker, tile::Tile};
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
            .with(ColliderInputSystem, "collider_input", &[])
            .with(KeyboardInputSystem, "keyboard_input", &[])
            .with(PlaceTokenSystem, "place_token", &["collider_input", "keyboard_input"])
            .with(PlaceTileSystem, "place_tile", &["collider_input", "keyboard_input"])
            .with(SelectTileSystem, "select_tile", &["collider_input", "keyboard_input"])
            .build();

        let render_dispatcher = DispatcherBuilder::new()
            .with(SvgOrderSystem, "svg_order", &[])
            .with(TransformSystem::new(&world), "transform", &[])
            .build();

        Self {
            state: Some(app::EnterUsername::default().into()),
            world,
            id_counter: 0,
            dispatcher,
            render_dispatcher,
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
    pub fn set_game(&mut self, game: BaseGame, state: BaseGameState, usernames: Vec<String>) -> app::Game {
        let board_svg = render::parse_svg(&game.board().render());
        let board_entity = self.world.create_entity()
            .with(Model::new(&board_svg, Model::ORDER_BOARD, &Self::svg_root(), &mut self.id_counter))
            .build();

        let (tile_hand_entities, gameplay_state) = if let Looker::Player(player) = state.looker() {
            let tile_hand_entities = state.player_state(player)
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
                
            if state.all_players_placed() {
                // Rejoined game
                (tile_hand_entities, gameplay::WaitTurn.into())
            } else if state.board_state().player_port(player).is_some() {
                // Rejoined game, already placed port
                (tile_hand_entities, gameplay::WaitPlaceTokens.into())
            } else {
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
                let token_entity = self.world.create_entity()
                    .with(Transform::new(Pt2::origin()))
                    .with(Model::new(
                        &render::parse_svg(&render::render_token(player, state.num_players(), &mut self.id_counter)),
                        Model::ORDER_PLAYER_TOKEN, 
                        &Self::svg_root(), &mut self.id_counter
                    ))
                    .with(TokenToPlace)
                    .build();
                    
                (tile_hand_entities, gameplay::PlaceToken{ start_ports, token_entity }.into())
            }

        } else {
            (vec![], gameplay::WaitTurn.into())
        };

        let num_players = state.num_players();
        let ports = (0..num_players)
            .map(|player| state.board_state().player_port(player))
            .collect_vec();
        let tiles = state.board_state().tiles_vec();

        let mut game_state = app::Game {
            game,
            state,
            player_usernames: usernames,
            board_entity,
            token_entities: vec![None; num_players as usize],
            tile_hand_entities, 
            board_tile_entities: vec![],
            gameplay_state: Some(gameplay_state),
        };

        game_state.display_state(self);

        // For spectators: add ports and tiles that have already been placed
        for (player, port) in ports.into_iter().enumerate() {
            if let Some(port) = port {
                game_state.move_token(self, player as u32, &port);
            }
        }
        for (loc, tile) in tiles {
            game_state.place_tile(self, &tile, &loc);
        }

        game_state
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

        if let Response::Usernames{ names } = &response {
            let names_str = names.iter().cloned().join("\n\n");
            document().get_element_by_id("usernames").unwrap().set_inner_html(&names_str);
        }

        self.state = Some(self.state.take()
            .expect("State is missing")
            .handle_response(self, response, &mut requests));

        requests
    }
}