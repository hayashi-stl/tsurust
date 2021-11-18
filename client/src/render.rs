use common::board::{AllBoardRenderer, BaseBoard, Board, RectangleBoard};
use itertools::{Itertools, iproduct};
use specs::{Component, VecStorage};
use wasm_bindgen::JsCast;
use web_sys::{DomParser, Element, SupportedType, SvgElement};

use crate::console_log;

fn create_svg_element<S: JsCast>(name: &str) -> S {
    web_sys::window().unwrap().document().unwrap().create_element_ns(Some("http://www.w3.org/2000/svg"), name)
        .expect("SVG element could not be created")
        .dyn_into()
        .expect("Wrong type specified")
}

/// Rendering component
pub struct Model {
    /// Id of the corresponding svg element
    svg_id: String
}

impl Component for Model {
    type Storage = VecStorage<Self>;
}

impl Model {
    /// Adds an SVG to a parent node, taking a counter that is used for the id and increments.
    /// Then returns a `Model`.
    pub fn new(svg_elem: &SvgElement, parent: &SvgElement, id: &mut u64) -> Self {
        svg_elem.set_id(&id.to_string());
        *id += 1;
        parent.append_child(&svg_elem).expect("Failed to add SVG");
        Model { svg_id: svg_elem.id() }
    }
}

pub trait SpecificBoardRenderer<B: Board>: AllBoardRenderer {
    fn render_specific(&self, board: &B) -> Self::Return;
}

pub struct BoardRenderer;

impl SpecificBoardRenderer<RectangleBoard> for BoardRenderer {
    fn render_specific(&self, board: &RectangleBoard) -> Self::Return {
        let svg_str = r##"<g xmlns="http://www.w3.org/2000/svg" fill="#ffd090" stroke="black">"##.to_owned() +
            &iproduct!(0..board.height(), 0..board.width()).map(|(y, x)|
                format!(r#"<rect x="{}" y="{}" stroke-width="0.04" width="1" height="1"/>"#, x, y))
                .join("") +
            r#"</g>"#;
        let svg = DomParser::new().unwrap().parse_from_string(&svg_str, SupportedType::ImageSvgXml)
            .expect("SVG could not be created");
        svg.document_element().expect("SVG doesn't have an element")
            .dyn_into().unwrap()
    }
}

impl AllBoardRenderer for BoardRenderer {
    type Return = SvgElement;

    fn render(&self, board: &BaseBoard) -> Self::Return {
        match board {
            BaseBoard::RectangleBoard(b) => SpecificBoardRenderer::render_specific(self, b)
        }
    }
}