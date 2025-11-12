//! Sink selection IPC commands.
use super::DesktopState;

/// List supported output sinks.
#[tauri::command]
pub fn list_sinks(ctx: DesktopState<'_>) -> Result<Vec<String>, String> {
    // Delegate to the sinks service to list all available output sinks.
    // Returns: ["paste", "clipboard", "file"]
    Ok(ctx.sinks.list())
}

/// Select an output sink and restart the pipeline if needed.
#[tauri::command]
pub fn select_sink(ctx: DesktopState<'_>, id: String) -> Result<(), String> {
    // Select the sink (updates config and returns selection result).
    let selection = ctx.sinks.select(&id)?;

    // Update the tray icon menu to show the correct sink checkmark.
    crate::tray::update_sink_checkmarks_from_config();

    // Create a user-friendly log message based on the selected sink.
    let log_message = match selection.sink_id.as_str() {
        "paste" => "sink changed to paste".to_string(),
        "clipboard" => "sink changed to clipboard".to_string(),
        "file" => {
            // For file sink, include the file path in the message.
            let path = selection
                .file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| String::from("(unknown)"));
            format!("sink changed to file ({path})")
        }
        other => format!("sink changed to {other}"),
    };

    eprintln!("[sinks] {log_message}");
    // Emit log message for UI display.
    ctx.events.emit("log_message", &log_message);
    // Emit output mode change event so frontend can update UI state.
    ctx.events.emit("output_mode_changed", &selection.sink_id);

    // Restart the pipeline to apply the new sink (sink changes require pipeline restart
    // because the output handler needs to be reconfigured).
    ctx.pipeline.request_restart();

    Ok(())
}
