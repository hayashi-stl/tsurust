use common::message::{Request, Response};
use itertools::Itertools;
use web_sys::WebSocket;

use crate::{console_log, game::GameWorld};

/// Processes a response and makes a nonnegative number of requests
pub fn process_response(resp: Response, game_world: &mut GameWorld) -> Vec<Request> {
    let doc = web_sys::window().unwrap().document().unwrap();

    match resp {
        Response::Usernames{ names } => {
            let names_str = names.into_iter().join("\n\n");
            doc.get_element_by_id("usernames").unwrap().set_inner_html(&names_str);
            vec![]
        }

        Response::State { game, state } => {
            game_world.set_game(game, state);
            vec![]
        }

        _ => vec![]
    }
}

/// Sends a request to the server.
pub fn send_request(req: &Request, ws: &WebSocket) {
    let bytes = bincode::serialize(&req).expect("Serialization went wrong");
    match ws.send_with_u8_array(&bytes) {
        Ok(_) => console_log!("Sent message: {:?}", req),
        Err(e) => console_log!("Error sending message {:?}: {:?}", req, e),
    }
}