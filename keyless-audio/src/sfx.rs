//! SFX module: split into `player` (public API) and `state` (realtime callback state).

pub mod player;
pub mod state;

pub use player::SfxPlayer;
