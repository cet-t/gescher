use crate::app_state;
use crate::config::{Config, Direction};
use crate::point::Point;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, Key, grab, simulate};
use std::f64;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thousands::Separable;

lazy_static::lazy_static! {
    static ref RECORDING_MODE: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref RECORDED_GESTURE: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

pub fn start_recording(_app: tauri::AppHandle) {
    *RECORDING_MODE.lock().unwrap() = true;
    *RECORDED_GESTURE.lock().unwrap() = None;
    println!("Gesture recording started");
}

pub fn stop_recording() -> Result<String, String> {
    *RECORDING_MODE.lock().unwrap() = false;
    let gesture = RECORDED_GESTURE.lock().unwrap().clone();
    println!("Gesture recording stopped: {:?}", gesture);
    gesture.ok_or_else(|| "No gesture recorded".to_string())
}

struct GestureTask {
    points: Vec<Point>,
    elapsed: Duration,
}

#[inline]
fn denoise_vec(points: Vec<Point>, window_size: usize) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut res = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let actual_size = window_size.min(points.len() - i);
        let mut sum = Point::zero();
        for j in 0..actual_size {
            sum = sum + points[i + j];
        }
        res.push(sum / actual_size as f64);
    }
    res
}

pub fn start_gesture_service(app_handle: tauri::AppHandle, config_atom: Arc<Mutex<Config>>) {
    let (tx, rx) = mpsc::channel::<GestureTask>();
    let config_analyze = Arc::clone(&config_atom);
    let app_handle_analyze = app_handle.clone();

    // 解析ワーカースレッド
    thread::spawn(move || {
        while let Ok(task) = rx.recv() {
            let (win, m_threshold, a_threshold, c_threshold, allow_diagonal, debug) = {
                let conf = config_analyze.lock().unwrap();
                (
                    conf.window_size,
                    conf.moving_threshold,
                    conf.angle_threshold.to_radians(),
                    conf.corner_threshold.to_radians(),
                    conf.allow_diagonal,
                    conf.debug,
                )
            };

            let mut points = task.points;
            points.reverse();
            let denoised = denoise_vec(points, win);

            // 判定に十分な長さがない場合
            if denoised.len() < win * 2 {
                app_state::add_ignore_count(2); // Press と Release の 2つ分
                let _ = simulate(&EventType::ButtonPress(Button::Right));
                let _ = simulate(&EventType::ButtonRelease(Button::Right));
                continue;
            }

            let dir_threshold = a_threshold.sin();
            let mut gesture_directions = Vec::new();
            let mut current_direction = Direction::UNDEFINED;
            let mut last_v = Point::zero();
            let mut last_corner_idx = 0;

            for i in 0..denoised.len().saturating_sub(win) {
                let p_start = denoised[i];
                let p_end = denoised[i + win];
                let seg_vec = p_end - p_start;
                if seg_vec.magnitude() > m_threshold / 2.0 {
                    let dir = seg_vec.direction(dir_threshold, allow_diagonal);
                    if last_v.magnitude() > f64::EPSILON {
                        let seg_angle = last_v.normalized().angle(seg_vec.normalized());
                        if seg_angle > c_threshold && (i > last_corner_idx + win) {
                            if dir != Direction::UNDEFINED && !current_direction.intersects(dir) {
                                gesture_directions.push(dir);
                                current_direction = dir;
                                last_corner_idx = i;
                            }
                        }
                    }
                    if i % win == 0 && dir != Direction::UNDEFINED {
                        if current_direction.is_empty() || !current_direction.intersects(dir) {
                            gesture_directions.push(dir);
                            current_direction = dir;
                        }
                    }
                    last_v = seg_vec;
                }
            }

            if !gesture_directions.is_empty() {
                // コンテキストメニューが出てしまっている場合に閉じる
                let _ = simulate(&EventType::KeyPress(Key::Escape));
                let _ = simulate(&EventType::KeyRelease(Key::Escape));

                let dirs_str: Vec<String> =
                    gesture_directions.iter().map(|d| d.to_string()).collect();
                let detected_str = dirs_str.join(" -> ");

                // アクション実行
                let current_app = match active_win_pos_rs::get_active_window() {
                    Ok(window) => Some(window.process_path.to_string_lossy().to_string()),
                    Err(_) => None,
                };

                let app_name = current_app
                    .as_ref()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    })
                    .unwrap_or(std::borrow::Cow::Borrowed("Unknown"));

                // デバッグモード時はイベントを発行
                if debug {
                    use tauri::Emitter;
                    #[derive(Clone, serde::Serialize)]
                    struct GestureDetectedPayload {
                        gestures: Vec<String>,
                        app_name: String,
                    }
                    let payload = GestureDetectedPayload {
                        gestures: dirs_str.clone(),
                        app_name: app_name.to_string(),
                    };
                    let _ = app_handle_analyze.emit("gesture-detected", &payload);
                }

                println!(
                    "{:<20} {:>6} {:>6} items",
                    "elapsed".bright_green().bold(),
                    task.elapsed
                        .as_millis()
                        .separate_with_commas()
                        .bright_white(),
                    denoised.len().separate_with_commas().bright_white()
                );
                println!(
                    "{:<20} {}",
                    "app".bright_blue().bold(),
                    app_name.bright_white()
                );
                println!(
                    "{:<20} {}",
                    "gesture".bright_purple().bold(),
                    detected_str.bright_white()
                );

                // 記録モードチェック
                let is_recording = *RECORDING_MODE.lock().unwrap();
                if is_recording {
                    *RECORDED_GESTURE.lock().unwrap() = Some(detected_str.clone());
                    use tauri::Emitter;
                    let _ = app_handle_analyze.emit("gesture-recorded", &detected_str);
                    println!(
                        "{:<20} {}",
                        "recorded".bright_cyan().bold(),
                        detected_str.bright_white()
                    );
                    continue; // 記録モード中は通常のアクション実行をスキップ
                }

                for g in config_analyze.lock().unwrap().gestures() {
                    if !g.enabled {
                        continue;
                    }

                    if g.trigger == detected_str {
                        // アプリケーション制限
                        if let Some(target) = &g.target_bin {
                            if let Some(app_path) = &current_app {
                                if !app_path.contains(target) {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }

                        // コマンド実行
                        if let Some(cmd) = &g.command {
                            println!("{:<20} {}", "execute".bright_yellow().bold(), cmd);
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("cmd").args(["/C", cmd]).spawn();
                            #[cfg(not(target_os = "windows"))]
                            let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
                        }

                        // テキスト入力
                        if let Some(text) = &g.text {
                            println!("{:<20} {}", "text input".bright_cyan().bold(), text);
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if clipboard.set_text(text.clone()).is_ok() {
                                    // 貼り付け操作 (Ctrl+V or Cmd+V)
                                    #[cfg(target_os = "macos")]
                                    let modifier = rdev::Key::MetaLeft;
                                    #[cfg(not(target_os = "macos"))]
                                    let modifier = rdev::Key::ControlLeft;

                                    let _ = simulate(&EventType::KeyPress(modifier));
                                    let _ = simulate(&EventType::KeyPress(rdev::Key::KeyV));
                                    let _ = simulate(&EventType::KeyRelease(rdev::Key::KeyV));
                                    let _ = simulate(&EventType::KeyRelease(modifier));
                                }
                            }
                        }

                        // キー入力 (例: "Ctrl+Shift+T")
                        if let Some(keys_str) = &g.keys {
                            println!("{:<20} {}", "key input".bright_magenta().bold(), keys_str);
                            let parts: Vec<&str> = keys_str.split('+').collect();
                            let mut keys_to_press = Vec::new();

                            for part in parts {
                                let key = match part.trim().to_lowercase().as_str() {
                                    "ctrl" | "control" => rdev::Key::ControlLeft,
                                    "shift" => rdev::Key::ShiftLeft,
                                    "alt" => rdev::Key::Alt,
                                    "meta" | "super" | "win" | "cmd" => rdev::Key::MetaLeft,
                                    "enter" | "return" => rdev::Key::Return,
                                    "space" => rdev::Key::Space,
                                    "backspace" => rdev::Key::Backspace,
                                    "delete" | "del" => rdev::Key::Delete,
                                    "tab" => rdev::Key::Tab,
                                    "esc" | "escape" => rdev::Key::Escape,
                                    "up" => rdev::Key::UpArrow,
                                    "down" => rdev::Key::DownArrow,
                                    "left" => rdev::Key::LeftArrow,
                                    "right" => rdev::Key::RightArrow,
                                    "f1" => rdev::Key::F1,
                                    "f2" => rdev::Key::F2,
                                    "f3" => rdev::Key::F3,
                                    "f4" => rdev::Key::F4,
                                    "f5" => rdev::Key::F5,
                                    "f6" => rdev::Key::F6,
                                    "f7" => rdev::Key::F7,
                                    "f8" => rdev::Key::F8,
                                    "f9" => rdev::Key::F9,
                                    "f10" => rdev::Key::F10,
                                    "f11" => rdev::Key::F11,
                                    "f12" => rdev::Key::F12,
                                    s if s.len() == 1 => {
                                        let c = s.chars().next().unwrap();
                                        match c {
                                            'a' => rdev::Key::KeyA,
                                            'b' => rdev::Key::KeyB,
                                            'c' => rdev::Key::KeyC,
                                            'd' => rdev::Key::KeyD,
                                            'e' => rdev::Key::KeyE,
                                            'f' => rdev::Key::KeyF,
                                            'g' => rdev::Key::KeyG,
                                            'h' => rdev::Key::KeyH,
                                            'i' => rdev::Key::KeyI,
                                            'j' => rdev::Key::KeyJ,
                                            'k' => rdev::Key::KeyK,
                                            'l' => rdev::Key::KeyL,
                                            'm' => rdev::Key::KeyM,
                                            'n' => rdev::Key::KeyN,
                                            'o' => rdev::Key::KeyO,
                                            'p' => rdev::Key::KeyP,
                                            'q' => rdev::Key::KeyQ,
                                            'r' => rdev::Key::KeyR,
                                            's' => rdev::Key::KeyS,
                                            't' => rdev::Key::KeyT,
                                            'u' => rdev::Key::KeyU,
                                            'v' => rdev::Key::KeyV,
                                            'w' => rdev::Key::KeyW,
                                            'x' => rdev::Key::KeyX,
                                            'y' => rdev::Key::KeyY,
                                            'z' => rdev::Key::KeyZ,
                                            '0' => rdev::Key::Num0,
                                            '1' => rdev::Key::Num1,
                                            '2' => rdev::Key::Num2,
                                            '3' => rdev::Key::Num3,
                                            '4' => rdev::Key::Num4,
                                            '5' => rdev::Key::Num5,
                                            '6' => rdev::Key::Num6,
                                            '7' => rdev::Key::Num7,
                                            '8' => rdev::Key::Num8,
                                            '9' => rdev::Key::Num9,
                                            _ => rdev::Key::Unknown(0),
                                        }
                                    }
                                    _ => rdev::Key::Unknown(0),
                                };
                                if key != rdev::Key::Unknown(0) {
                                    keys_to_press.push(key);
                                }
                            }

                            // 繰り返し設定
                            let repeat_count = g.repeat_count.unwrap_or(1).max(1);
                            let repeat_interval = g.repeat_interval.unwrap_or(0);

                            for i in 0..repeat_count {
                                // キープレス
                                for k in &keys_to_press {
                                    let _ = simulate(&EventType::KeyPress(*k));
                                }
                                // キーリリースは逆順に行う
                                for k in keys_to_press.iter().rev() {
                                    let _ = simulate(&EventType::KeyRelease(*k));
                                }

                                // 最後の繰り返し以外は待機
                                if i < repeat_count - 1 && repeat_interval > 0 {
                                    thread::sleep(Duration::from_millis(repeat_interval));
                                }
                            }
                        }
                    }
                }
            } else {
                app_state::add_ignore_count(2);
                let _ = simulate(&EventType::ButtonPress(Button::Right));
                let _ = simulate(&EventType::ButtonRelease(Button::Right));
            }
        }
    });

    // 入力監視スレッド
    let config_grab = Arc::clone(&config_atom);
    thread::spawn(move || {
        let (buffer_size, window_size) = {
            let conf = config_grab.lock().unwrap();
            (conf.buffer_size, conf.window_size)
        };
        let trails_atm = Arc::new(Mutex::new(CircularQueue::<Point>::with_capacity(
            buffer_size,
        )));
        let trails_clone = Arc::clone(&trails_atm);
        let st_mu = Arc::new(Mutex::new(Option::<Instant>::None));
        let lp_mu = Arc::new(Mutex::new(Point::zero()));
        let st_clone = Arc::clone(&st_mu);
        let lp_clone = Arc::clone(&lp_mu);

        println!(
            "Starting rdev grab thread with window_size: {}",
            window_size
        );

        // Windows では grab を実行するスレッド自身でメッセージを処理するか、
        // 少なくとも同じプロセス内で定期的にメッセージが処理される必要がある。
        // grab 自身がブロックするため、別スレッドでメッセージポンプを回す
        #[cfg(target_os = "windows")]
        thread::spawn(|| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG};
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) > 0 {}
            }
        });

        if let Err(e) = grab(move |event| {
            let active = app_state::is_active();
            let ignore = app_state::should_ignore();

            if ignore {
                return Some(event);
            }

            let mut intercept = false;
            match event.event_type {
                EventType::ButtonPress(Button::Right) => {
                    if let Ok(mut st) = st_clone.lock() {
                        *st = Some(Instant::now());
                    }
                    // 設定画面が開いていない時のみ、右ボタンを遮断する
                    if active {
                        intercept = true;
                    }
                }
                EventType::ButtonRelease(Button::Right) => {
                    let mut start_val = None;
                    if let Ok(mut st) = st_clone.lock() {
                        start_val = st.take();
                    }
                    if let Some(start) = start_val {
                        let elapsed = Instant::now() - start;
                        if let Ok(mut q) = trails_clone.lock() {
                            let points_vec: Vec<Point> = q.iter().cloned().collect();
                            q.clear();
                            let _ = tx.send(GestureTask {
                                points: points_vec,
                                elapsed,
                            });
                        }
                    }
                    // 設定画面が開いていない時のみ、右ボタンを遮断する
                    if active {
                        intercept = true;
                    }
                }
                EventType::MouseMove { x, y } => {
                    let point = Point::new(x, -y);
                    let mut is_tracking = false;
                    let mut prev_lp = Point::zero();
                    if let Ok(st) = st_clone.lock() {
                        is_tracking = st.is_some();
                    }
                    if is_tracking {
                        if let Ok(mut lp) = lp_clone.lock() {
                            prev_lp = *lp;
                            *lp = point;
                        }
                        if (prev_lp - point).magnitude() > 1.0 {
                            if let Ok(mut q) = trails_clone.lock() {
                                let _ = q.push(point);
                            }
                        }
                    }
                }
                _ => {}
            }
            if intercept { None } else { Some(event) }
        }) {
            println!("Grab Error: {:?}", e);
        }
    });
}
