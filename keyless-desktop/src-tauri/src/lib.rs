// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use keyless_core::error::{KeylessError, KeylessResult};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("hello, {}", name)
}

fn run_inner() -> KeylessResult<()> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet]);

    builder
        .run(tauri::generate_context!())
        .map_err(|e| KeylessError::System(format!("tauri runtime error: {e}")))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = run_inner() {
        // Use stderr for early scaffold; later we'll wire tracing.
        eprintln!("{e}");
    }
}
