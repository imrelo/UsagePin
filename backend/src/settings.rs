use tauri::{AppHandle, Emitter, Manager, Window, WindowEvent};

#[tauri::command]
pub(crate) fn open_settings(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "Settings window is unavailable".to_string())?;
    window.unminimize().map_err(|error| error.to_string())?;
    window.emit("settings-opened", ()).map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() == "settings" {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.emit("settings-close-requested", ());
        }
    }
}
