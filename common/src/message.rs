use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::GameInstance;
use crate::game::{BaseGame, GameId};
use crate::game_state::BaseGameState;
use crate::board::{BasePort, BaseTLoc};
use crate::tile::{BaseKind, BaseGAct};

/// The request type used by the client to communicate to the server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    /// Set the username for a player
    SetUsername{ username: String },
    JoinLobby,
    CreateGame,
    JoinGame{ id: GameId },
    /// Starts the game
    StartGame{ id: GameId },
    PlaceToken{ id: GameId, player: u32, port: BasePort },
    PlaceTile{ id: GameId, player: u32, kind: BaseKind, index: u32, action: BaseGAct, loc: BaseTLoc },
    RemovePeer,
}

/// The response type used by the server to communicate to the client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    /// Responds with the index of the player
    PlayerIndex{ id: GameId, index: u32 },
    /// List of players of the game have changed
    ChangedPlayers{ id: GameId, names: Vec<String> },
    /// A game was created or edited in the lobby
    ChangedGame{ game: GameInstance },
    /// A game was joined
    JoinedGame{ game: GameInstance },
    /// The lobby was joined. The lobby has games.
    JoinedLobby{ games: Vec<GameInstance> },
    /// Responds with the game's state
    StartedGame{ id: GameId, state: BaseGameState },
    /// Player `player` has placed a token on port `port`.
    PlacedToken{ id: GameId, player: u32, port: BasePort },
    /// Invalid username
    RejectedUsername,
    /// Invalid move, please undo
    Rejected{ id: GameId },
    /// Everyone placed their tokens; it's time to place some tiles
    AllPlacedTokens{ id: GameId },
    /// It's your turn, make a move
    YourTurn{ id: GameId },
    /// Player `player` has placed a tile transformed by group action `action`
    /// from index `index` in their list of tiles of kind `kind` onto location `loc`.
    PlacedTile{ id: GameId, player: u32, kind: BaseKind, index: u32, action: BaseGAct, loc: BaseTLoc },
    ///// Players moved across tiles. Stores a port per player
    //CrossedTiles{ new_ports: Vec<G::Port> },
    ///// Players died. Stores players that died
    //Died{ dead: Vec<u32> },
    ///// Tiles have been dealt. Stores number of tiles dealt and new tiles per player.
    //DealtTiles{ num_tiles_dealt: u32,  }
}