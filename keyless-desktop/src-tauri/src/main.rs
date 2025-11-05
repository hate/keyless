//! binary entry point for keyless-desktop.
//!
//! Delegates to the library's `run` function to keep logic centralized.
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    keyless_desktop_lib::run()
}
