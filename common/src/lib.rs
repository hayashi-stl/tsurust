pub mod board;
pub mod math;
pub mod tile;
pub mod game;
pub mod player_state;
pub mod board_state;
pub mod game_state;
pub mod message;

use game::GameId;
use game::BaseGame;
use game_state::BaseGameState;
use getset::{Getters, CopyGetters};
pub use nalgebra;
use player_state::Looker;
use rand::{distributions::{Uniform}, prelude::Distribution, thread_rng};
use rand_pcg::Pcg64;
use rand_core::SeedableRng;
use serde::Deserialize;
use serde::Serialize;

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

pub trait WrapBase {
    type Base;

    fn wrap_base(self) -> Self::Base;

    fn unwrap_base(base: Self::Base) -> Self;

    fn unwrap_base_ref(base: &Self::Base) -> &Self;
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

#[macro_export]
macro_rules! impl_wrap_base {
    ($base:ident :: $variant:ident ( $ty:ty )) => {
        impl $crate::WrapBase for $ty {
            type Base = $base;

            fn wrap_base(self) -> Self::Base {
                Self::Base::$variant(self)
            }

            fn unwrap_base(base: Self::Base) -> Self {
                #[allow(irrefutable_let_patterns)]
                if let Self::Base::$variant(x) = base {
                    x
                } else { panic!("Mismatched type and associated type") }
            }

            fn unwrap_base_ref(base: &Self::Base) -> &Self {
                #[allow(irrefutable_let_patterns)]
                if let Self::Base::$variant(x) = base {
                    x
                } else { panic!("Mismatched type and associated type") }
            }
        }
    };
}

#[derive(Clone, Debug, Getters, CopyGetters, Serialize, Deserialize)]
pub struct GameInstance {
    #[getset(get_copy = "pub")]
    id: GameId,
    #[getset(get = "pub")]
    game: BaseGame,
    /// None if the game hasn't started
    #[getset(get = "pub")]
    state: Option<BaseGameState>,
    /// stores username
    #[getset(get = "pub")]
    players: Vec<String>, 
}

impl GameInstance {
    pub fn new(id: GameId, game: BaseGame, state: Option<BaseGameState>, players: Vec<String>) -> Self {
        Self { id, game, state, players }
    }

    /// Sets the looker of the game state. The game state must exist.
    pub fn set_looker(&mut self, looker: Looker) {
        self.state = Some(self.state.as_ref().unwrap().visible_state(looker));
    }

    /// Extracts all the fields for separate manipulation.
    pub fn into_fields(self) -> (GameId, BaseGame, Option<BaseGameState>, Vec<String>) {
        (self.id, self.game, self.state, self.players)
    }
}