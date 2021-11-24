use common::{board::BasePort, game_state::BaseGameState, message::{Request, Response}};
use specs::prelude::*;
use enum_dispatch::enum_dispatch;
use common::game::BaseGame;

use crate::render::{self, BaseBoardExt, Model, Transform};

use super::GameWorld;
use gameplay::GameplayStateT;

#[derive(Debug)]
pub struct EnterUsername;

#[derive(Debug)]
pub struct Game {
    pub(crate) game: BaseGame,
    pub(crate) state: BaseGameState,
    pub(crate) board_entity: Entity,
    /// An token entity for each player.
    /// None if the player didn't place their token yet
    pub(crate) token_entities: Vec<Option<Entity>>,
    /// Entites for tiles in the player's hand 
    pub(crate) tile_hand_entities: Vec<Entity>,
    /// None if this is being edited
    pub(crate) gameplay_state: Option<gameplay::State>,
}

#[enum_dispatch]
pub trait AppStateT {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState;

    fn handle_response(self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState;
}

impl AppStateT for EnterUsername {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.into()
    }

    fn handle_response(self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        if let Response::State{ game, state } = response {
            world.set_game(game, state).into()
        } else {
            self.into()
        }
    }
}

impl AppStateT for Game {
    fn update(mut self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.gameplay_state = Some(self.gameplay_state.take()
            .expect("Missing gameplay state")
            .update(&mut self, world, requests));
        self.into()
    }

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        if let Response::PlacedToken{ player, port } = &response {
            self.set_token_position(world, *player, port);
        } // and let the gameplay state handle it too

        self.gameplay_state = Some(self.gameplay_state.take()
            .expect("Missing gameplay state")
            .handle_response(&mut self, world, response, requests));
        self.into()
    }
}

impl Game {
    /// Set the position of some player's token.
    /// This does not care about `self.gameplay_state` and can be called with it being `None`.
    pub fn set_token_position(&mut self, world: &mut GameWorld, player: u32, port: &BasePort) {
        let position = self.game.board().port_position(port);
        self.state.place_player(player, port);

        if let Some(token) = self.token_entities[player as usize] {
            world.world.write_component::<Transform>()
                .get_mut(token)
                .expect("Expected token to exist since its ID is stored")
                .position = position;
        } else {
            self.token_entities[player as usize] = Some(world.world.create_entity()
                .with(Transform::new(position))
                .with(Model::new(
                    &render::render_token(player, self.state.num_players(), &mut world.id_counter),
                    Model::ORDER_PLAYER_TOKEN, 
                    &GameWorld::svg_root(), &mut world.id_counter
                ))
                .build());
        }
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
    use common::{message::{Request, Response}};

    use crate::{console_log, game::{GameWorld, app}, render::{RunPlaceTokenSystem, TokenToPlace}};

    #[derive(Debug)]
    pub struct PlaceToken {
        pub(crate) start_ports: Vec<Entity>,
        /// The port that belongs to this player
        pub(crate) token_entity: Entity,
    }

    /// Waiting for the server to check the validity of the token placement
    #[derive(Debug)]
    pub struct WaitPlaceTokenCheck {
        pub(crate) start_ports: Vec<Entity>,
        pub(crate) token_entity: Entity,
    }

    #[derive(Debug)]
    pub struct WaitPlaceTokens;

    #[derive(Debug)]
    pub struct WaitTurn;

    #[derive(Debug)]
    pub struct PlaceTile;

    #[enum_dispatch]
    pub trait GameplayStateT {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState;

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState;
    }

    impl GameplayStateT for PlaceToken {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            world.world.get_mut::<RunPlaceTokenSystem>().expect("Missing RunPlaceTokenSystem").0 = true;

            if let Ok(port) = world.port_receiver.try_recv() {
                requests.push(Request::PlaceToken { player: app.state.looker_expect(), port });
                // Suspend this while waiting for the check
                world.world.get_mut::<RunPlaceTokenSystem>().expect("Missing RunPlaceTokenSystem").0 = false;
                WaitPlaceTokenCheck { start_ports: self.start_ports, token_entity: self.token_entity }.into()
            } else {
                self.into()
            }
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }
    }

    impl GameplayStateT for WaitPlaceTokenCheck {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            match response {
                Response::PlacedToken { player, port } => if player == app.state.looker_expect() {
                    world.world.delete_entity(self.token_entity).expect("Entity was deleted too early");
                    world.world.delete_entities(&self.start_ports).expect("Entity was deleted too early");
                    WaitPlaceTokens.into()
                } else {
                    self.into()
                },

                Response::Rejected => {
                    PlaceToken { start_ports: self.start_ports, token_entity: self.token_entity }.into()
                },

                _ => self.into()
            }
        }
    }

    impl GameplayStateT for WaitPlaceTokens {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            if let Response::AllPlacedTokens = response {
                console_log!("Wait your turn.");
                WaitTurn.into()
            } else {
                self.into()
            }
        }
    }

    impl GameplayStateT for WaitTurn {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            if let Response::YourTurn = response {
                console_log!("Your turn!");
                PlaceTile.into()
            } else {
                self.into()
            }
        }
    }

    impl GameplayStateT for PlaceTile {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }
    }

    #[enum_dispatch(GameplayStateT)]
    #[derive(Debug)]
    pub enum GameplayState {
        PlaceToken,
        WaitPlaceTokenCheck,
        WaitPlaceTokens,
        WaitTurn,
        PlaceTile,
    }

    // Workaround for enum_dispatch bug
    // where enum_dispatch can't handle two enums being called State
    pub type State = GameplayState;
}