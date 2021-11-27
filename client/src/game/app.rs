use common::{board::{BasePort, BaseTLoc}, game_state::BaseGameState, message::{Request, Response}, tile::{BaseGAct, BaseKind}};
use specs::prelude::*;
use enum_dispatch::enum_dispatch;
use common::game::BaseGame;

use crate::{console_log, render::{self, BaseBoardExt, BaseTileExt}, ecs::{Model, TileSelect, Transform}};

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
    /// Tiles on the board
    pub(crate) board_tile_entities: Vec<Entity>,
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
        match &response {
            Response::PlacedToken{ player, port } =>
                self.set_token_position(world, *player, port),

            Response::PlacedTile{ player, kind, index, action, loc } =>
                self.take_turn_placing_tile(world, *player, kind, *index, action, loc),

            _ => {}
        }
        // and let the gameplay state handle it too

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

    pub fn take_turn_placing_tile(&mut self, world: &mut GameWorld, player: u32, kind: &BaseKind, index: u32, action: &BaseGAct, loc: &BaseTLoc) {
        let delta = self.state.take_turn_placing_tile(&self.game, kind, index, action, loc);

        let board_tile_entity = delta.tile_placed().1.create_on_board_entity(
            &self.game.board(),
            &delta.tile_loc(),
            &mut world.world,
            &mut world.id_counter,
        );
        self.board_tile_entities.push(board_tile_entity);

        for (player, port) in delta.player_ports().iter().enumerate() {
            self.set_token_position(world, player as u32, port);
        }

        if delta.dead_players().contains(&self.state.looker_expect()) {
            world.world.delete_entities(&self.tile_hand_entities).expect("Entities deleted too early");
            self.tile_hand_entities.clear();
        }

        // Delete placed tile if necessary
        else if delta.tile_placer() == self.state.looker_expect() {
            let storage = world.world.read_component::<TileSelect>();
            let (i, kind, index, entity) = self.tile_hand_entities.iter()
                .enumerate()
                .find_map(|(i, entity)| {
                    let tile_select = storage.get(*entity).expect("Hand tile is missing TileSelect");
                    (tile_select.index() == delta.tile_placed().0 && tile_select.kind() == &delta.tile_placed().1.kind())
                        .then(|| (i, tile_select.kind().clone(), tile_select.index(), *entity))
                }).expect("Placed tile not in your hand");
            std::mem::drop(storage);

            world.world.delete_entity(entity).expect("Entity deleted too early");
            self.tile_hand_entities.remove(i);

            // Shift indexes
            let mut storage = world.world.write_component::<TileSelect>();
            for entity in &self.tile_hand_entities {
                let tile_select = storage.get_mut(*entity).expect("Hand tile is missing TileSelect");
                if tile_select.kind() == &kind && tile_select.index() > index {
                    *tile_select.index_mut() -= 1;
                }
            }
        }

        for (player, index, tile) in delta.drawn_tiles() {
            if *player == self.state.looker_expect() {
                let entity = tile.create_hand_entity(
                    *index, 
                    &tile.identity_action(),
                    &mut world.world, 
                    &mut world.id_counter
                );
                self.tile_hand_entities.push(entity);
            }
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
    use common::{message::{Request, Response}, tile::BaseGAct};

    use crate::{
        console_log,
        game::{GameWorld, app},
        render::{BaseBoardExt, BaseTileExt},
        ecs::{PlacedPort, PlacedTLoc, RunPlaceTileSystem, RunPlaceTokenSystem, SelectedTile, TileLabel, TokenToPlace}
    };

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
    pub struct PlaceTile {
        pub(crate) locs: Vec<Entity>,
        pub(crate) tile_entity: Option<Entity>,
        pub(crate) tile_index: u32,
        pub(crate) tile_action: Option<BaseGAct>,
    }

    /// Waiting for the server to check the validity of the tile placement
    #[derive(Debug)]
    pub struct WaitPlaceTileCheck {
        pub(crate) locs: Vec<Entity>,
        pub(crate) tile_entity: Option<Entity>,
        pub(crate) tile_index: u32,
        pub(crate) tile_action: Option<BaseGAct>,
    }

    #[enum_dispatch]
    pub trait GameplayStateT {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState;

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState;
    }

    impl GameplayStateT for PlaceToken {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            world.world.get_mut::<RunPlaceTokenSystem>().expect("Missing RunPlaceTokenSystem").0 = true;

            if let Some(port) = world.world.get_mut::<PlacedPort>().expect("Missing PlacedPort").0.take() {
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
                let port = app.state.board_state().player_port(app.state.looker_expect()).expect("Port should be placed");
                let locs = app.game.board().port_locs(&port).into_iter().map(|loc| {
                    app.game.board().create_loc_collider_entity(&loc, &mut world.world, &mut world.id_counter)
                }).collect();

                PlaceTile {
                    locs,
                    tile_entity: None,
                    tile_index: 0,
                    tile_action: None,
                }.into()
            } else {
                self.into()
            }
        }
    }

    impl GameplayStateT for PlaceTile {
        fn update(mut self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            // Tile selection
            {
                let selected_tile = world.world.fetch::<SelectedTile>();
                let storage = world.world.read_component::<TileLabel>();
                let tile_label = self.tile_entity.map(|entity| 
                    &storage.get(entity).expect("Tile entity should have TileLabel").0
                );

                self.tile_index = selected_tile.0;
                if selected_tile.2.as_ref() != tile_label || selected_tile.1.as_ref() != self.tile_action.as_ref() {
                    self.tile_action = selected_tile.1.clone();

                    // Replace tile to place
                    let tile = selected_tile.2.clone();
                    std::mem::drop((selected_tile, storage));
                    self.tile_entity.map(|entity| world.world.delete_entity(entity).ok());
                    if let Some(tile) = tile {
                        self.tile_entity = Some(tile.create_to_place_entity(
                            &self.tile_action.clone().expect("Group action should exist"),
                            &mut world.world,
                            &mut world.id_counter,
                        ));
                    }
                }
            }

            // Tile placement
            world.world.get_mut::<RunPlaceTileSystem>().expect("Missing RunPlaceTileSystem").0 = true;
            if let (Some(loc), Some(tile_entity)) = (
                world.world.get_mut::<PlacedTLoc>().expect("Missing PlacedTLoc").0.take(),
                self.tile_entity
            ) {
                // Suspend while waiting for the check
                world.world.get_mut::<RunPlaceTileSystem>().expect("Missing RunPlaceTileSystem").0 = false;
                let kind = world.world.read_component::<TileLabel>().get(tile_entity)
                    .expect("Tile is missing label").0.kind();
                requests.push(Request::PlaceTile {
                    player: app.state.looker_expect(),
                    kind,
                    index: self.tile_index,
                    action: self.tile_action.clone().expect("Group action should exist"),
                    loc
                });

                WaitPlaceTileCheck {
                    locs: self.locs,
                    tile_entity: self.tile_entity,
                    tile_index: self.tile_index,
                    tile_action: self.tile_action,
                }.into()
            } else {
                self.into()
            }
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }
    }

    impl GameplayStateT for WaitPlaceTileCheck {
        fn update(self, app: &mut app::Game, world: &mut GameWorld, requests: &mut Vec<Request>) -> GameplayState {
            self.into()
        }

        fn handle_response(self, app: &mut app::Game, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> GameplayState {
            match response {
                Response::PlacedTile{ player, .. } => if player == app.state.looker_expect() {
                    self.tile_entity.map(|e| world.world.delete_entity(e).expect("Entity was deleted too early"));
                    world.world.delete_entities(&self.locs).expect("Entity was deleted too early");
                    world.world.get_mut::<SelectedTile>().expect("Missing SelectedTile").2 = None;
                    WaitTurn.into()
                } else {
                    self.into()
                },

                Response::Rejected => {
                    PlaceTile {
                        locs: self.locs,
                        tile_entity: self.tile_entity,
                        tile_index: self.tile_index,
                        tile_action: self.tile_action,
                    }.into()
                },

                _ => self.into()
            }
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
        WaitPlaceTileCheck,
    }

    // Workaround for enum_dispatch bug
    // where enum_dispatch can't handle two enums being called State
    pub type State = GameplayState;
}