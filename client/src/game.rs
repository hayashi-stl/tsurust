use std::sync::mpsc::{self, Receiver};

use common::{board::BasePort, board_state::GenericBoardState, game::{BaseGame, GenericGame}, game_state::{BaseVisibleGameState, GenericVisibleGameState}, math::{Pt2, Vec2}, message::Request};
use itertools::Itertools;
use specs::{Builder, Dispatcher, DispatcherBuilder, Entity, World, WorldExt};
use wasm_bindgen::JsCast;
use web_sys::SvgElement;
use enum_dispatch::enum_dispatch;

use crate::{console_log, document, render::{self, BaseBoardExt, BaseGameExt, BoardInput, Collider, ColliderInputSystem, PlaceTokenSystem, PortLabel, SvgModel, SvgOrderSystem, TokenSlot, TokenToPlace, Transform, TransformSystem}};

mod app;
use app::{gameplay, AppStateT};

/// The game and state, including components such as collision and rendering
pub struct GameWorld {
    /// None if the state is being edited
    state: Option<app::State>,
    world: World,
    id_counter: u64,
    dispatcher: Dispatcher<'static, 'static>,
    port_receiver: Receiver<BasePort>,
}

impl GameWorld {
    /// Constructs a game world
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<SvgModel>();
        world.register::<Collider>();
        world.register::<TokenSlot>();
        world.register::<TokenToPlace>();
        world.register::<Transform>();
        world.register::<PortLabel>();
        world.insert(BoardInput::new(&document().get_element_by_id("svg_root").expect("Missing main panel svg")
            .dyn_into().expect("Not an <svg> element")));

        let (port_sender, port_receiver) = mpsc::channel();
        let dispatcher = DispatcherBuilder::new()
            .with(SvgOrderSystem, "svg_order", &[])
            .with(ColliderInputSystem, "collider_input", &[])
            .with(PlaceTokenSystem::new(port_sender), "place_token", &["collider_input"])
            .with(TransformSystem::new(&world), "transform", &["place_token"])
            .build();

        Self {
            state: Some(app::EnterUsername.into()),
            world,
            id_counter: 0,
            dispatcher,
            port_receiver,
        }
    }

    fn svg_root() -> SvgElement {
        web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("svg_root").unwrap()
            .dyn_into().unwrap()
    }

    /// Constructs a game world from a game and state
    pub fn set_game(&mut self, game: BaseGame, state: BaseVisibleGameState) {
        let board_svg = game.board().render();
        let start_ports = game.start_ports_and_positions().into_iter()
            .map(|(port, position)| {
                let svg = render::render_port_collider();
                self.world.create_entity()
                    .with(Transform::new(position))
                    .with(SvgModel::new(
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
            .with(SvgModel::new(&board_svg, SvgModel::ORDER_BOARD, &Self::svg_root(), &mut self.id_counter))
            .build();
        let token_entity = self.world.create_entity()
            .with(Transform::new(Pt2::origin()))
            .with(SvgModel::new(
                &render::render_token(state.index(), state.num_players(), &mut self.id_counter),
                SvgModel::ORDER_PLAYER_TOKEN, 
                &Self::svg_root(), &mut self.id_counter
            ))
            .with(TokenToPlace)
            .build();

        let num_players = state.num_players();
        let ports = (0..num_players)
            .map(|player| state.board_state().player_port(player))
            .collect_vec();

        self.state = Some(app::Game {
            game,
            state,
            board_entity,
            token_entities: vec![None; num_players as usize],
            gameplay_state: Some(gameplay::PlaceToken{ start_ports, token_entity }.into()),
        }.into());

        // For spectators: add ports that have already been placed
        for (player, port) in ports.into_iter().enumerate() {
            if let Some(port) = port {
                self.set_token_position(player as u32, &port);
            }
        }
    }

    pub fn update(&mut self) -> Vec<Request> {
        self.dispatcher.dispatch(&mut self.world);

        let mut requests = vec![];

        self.state = Some(self.state.take()
            .expect("State is missing")
            .update(self, &mut requests));

        requests
    }

    /// Set the position of some player's token.
    pub fn set_token_position(&mut self, player: u32, port: &BasePort) {
        if let app::State::Game(game) = self.state.as_mut().unwrap() {
            let game: &mut app::Game = game;
            let position = game.game.board().port_position(port);

            if let Some(token) = game.token_entities[player as usize] {
                self.world.write_component::<Transform>()
                    .get_mut(token)
                    .expect("Expected token to exist since its ID is stored")
                    .position = position;
            } else {
                game.token_entities[player as usize] = Some(self.world.create_entity()
                    .with(Transform::new(position))
                    .with(SvgModel::new(
                        &render::render_token(player, game.state.num_players(), &mut self.id_counter),
                        SvgModel::ORDER_PLAYER_TOKEN, 
                        &Self::svg_root(), &mut self.id_counter
                    ))
                    .build());
            }
        }
    }
}