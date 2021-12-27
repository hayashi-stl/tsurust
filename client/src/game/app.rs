use common::{board::{BasePort, BaseTLoc}, game_state::BaseGameState, message::{Request, Response}, player_state::{Looker}, tile::{BaseGAct, BaseKind, BaseTile}, game::GameId, GameInstance, math::Pt2};
use format_xml::{spaced, xml};
use itertools::{Itertools, chain};
use specs::prelude::*;
use enum_dispatch::enum_dispatch;
use common::game::BaseGame;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlTemplateElement};

use crate::{SVG_NS, console_log, document, ecs::{Model, TileSelect, Transform, Collider, TokenSlot, PortLabel, TokenToPlace, RunSelectGameSystem, SelectedGame}, render::{self, BaseBoardExt, BaseTileExt, TOKEN_RADIUS, BaseGameExt, ScreenState}, window};

use super::GameWorld;
use gameplay::GameplayStateT;

/// Initial state. Must enter a username.
#[derive(Debug, Default)]
pub struct EnterUsername {
    usernames: Vec<String>,
}

/// User is in the lobby
#[derive(Debug)]
pub struct Lobby {
    game_entities: Vec<(GameId, Entity)>,
}

/// User is waiting to join a game.
/// This happens between sending the request and receiving the response.
#[derive(Debug)]
pub struct WaitJoinGame {
    id: GameId,
    game_entities: Vec<(GameId, Entity)>,
}

/// User is in a game that hasn't started yet
#[derive(Debug)]
pub struct StatelessGame {
    id: GameId,
    game: BaseGame,
    player_usernames: Vec<String>,
    board_entity: Entity,
}

/// User is in a game that started.
#[derive(Debug)]
pub struct Game {
    pub(crate) id: GameId,
    pub(crate) game: BaseGame,
    pub(crate) state: BaseGameState,
    pub(crate) player_usernames: Vec<String>,
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

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        match response {
            Response::JoinedLobby{ games } => {
                Lobby::new(games, world).into()
            }

            Response::RejectedUsername => {
                let username = window().prompt_with_message("Enter a username. The one you entered is already taken.")
                    .unwrap_or(None)
                    .unwrap_or("Guest".to_owned());
                render::set_username(&username);
                requests.push(Request::SetUsername{ username });
                self.into()
            }

            _ => self.into()
        }
    }
}

impl AppStateT for Lobby {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        world.world.get_mut::<RunSelectGameSystem>().unwrap().0 = true;
        if let Some(id) = world.world.get_mut::<SelectedGame>().unwrap().0.take() {
            world.world.get_mut::<RunSelectGameSystem>().unwrap().0 = false;
            requests.push(Request::JoinGame{ id });
            WaitJoinGame{ id, game_entities: self.game_entities }.into()
        } else {
            self.into()
        }
    }

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        match response {
            Response::ChangedGame { game } => {
                match self.game_entities.binary_search_by_key(&game.id(), |(id, _)| *id) {
                    Ok(pos) => {
                        world.world.delete_entity(self.game_entities[pos].1).ok();
                        self.game_entities[pos].1 = render::game_entity(game, &mut world.world, &mut world.id_counter);
                    }

                    Err(pos) => {
                        let id = game.id();
                        let entity = render::game_entity(game, &mut world.world, &mut world.id_counter);
                        self.game_entities.insert(pos, (id, entity));
                    }
                }
                self.into()
            }

            _ => self.into()
        }
    }
}

impl Lobby {
    fn new(games: Vec<GameInstance>, world: &mut GameWorld) -> Self {
        render::set_screen_state(ScreenState::Lobby);
        Self {
            game_entities: games.into_iter().map(|game| (
                game.id(), render::game_entity(game, &mut world.world, &mut world.id_counter)
            )).collect()
        }
    }
}

impl AppStateT for WaitJoinGame {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.into()
    }

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        match response {
            Response::JoinedGame { game } => {
                if self.id == game.id() {
                    self.game_entities.drain(..).for_each(|(_, entity)| {
                        world.world.delete_entity(entity).ok();
                    });
                    Game::app_state(game, world)
                } else { self.into() }
            }

            Response::Rejected{ id } => {
                if self.id == id {
                    Lobby{ game_entities: self.game_entities }.into()
                } else { self.into() }
            }

            _ => self.into()
        }
    }
}

impl AppStateT for StatelessGame {
    fn update(self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        if world.world.read_component::<Collider>().get(world.start_game_entity).unwrap().clicked() {
            requests.push(Request::StartGame{ id: self.id });
        } else if world.world.read_component::<Collider>().get(world.leave_game_entity).unwrap().clicked() {
            requests.push(Request::JoinLobby);
        }
        self.into()
    }

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) ->AppState {
        match response {
            Response::ChangedPlayers{ id, names } => {
                if id == self.id {
                    let names_str = names.iter()
                        .map(|name| html_escape::encode_text(name))
                        .join("<br>");
                    document().get_element_by_id("usernames").unwrap().set_inner_html(&names_str);
                    self.player_usernames = names;
                }
                self.into()
            }

            Response::JoinedLobby{ games } => {
                world.world.delete_entity(self.board_entity).ok();
                Lobby::new(games, world).into()
            }

            Response::StartedGame{ id, state } => {
                if id == self.id {
                    self.with_state(state, world).into()
                } else {
                    self.into()
                }
            }

            _ => self.into()
        }
    }
}

impl StatelessGame {
    fn new(id: GameId, game: BaseGame, players: Vec<String>, world: &mut GameWorld) -> Self {
        render::set_screen_state(ScreenState::StatelessGame);
        let board_svg = render::parse_svg(&game.board().render());
        let board_entity = world.world.create_entity()
            .with(Model::new(&board_svg, Model::ORDER_BOARD, &GameWorld::svg_root(), &mut world.id_counter))
            .build();

        Self { id, game, player_usernames: players, board_entity }
    }

    fn with_state(self, state: BaseGameState, world: &mut GameWorld) -> Game {
        render::set_screen_state(ScreenState::Game);
        let StatelessGame{ id, game, player_usernames, board_entity } = self;

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
                    &mut world.world,
                    &mut world.id_counter,
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
                        world.world.create_entity()
                            .with(Transform::new(position))
                            .with(Model::new(
                                &svg,
                                Collider::ORDER_START_PORT,
                                &GameWorld::svg_root(),
                                &mut world.id_counter
                            ))
                            .with(Collider::new(&svg))
                            .with(TokenSlot)
                            .with(PortLabel(port))
                            .build()
                    })
                    .collect_vec();
                let token_entity = world.world.create_entity()
                    .with(Transform::new(Pt2::origin()))
                    .with(Model::new(
                        &render::parse_svg(&render::render_token(player, state.num_players(), &mut world.id_counter)),
                        Model::ORDER_PLAYER_TOKEN, 
                        &GameWorld::svg_root(), &mut world.id_counter
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

        let mut game_state = Game {
            id,
            game,
            state,
            player_usernames,
            board_entity,
            token_entities: vec![None; num_players as usize],
            tile_hand_entities, 
            board_tile_entities: vec![],
            gameplay_state: Some(gameplay_state),
        };

        game_state.display_state(world);

        // For spectators: add ports and tiles that have already been placed
        for (player, port) in ports.into_iter().enumerate() {
            if let Some(port) = port {
                game_state.move_token(world, player as u32, &port);
            }
        }
        for (loc, tile) in tiles {
            game_state.place_tile(world, &tile, &loc);
        }

        game_state
    }
}

impl AppStateT for Game {
    fn update(mut self, world: &mut GameWorld, requests: &mut Vec<Request>) -> AppState {
        self.gameplay_state = Some(self.gameplay_state.take()
            .expect("Missing gameplay state")
            .update(&mut self, world, requests));
        if world.world.read_component::<Collider>().get(world.leave_game_entity).unwrap().clicked() {
            requests.push(Request::JoinLobby);
        }
        self.into()
    }

    fn handle_response(mut self, world: &mut GameWorld, response: Response, requests: &mut Vec<Request>) -> AppState {
        if let Response::JoinedLobby{ games } = response {
            let to_delete = chain!(
                [self.board_entity],
                self.token_entities.drain(..).flatten(),
                self.tile_hand_entities.drain(..),
                self.board_tile_entities.drain(..),
            ).collect_vec();

            world.world.delete_entities(&to_delete).ok();
            return Lobby::new(games, world).into();
        }

        match &response {
            Response::PlacedToken{ id, player, port } => if *id == self.id {
                self.set_token_position(world, *player, port)
            },

            Response::PlacedTile{ id, player, kind, index, action, loc } => if *id == self.id {
                self.take_turn_placing_tile(world, *player, kind, *index, action, loc)
            }

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
    /// Returns either an `StatelessGame` or a `Game` depending on whether the game has started.
    fn app_state(game: GameInstance, world: &mut GameWorld) -> AppState {
        let (id, game, state, players) = game.into_fields();
        let stateless = StatelessGame::new(id, game, players, world);
        if let Some(state) = state {
            stateless.with_state(state, world).into()
        } else {
            stateless.into()
        }
    }

    /// Moves a player token to some location.
    /// This does not care about `self.gameplay_state` and can be called with it being `None`.
    pub fn move_token(&mut self, world: &mut GameWorld, player: u32, port: &BasePort) {
        let position = self.game.board().port_position(port);

        if let Some(token) = self.token_entities[player as usize] {
            world.world.write_component::<Transform>()
                .get_mut(token)
                .expect("Expected token to exist since its ID is stored")
                .position = position;
        } else {
            self.token_entities[player as usize] = Some(world.world.create_entity()
                .with(Transform::new(position))
                .with(Model::new(
                    &render::parse_svg(&render::render_token(player, self.state.num_players(), &mut world.id_counter)),
                    Model::ORDER_PLAYER_TOKEN, 
                    &GameWorld::svg_root(), &mut world.id_counter
                ))
                .build());
        }
    }

    /// Set the position of some player's token, editing the state.
    /// This does not care about `self.gameplay_state` and can be called with it being `None`.
    pub fn set_token_position(&mut self, world: &mut GameWorld, player: u32, port: &BasePort) {
        self.state.place_player(player, port);
        self.display_state(world);
        self.move_token(world, player, port);
    }

    /// Renders a tile at some location.
    /// This does not care about `self.gameplay_state` and can be called with it being `None`.
    pub fn place_tile(&mut self, world: &mut GameWorld, tile: &BaseTile, loc: &BaseTLoc) {
        let board_tile_entity = tile.create_on_board_entity(
            &self.game.board(),
            &loc,
            &mut world.world,
            &mut world.id_counter,
        );
        self.board_tile_entities.push(board_tile_entity);
    }

    pub fn take_turn_placing_tile(&mut self, world: &mut GameWorld, player: u32, kind: &BaseKind, index: u32, action: &BaseGAct, loc: &BaseTLoc) {
        let delta = self.state.take_turn_placing_tile(&self.game, kind, index, action, loc);
        self.display_state(world);

        self.place_tile(world, &delta.tile_placed().1, loc);

        for (player, port) in delta.player_ports().iter().enumerate() {
            self.set_token_position(world, player as u32, port);
        }

        if let Looker::Player(looker) = self.state.looker() {
            // Wipe tiles if dead
            if delta.dead_players().contains(&looker) {
                world.world.delete_entities(&self.tile_hand_entities).expect("Entities deleted too early");
                self.tile_hand_entities.clear();
            }

            // Delete placed tile if necessary
            else if delta.tile_placer() == looker {
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

            // Add new tiles
            for (player, index, tile) in delta.drawn_tiles() {
                if *player == looker {
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

    fn display_player_state(&mut self, world: &mut GameWorld, player: u32, html_string: &mut String) {
        let token = render::render_token(player, self.state.num_players(), &mut world.id_counter);
        let tile_svgs = self.state.player_state(player)
            .map(|state| state.tiles_vec())
            .into_iter()
            .flat_map(|tiles| tiles.into_iter().flat_map(|(_, tiles)| tiles))
            .map(|tile| render::wrap_svg(&tile.render(), "state-tile"))
            .collect::<String>();

        let dead = self.state.player_state(player).is_none();
        let won = self.state.won(player);
        let turn = self.state.turn_player() == player;
        let state_string = xml! {
            <div class="state">
                <div class="state-top">
                    <div class="state-token">
                        <svg xmlns={SVG_NS} viewBox={spaced!(-TOKEN_RADIUS, -TOKEN_RADIUS, TOKEN_RADIUS * 2.0, TOKEN_RADIUS * 2.0)}
                        width="20" height="20">{token}</svg>
                    </div>
                    <div class=("state-username"{if dead {"-dead"} else {""}})>{
                        html_escape::encode_text(&self.player_usernames[player as usize])
                    }</div>
                    if (won) { <div class="state-winner">"WIN"</div> }
                    if (turn && !self.state.game_over()) { <div class="state-winner">"TURN"</div> }
                </div>
                <div class="state-tiles">{tile_svgs}</div>
                <div class="state-separator"></div>
            </div>
        }.to_string();
        html_string.push_str(&state_string);
    }

    /// Displays the state of the game in the state panel.
    pub fn display_state(&mut self, world: &mut GameWorld) {
        let state_panel = document().get_element_by_id("state_panel").expect("Missing state panel");

        let mut html_string = String::new();

        for player in 0..self.state.num_players() {
            self.display_player_state(world, player, &mut html_string);
        }

        let draw_pile_svgs = self.state.num_tiles_left_by_kind().into_iter()
            .filter(|(_, num_tiles)| *num_tiles > 0)
            .map(|(kind, num_tiles)| {
                let representative = self.state.top_tile_left_of_kind(&kind)
                    .expect("Must have at least 1 tile in the pile");

                let tile_svg = render::wrap_svg(&representative.render(), "state-draw-tile");
                xml!(
                    <div class="state-draw-pile">
                        {tile_svg}
                        <div class="state-draw-count">{num_tiles}</div>
                    </div>
                ).to_string()
            })
            .collect::<String>();

        html_string += &xml! {
            <div class="state-draw-piles">{draw_pile_svgs}</div>
        }.to_string();

        state_panel.set_inner_html(&html_string);
        state_panel.remove_attribute("style").expect("Failed to show state panel"); // remove the hiding attribute
    }
}

#[enum_dispatch(AppStateT)]
#[derive(Debug)]
pub enum AppState {
    EnterUsername,
    Lobby,
    WaitJoinGame,
    StatelessGame,
    Game,
}

pub type State = AppState;

pub mod gameplay {
    use specs::{Entity, WorldExt};
    use enum_dispatch::enum_dispatch;
    use common::{math::Pt2, message::{Request, Response}, tile::BaseGAct};

    use crate::{console_log, ecs::{PlacedPort, PlacedTLoc, RunPlaceTileSystem, RunPlaceTokenSystem, SelectedTile, TileLabel, TokenToPlace, Transform}, game::{GameWorld, app}, render::{BaseBoardExt, BaseTileExt}};

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
                requests.push(Request::PlaceToken { id: app.id, player: app.state.player_expect(), port });
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
                Response::PlacedToken { id, player, port } => if id == app.id && player == app.state.player_expect() {
                    world.world.delete_entity(self.token_entity).expect("Entity was deleted too early");
                    world.world.delete_entities(&self.start_ports).expect("Entity was deleted too early");
                    WaitPlaceTokens.into()
                } else {
                    self.into()
                },

                Response::Rejected { id } => if id == app.id {
                    PlaceToken { start_ports: self.start_ports, token_entity: self.token_entity }.into()
                } else {
                    self.into()
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
            if let Response::AllPlacedTokens { id } = response {
                if id == app.id {
                    WaitTurn.into()
                } else {
                    self.into()
                }
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
            if let Response::YourTurn { id } = response {
                if id == app.id {
                    let port = app.state.board_state().player_port(app.state.player_expect()).expect("Port should be placed");
                    let locs = app.game.board().port_locs(&port).into_iter().map(|loc| {
                        app.game.board().create_loc_collider_entity(&loc, &mut world.world, &mut world.id_counter)
                    }).collect();

                    PlaceTile {
                        locs,
                        tile_entity: None,
                        tile_index: 0,
                        tile_action: None,
                    }.into()
                } else { self.into() }
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
                    // Recover transform to apply it to the new tile
                    let transform = self.tile_entity.and_then(|entity| {
                        let transform = world.world.read_component::<Transform>()
                            .get(entity)
                            .cloned();
                        world.world.delete_entity(entity).ok();
                        transform
                    }).unwrap_or(Transform::new(Pt2::origin()));

                    if let Some(tile) = tile {
                        self.tile_entity = Some(tile.create_to_place_entity(
                            &self.tile_action.clone().expect("Group action should exist"),
                            transform,
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
                    id: app.id,
                    player: app.state.player_expect(),
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
                Response::PlacedTile{ id, player, .. } => if id == app.id && player == app.state.player_expect() {
                    self.tile_entity.map(|e| world.world.delete_entity(e).expect("Entity was deleted too early"));
                    world.world.delete_entities(&self.locs).expect("Entity was deleted too early");
                    world.world.get_mut::<SelectedTile>().expect("Missing SelectedTile").2 = None;
                    WaitTurn.into()
                } else {
                    self.into()
                },

                Response::Rejected{ id } => if id == app.id {
                    PlaceTile {
                        locs: self.locs,
                        tile_entity: self.tile_entity,
                        tile_index: self.tile_index,
                        tile_action: self.tile_action,
                    }.into()
                } else { self.into() },

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