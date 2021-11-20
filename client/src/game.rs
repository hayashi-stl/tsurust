use common::{game::{BaseGame, GenericGame}, game_state::BaseVisibleGameState};
use itertools::Itertools;
use specs::{Builder, Dispatcher, DispatcherBuilder, Entity, World, WorldExt};
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

use crate::{console_log, render::{BaseBoardExt, BaseGameExt, Collider, ColliderInputSystem, Model, SvgOrderSystem}};

pub enum GameplayState {
    PlaceToken {
        start_ports: Vec<Entity>,
    }
}

/// State of the entire app
pub enum AppState {
    EnterUsername,
    Game {
        game: BaseGame,
        state: BaseVisibleGameState,
        board_entity: Entity,
        gameplay_state: GameplayState,
    }
}

/// The game and state, including components such as collision and rendering
pub struct GameWorld {
    /// None if the state is being edited
    state: Option<AppState>,
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

        let dispatcher = DispatcherBuilder::new()
            .with(SvgOrderSystem, "svg_order", &[])
            .with(ColliderInputSystem, "collider_input", &[])
            .build();

        Self {
            state: Some(AppState::EnterUsername),
            world,
            id_counter: 0,
            dispatcher,
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
        let start_ports = game.start_port_colliders().into_iter()
            .map(|svg| {
                self.world.create_entity()
                    .with(Collider::new(
                        &svg,
                        Collider::ORDER_START_PORT,
                        &Self::svg_root(),
                        &mut self.id_counter
                    ))
                    .build()
            })
            .collect_vec();
        let board_entity = self.world.create_entity()
            .with(Model::new(&board_svg, Model::ORDER_BOARD, &Self::svg_root(), &mut self.id_counter))
            .build();


        self.state = Some(AppState::Game {
            game,
            state,
            board_entity,
            gameplay_state: GameplayState::PlaceToken{ start_ports },
        });
    }

    pub fn update(&mut self) {
        self.dispatcher.dispatch(&mut self.world);

        self.state = match self.state.take().expect("State is missing") {
            AppState::EnterUsername => Some(AppState::EnterUsername),

            AppState::Game{ game, state, board_entity, gameplay_state } => {
                let gameplay_state = match gameplay_state {
                    GameplayState::PlaceToken{ start_ports } => {
                        GameplayState::PlaceToken{ start_ports }
                    }
                };
                Some(AppState::Game{ game, state, board_entity, gameplay_state })
            }
        }
    }
}