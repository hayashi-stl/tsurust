use common::message::{Request, Response};
use itertools::Itertools;
use web_sys::WebSocket;

/// Processes a response and makes a nonnegative number of requests
pub fn process_response(resp: &Response) -> Vec<Request> {
    let doc = web_sys::window().unwrap().document().unwrap();

    match resp {
        Response::Usernames{ names } => {
            let names_str = names.into_iter().join("\n\n");
            doc.get_element_by_id("usernames").unwrap().set_inner_html(&names_str);
            vec![]
        }

        _ => vec![]
    }
}