use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

#[derive(Default)]
struct ClickThroughState(Mutex<bool>);

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn set_click_through(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "UsagePin window is unavailable".to_string())?;
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|error| error.to_string())?;

    let state = app.state::<ClickThroughState>();
    *state.0.lock().map_err(|error| error.to_string())? = enabled;

    app.emit("click-through-changed", enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn toggle_click_through(app: AppHandle) -> Result<bool, String> {
    let current = *app
        .state::<ClickThroughState>()
        .0
        .lock()
        .map_err(|error| error.to_string())?;
    let next = !current;
    set_click_through(&app, next)?;
    Ok(next)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ClickThroughState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, toggle_click_through])
        .setup(|app| {
            let toggle = MenuItem::with_id(
                app,
                "toggle-click-through",
                "Toggle click-through",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit UsagePin", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;

            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .menu_on_left_click(true)
                .tooltip("UsagePin")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle-click-through" => {
                        let current = *app
                            .state::<ClickThroughState>()
                            .0
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        if let Err(error) = set_click_through(app, !current) {
                            eprintln!("Could not toggle click-through: {error}");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });

            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }

            let _tray_icon = tray.build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running UsagePin");
}
