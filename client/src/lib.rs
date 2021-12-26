pub mod processor;
pub mod render;
pub mod game;
pub mod ecs;

use common::game::GameId;
use common::message::Request;
use common::message::Response;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Document;
use web_sys::Element;
use web_sys::ErrorEvent;
use web_sys::Event;
use web_sys::HtmlInputElement;
use web_sys::Window;
use web_sys::{BinaryType, MessageEvent, WebSocket, console};
use std::cell::Cell;
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use crate::game::GameWorld;
use crate::processor::process_response;
use crate::processor::send_request;

/// The SVG namespace
pub const SVG_NS: &'static str = "http://www.w3.org/2000/svg";

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        $crate::log(&format_args!($($t)*).to_string())
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

pub fn window() -> Window {
    web_sys::window().expect("Cannot get window")
}

pub fn document() -> Document {
    window().document().expect("Cannot get document")
}

/// Adds an event listener to an element.
/// WARNING: This leaks the callback.
fn add_event_listener<E: 'static + FromWasmAbi>(element: &Element, event_name: &str, callback: impl FnMut(E) + 'static) {
    let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(E)>);
    element.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref()).unwrap();
    closure.forget()
}

fn request_animation_frame(callback: &Closure<dyn FnMut()>) {
    window().request_animation_frame(callback.as_ref().unchecked_ref()).expect("Cannot request animation frame");
}

fn run() -> Result<(), JsValue> {
    let ws = WebSocket::new(&format!("ws://{}/", common::HOST_ADDRESS))?;
    ws.set_binary_type(BinaryType::Arraybuffer);
    let game_world = Arc::new(Mutex::new(GameWorld::new()));

    let username = window().prompt_with_message("Enter a username")
        .unwrap_or(None)
        .unwrap_or("Guest".to_owned());
    send_request(&Request::SetUsername{ username }, &ws);

    let cws = ws.clone();
    add_event_listener(&document().get_element_by_id("create").unwrap(), "click", move |_: Event| {
        send_request(&Request::CreateGame, &cws);
    });
    
    let cws = ws.clone();
    let cgw = Arc::clone(&game_world);
    let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(msg) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&msg);
            let msg = bincode::deserialize::<Response>(&array.to_vec()).unwrap();
            console_log!("received response: {:?}", msg);
            
            for req in process_response(msg, &mut cgw.lock().unwrap()) {
                send_request(&req, &cws);
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

    let on_frame = Rc::new(RefCell::new(None));
    let on_frame_clone = Rc::clone(&on_frame);
    let cgw = Arc::clone(&game_world);
    let cws = ws.clone();
    *on_frame.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        for req in cgw.lock().unwrap().update() {
            send_request(&req, &cws);
        }

        request_animation_frame(on_frame_clone.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));
    request_animation_frame(on_frame.borrow().as_ref().unwrap());
    
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
