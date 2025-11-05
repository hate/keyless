//! build script for keyless-desktop.
//!
//! This script lets tauri embed metadata and resources at compile time.
//! It has no runtime side effects beyond build-time configuration.
fn main() {
    tauri_build::build()
}
