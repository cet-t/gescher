use crate::app_state;
use crate::config::{Config, Direction};
use crate::point::Point;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, grab, simulate};
use std::f64;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thousands::Separable;

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
                let dirs_str: Vec<String> =
                    gesture_directions.iter().map(|d| d.to_string()).collect();

                // デバッグモード時はイベントを発行
                if debug {
                    use tauri::Emitter;
                    let _ = app_handle_analyze.emit("gesture-detected", &dirs_str);
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
                    "gesture".bright_purple().bold(),
                    dirs_str.join(" -> ").bright_white()
                );
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
