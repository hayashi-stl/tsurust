pub mod processor;

use common::message::Request;
use common::message::Response;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::ErrorEvent;
use web_sys::HtmlInputElement;
use web_sys::{BinaryType, MessageEvent, WebSocket, console};
use std::iter;

use crate::processor::process_response;

macro_rules! console_log {
    ($($t:tt)*) => {
        log(&format_args!($($t)*).to_string())
    };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn run() -> Result<(), JsValue> {
    let ws = WebSocket::new(&format!("ws://{}/", common::HOST_ADDRESS))?;
    ws.set_binary_type(BinaryType::Arraybuffer);

    let username = web_sys::window().unwrap().prompt_with_message("Enter a username")
        .unwrap_or(None)
        .unwrap_or("Guest".to_owned());
    let username_req = Request::SetUsername{ name: username.clone() };
    let username_bytes = bincode::serialize(&username_req).expect("Serialization went wrong");
    match ws.send_with_u8_array(&username_bytes) {
        Ok(_) => {}
        Err(e) => console_log!("Error sending message: {:?}", e),
    }

    let document = web_sys::window().unwrap().document().unwrap();
    
    let mut cws = ws.clone();
    let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(msg) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&msg);
            let msg = bincode::deserialize::<Response>(&array.to_vec()).unwrap();
            console_log!("received response: {:?}", msg);
            
            for req in process_response(&msg) {
                let bytes = bincode::serialize(&msg).expect("Serialization went wrong");
                match cws.send_with_u8_array(&bytes) {
                    Ok(_) => {}
                    Err(e) => console_log!("Error sending message: {:?}", e),
                }
            }
        } 
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let on_error = Closure::wrap(Box::new(move |e: ErrorEvent| {
        console_log!("error {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();

    let cws = ws.clone();
    let on_open = Closure::wrap(Box::new(move |_| {

    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    Ok(())
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    run()
}
