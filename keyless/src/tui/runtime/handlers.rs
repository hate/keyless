//! Input handlers for Config and Dictating screens.

pub mod config;
pub mod dictating;

pub use config::{ConfigAction, build_config_from_app, handle_config_input};
pub use dictating::DictatingAction;
