mod app_state;
mod config;
mod gesture;
mod logger;
mod point;

use crate::config::Config;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};

const CONFIG_PATH: &str = "config.yaml";

struct AppConfig(Arc<Mutex<Config>>);

#[tauri::command]
fn get_config(state: tauri::State<'_, AppConfig>) -> Config {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn update_config(_app: tauri::AppHandle, state: tauri::State<'_, AppConfig>, config: Config) {
    let mut current_config = state.0.lock().unwrap();
    *current_config = config;
    logger::set_enabled(current_config.debug);
}

#[tauri::command]
fn save_config(state: tauri::State<'_, AppConfig>) -> Result<(), String> {
    let current_config = state.0.lock().unwrap();
    current_config.save(CONFIG_PATH).map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_window(window: tauri::Window) {
    app_state::set_active(true);
    window.hide().unwrap();
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn reload_config(state: tauri::State<'_, AppConfig>) -> Result<Config, String> {
    let file = File::open(CONFIG_PATH).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let new_config: Config = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

    let mut current_config = state.0.lock().unwrap();
    *current_config = new_config.clone();
    logger::set_enabled(current_config.debug);

    Ok(new_config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. Load config
    let file = File::open(CONFIG_PATH).expect("Failed to open config.yaml");
    let reader = BufReader::new(file);
    let initial_config: Config =
        serde_yaml::from_reader(reader).expect("Failed to parse config.yaml");
    let config_atom = Arc::new(Mutex::new(initial_config));

    logger::set_enabled(config_atom.lock().unwrap().debug);

    // 2. Start gesture service
    app_state::set_active(false);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppConfig(Arc::clone(&config_atom)))
        .setup(move |app| {
            // Start gesture service with handle
            gesture::start_gesture_service(app.handle().clone(), config_atom);

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            app_state::set_active(false);
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|window, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = window.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            app_state::set_active(false);
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                app_state::set_active(true);
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            save_config,
            hide_window,
            exit_app,
            reload_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
