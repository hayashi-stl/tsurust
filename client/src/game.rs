use common::{board::AllBoardRenderer, game::{BaseGame, GenericGame}, game_state::BaseVisibleGameState};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

use crate::{console_log, render::{BoardRenderer, Model, SpecificBoardRenderer}};

/// The game and state, including components such as collision and rendering
pub struct GameWorld {
    game: Option<BaseGame>,
    state: Option<BaseVisibleGameState>,
    world: World,
    board_entity: Option<Entity>,
    id_counter: u64,
}

impl GameWorld {
    /// Constructs a game world
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<Model>();

        Self {
            game: None,
            state: None,
            world,
            board_entity: None,
            id_counter: 0,
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
        let board_svg = BoardRenderer.render(&game.board());
        let board_entity = self.world.create_entity()
            .with(Model::new(&board_svg, &Self::svg_root(), &mut self.id_counter))
            .build();

        self.game = Some(game);
        self.state = Some(state);
        self.board_entity = Some(board_entity);
    }
}