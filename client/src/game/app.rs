use common::message::Request;
use specs::prelude::*;
use enum_dispatch::enum_dispatch;
use common::game::BaseGame;
use common::game_state::BaseVisibleGameState;

use super::GameWorld;
use gameplay::GameplayStateT;

#[derive(Debug)]
pub struct EnterUsername;

#[derive(Debug)]
pub struct Game {
    pub(crate) game: BaseGame,
    pub(crate) state: BaseVisibleGameState,
    pub(crate) board_entity: Entity,
    /// An token entity for each player.
    /// None if the player didn't place their token yet
    pub(crate) token_entities: Vec<Option<Entity>>,
    /// None if this is being edited
    pub(crate) gameplay_state: Option<gameplay::State>,
}

#[enum_dispatch]
pub trait AppStateT {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState;
}

impl AppStateT for EnterUsername {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.into()
    }
}

impl AppStateT for Game {
    fn update(mut self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.gameplay_state = Some(self.gameplay_state.take()
            .expect("Missing gameplay state")
            .update(&mut self, world, requests));
        self.into()
    }
}

#[enum_dispatch(AppStateT)]
#[derive(Debug)]
pub enum AppState {
    EnterUsername,
    Game,
}

pub type State = AppState;

pub mod gameplay {
    use specs::{Entity, WorldExt};
    use enum_dispatch::enum_dispatch;
    use common::{game_state::GenericVisibleGameState, message::Request};

    use crate::{game::{GameWorld, app}, render::TokenToPlace};

    #[derive(Debug)]
    pub struct PlaceToken {
        pub(crate) start_ports: Vec<Entity>,
        /// The port that belongs to this player
        pub(crate) token_entity: Entity,
    }

    #[derive(Debug)]
    pub struct WaitPlaceTokens;

    #[enum_dispatch]
    pub trait GameplayStateT {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState;
    }

    impl GameplayStateT for PlaceToken {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            if let Ok(port) = world.port_receiver.try_recv() {
                app.token_entities[app.state.index() as usize] = Some(self.token_entity);
                // The token has been placed; remove the PlaceToken component
                world.world.write_component::<TokenToPlace>()
                    .remove(self.token_entity);

                world.world.delete_entities(&self.start_ports).expect("Entity was deleted too early");
                requests.push(Request::PlaceToken { player: app.state.index(), port });
                WaitPlaceTokens.into()
            } else {
                PlaceToken{
                    start_ports: self.start_ports,
                    token_entity: self.token_entity
                }.into()
            }
        }
    }

    impl GameplayStateT for WaitPlaceTokens {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }
    }

    #[enum_dispatch(GameplayStateT)]
    #[derive(Debug)]
    pub enum GameplayState {
        PlaceToken,
        WaitPlaceTokens,
    }

    // Workaround for enum_dispatch bug
    // where enum_dispatch can't handle two enums being called State
    pub type State = GameplayState;
}