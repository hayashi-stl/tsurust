use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::game::BaseGame;
use crate::game_state::BaseGameState;
use crate::board::{BasePort, BaseTLoc};
use crate::tile::BaseKind;

/// The request type used by the client to communicate to the server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    /// Set the username for a player
    SetUsername{ name: String },
    /// Starts the game
    StartGame,
    PlaceToken{ player: u32, port: BasePort },
    PlaceTile{ player: u32, kind: BaseKind, index: u32, loc: BaseTLoc },
    RemovePeer{ addr: SocketAddr },
}

/// The response type used by the server to communicate to the client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    /// Responds with the index of the player
    PlayerIndex{ index: u32 },
    /// Responds with the usernames of all players, in order of index
    Usernames{ names: Vec<String> },
    /// Responds with the game's state
    State{ game: BaseGame, state: BaseGameState },
    /// Player `player` has placed a token on port `port`.
    PlacedToken{ player: u32, port: BasePort },
    /// Invalid move, please undo
    Rejected,
}