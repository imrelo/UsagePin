use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

#[cfg(windows)]
mod resize;
mod settings;
mod startup_layout;
mod codex_usage;
<<<<<<< HEAD
mod antigravity_usage;
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc

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
fn get_click_through(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<ClickThroughState>();
    let current = *state.0.lock().map_err(|error| error.to_string())?;
    Ok(current)
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

#[tauri::command]
<<<<<<< HEAD
fn reset_click_through(app: AppHandle) -> Result<(), String> {
    set_click_through(&app, false)
}

#[tauri::command]
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
fn set_compact_view(compact: bool) {
    #[cfg(windows)]
    resize::set_compact_view(compact);
    #[cfg(not(windows))]
    let _ = compact;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ClickThroughState::default())
        .plugin(tauri_plugin_opener::init())
<<<<<<< HEAD
        .invoke_handler(tauri::generate_handler![greet, toggle_click_through, get_click_through, reset_click_through, set_compact_view, settings::open_settings, codex_usage::get_codex_usage, codex_usage::refresh_codex_usage, codex_usage::set_usage_refresh_interval, antigravity_usage::get_antigravity_usage, antigravity_usage::refresh_antigravity_usage])
        .on_window_event(settings::on_window_event)
        .setup(|app| {
            codex_usage::start(app.handle())?;
            antigravity_usage::start(app.handle())?;
=======
        .invoke_handler(tauri::generate_handler![greet, toggle_click_through, get_click_through, set_compact_view, settings::open_settings, codex_usage::get_codex_usage, codex_usage::refresh_codex_usage, codex_usage::set_usage_refresh_interval])
        .on_window_event(settings::on_window_event)
        .setup(|app| {
            codex_usage::start(app.handle())?;
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
            let main_window = app.get_webview_window("main").ok_or("UsagePin window is unavailable")?;
            startup_layout::apply(&main_window)?;
            #[cfg(windows)]
            resize::install(&main_window)?;

            let open_settings = MenuItem::with_id(
                app,
                "open-settings",
                "Open Settings",
                true,
                None::<&str>,
            )?;
            let toggle = MenuItem::with_id(
                app,
                "toggle-click-through",
                "Toggle click-through",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit UsagePin", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_settings, &toggle, &quit])?;

            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .menu_on_left_click(true)
                .tooltip("UsagePin")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open-settings" => {
                        if let Err(error) = settings::open_settings(app.clone()) {
                            eprintln!("Could not open Settings: {error}");
                        }
                    }
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
            main_window.show()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running UsagePin");
}
