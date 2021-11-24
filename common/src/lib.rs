pub mod board;
pub mod math;
pub mod tile;
pub mod game;
pub mod player_state;
pub mod board_state;
pub mod game_state;
pub mod message;

pub use nalgebra;
use rand::{distributions::{Uniform, uniform::UniformInt}, prelude::Distribution, thread_rng};
use rand_pcg::Pcg64;
use rand_core::SeedableRng;

pub const HOST_ADDRESS: &str = "127.0.0.1:7878";

/// Constructs a PCG RNG from a seed
pub fn pcg64_seeded(seed: u64) -> Pcg64 {
    Pcg64::seed_from_u64(seed)
}

pub fn pcg64() -> Pcg64 {
    let seed = Uniform::from(0..=u64::MAX).sample(&mut thread_rng());
    pcg64_seeded(seed)
}

/// Constructs a PCG64 from a random seed and debugs the seed
#[macro_export]
macro_rules! pcg64 {
    ($($t:tt)*) => {
        {
            use rand::prelude::Distribution;
            let seed = rand::distributions::Uniform::from(0..=u64::MAX).sample(&mut rand::thread_rng());
            log::debug!($($t)*);
            log::debug!("Seed {}", seed);
            $crate::pcg64_seeded(seed)
        }
    };
}

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
    (($($vis:tt)*) $base:ident, $variant:ident) => {
        $($vis)* fn wrap_base(self) -> $base {
            $base::$variant(self)
        }

        $($vis)* fn unwrap_base(base: $base) -> Self {
            #[allow(irrefutable_let_patterns)]
            if let $base::$variant(x) = base {
                x
            } else { panic!("Mismatched type and associated type") }
        }

        $($vis)* fn unwrap_base_ref(base: &$base) -> &Self {
            #[allow(irrefutable_let_patterns)]
            if let $base::$variant(x) = base {
                x
            } else { panic!("Mismatched type and associated type") }
        }
    };
}