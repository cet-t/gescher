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

fn get_config_path() -> std::path::PathBuf {
    // 実行ファイルのディレクトリを取得
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let exe_dir = exe_path
        .parent()
        .expect("Failed to get executable directory");

    // 開発環境・本番環境ともに実行ファイルと同じディレクトリ
    let config_path = exe_dir.join("config.yaml");

    config_path
}

fn ensure_config_exists(config_path: &std::path::Path) -> Result<(), String> {
    if !config_path.exists() {
        println!(
            "Config file not found, creating default config at {:?}",
            config_path
        );

        // デフォルト設定を作成
        let default_config = Config {
            buffer_size: 1024,
            window_size: 10,
            moving_threshold: 10.0,
            angle_threshold: 15.0,
            corner_threshold: 30.0,
            allow_diagonal: false,
            debug: true,
            language: "en".to_string(),
            gestures: vec![],
        };

        default_config
            .save(config_path.to_str().unwrap())
            .map_err(|e| e.to_string())?;
        println!("Default config created successfully");
    }
    Ok(())
}

struct AppConfig(Arc<Mutex<Config>>);

#[tauri::command]
fn get_config(state: tauri::State<'_, AppConfig>) -> Result<Config, String> {
    // ファイルから最新の設定を読み込む
    let config_path = get_config_path();
    let file = File::open(&config_path)
        .map_err(|e| format!("Failed to open config file at {:?}: {}", config_path, e))?;
    let reader = BufReader::new(file);
    let config: Config = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

    // メモリも更新
    let mut current_config = state.0.lock().unwrap();
    *current_config = config.clone();

    Ok(config)
}

#[tauri::command]
fn update_config(
    _app: tauri::AppHandle,
    state: tauri::State<'_, AppConfig>,
    config: Config,
) -> Result<(), String> {
    println!(
        "update_config called with {} gestures",
        config.gestures.len()
    );

    // デバッグ: 受信した設定の詳細を出力
    if let Ok(json) = serde_json::to_string_pretty(&config) {
        println!("Received config:\n{}", json);
    }

    for (i, g) in config.gestures.iter().enumerate() {
        println!(
            "Gesture {}: trigger={}, command={:?}, text={:?}, keys={:?}",
            i, g.trigger, g.command, g.text, g.keys
        );
    }

    // メモリを更新
    let mut current_config = state.0.lock().unwrap();
    *current_config = config.clone();
    logger::set_enabled(current_config.debug);

    // 即座にファイルに保存
    let config_path = get_config_path();
    println!("Saving config to {:?}", config_path);
    current_config
        .save(config_path.to_str().unwrap())
        .map_err(|e| e.to_string())?;

    println!("Config saved successfully");
    Ok(())
}

#[tauri::command]
fn save_config(state: tauri::State<'_, AppConfig>) -> Result<(), String> {
    let current_config = state.0.lock().unwrap();
    let config_path = get_config_path();
    current_config
        .save(config_path.to_str().unwrap())
        .map_err(|e| e.to_string())
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
    let config_path = get_config_path();
    let file = File::open(&config_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let new_config: Config = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

    let mut current_config = state.0.lock().unwrap();
    *current_config = new_config.clone();
    logger::set_enabled(current_config.debug);

    Ok(new_config)
}

#[tauri::command]
fn get_running_processes() -> Result<Vec<String>, String> {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut processes: Vec<String> = sys
        .processes()
        .values()
        .filter_map(|p| {
            p.exe().and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
        })
        .collect();

    processes.sort();
    processes.dedup();
    Ok(processes)
}

#[tauri::command]
fn start_gesture_recording(app: tauri::AppHandle) -> Result<(), String> {
    gesture::start_recording(app);
    Ok(())
}

#[tauri::command]
fn stop_gesture_recording() -> Result<String, String> {
    gesture::stop_recording()
}

#[tauri::command]
fn get_locales() -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    let locales_path = exe_dir.join("locales.json");

    std::fs::read_to_string(&locales_path)
        .map_err(|e| format!("Failed to read locales.json: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. Load config
    let config_path = get_config_path();
    println!("Loading config from: {:?}", config_path);

    // 設定ファイルが存在しない場合は作成
    ensure_config_exists(&config_path).expect("Failed to create default config");

    let file = File::open(&config_path).expect("Failed to open config.yaml");
    let reader = BufReader::new(file);
    let initial_config: Config =
        serde_yaml::from_reader(reader).expect("Failed to parse config.yaml");
    let config_atom = Arc::new(Mutex::new(initial_config));

    logger::set_enabled(config_atom.lock().unwrap().debug);

    // 2. Start gesture service
    app_state::set_active(false);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
                            window.unminimize().unwrap();
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
                            window.unminimize().unwrap();
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
            tauri::WindowEvent::Resized(size) => {
                if size.width == 0 && size.height == 0 {
                    window.hide().unwrap();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            save_config,
            hide_window,
            exit_app,
            reload_config,
            get_running_processes,
            start_gesture_recording,
            stop_gesture_recording,
            get_locales
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
