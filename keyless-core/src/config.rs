//! Application configuration.
//!
//! This module provides configuration types, validation, and persistence for the keyless
//! workspace. It organizes the implementation into focused submodules:
//!
//! - `schema`: Configuration schema types (`Config`, `OutputMode`, `VadThresholds`, `EqTuning`)
//!   with validation and constants for EQ parameter bounds
//! - `storage`: Persistence helpers for loading and saving config from disk (JSON, atomic writes)

/// Configuration schema (Config, OutputMode, VAD, EQ tuning).
pub mod schema;
/// Persistence helpers (load/save config from disk).
pub mod storage;

pub use schema::{
    Config, DEFAULT_MODEL_ID, EQ_ATTACK_MAX, EQ_ATTACK_MIN, EQ_BANDS_MAX, EQ_BANDS_MIN,
    EQ_DECAY_MAX, EQ_DECAY_MIN, EQ_GAMMA_MAX, EQ_GAMMA_MIN, EQ_NOISE_REDUCTION_MAX,
    EQ_NOISE_REDUCTION_MIN, EQ_WINDOW_DB_MAX, EQ_WINDOW_DB_MIN, EqTuning, OutputMode,
    VadThresholds,
};
