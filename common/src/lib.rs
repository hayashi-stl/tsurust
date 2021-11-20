pub mod board;
pub mod math;
pub mod tile;
pub mod game;
pub mod player_state;
pub mod board_state;
pub mod game_state;
pub mod message;

pub use nalgebra;

pub const HOST_ADDRESS: &str = "127.0.0.1:7878";

#[macro_export]
macro_rules! wrap_functions {
    ($base:ident) => {
        fn wrap_base(self) -> $base;
        fn unwrap_base(base: $base) -> Self;
        fn unwrap_base_ref(base: &$base) -> &Self;
    };
}

#[macro_export]
macro_rules! impl_wrap_functions {
    ($base:ident, $variant:ident) => {
        fn wrap_base(self) -> $base {
            $base::$variant(self)
        }

        fn unwrap_base(base: $base) -> Self {
            #[allow(irrefutable_let_patterns)]
            if let $base::$variant(x) = base {
                x
            } else { panic!("Mismatched type and associated type") }
        }

        fn unwrap_base_ref(base: &$base) -> &Self {
            #[allow(irrefutable_let_patterns)]
            if let $base::$variant(x) = base {
                x
            } else { panic!("Mismatched type and associated type") }
        }
    };
}