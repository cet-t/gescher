mod app_state;
mod config;
mod filters;
mod logger;
mod point;
mod tray;

use crate::config::{Config, Direction};
use crate::point::Point;
use crate::tray::{TrayAction, TrayManager};
use anyhow::Result;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, grab, simulate};
use slint::ComponentHandle;
use std::f64;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thousands::Separable;

slint::include_modules!();

const CONFIG_PATH: &str = "config.yaml";

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

fn main() -> Result<()> {
    let file = File::open(CONFIG_PATH)?;
    let reader = BufReader::new(&file);
    let initial_config = serde_yaml::from_reader::<_, Config>(reader)?;

    // 設定を共有管理
    let config_atom = Arc::new(Mutex::new(initial_config));
    let config_main = Arc::clone(&config_atom);

    logger::set_enabled(config_main.lock().unwrap().debug);

    let (tx, rx) = mpsc::channel::<GestureTask>();
    let config_analyze = Arc::clone(&config_atom);

    // --- 解析専用ワーカースレッド ---
    thread::spawn(move || {
        while let Ok(task) = rx.recv() {
            let (win, m_threshold, a_threshold, c_threshold, allow_diagonal) = {
                let conf = config_analyze.lock().unwrap();
                (
                    conf.window_size,
                    conf.moving_threshold,
                    conf.angle_threshold.to_radians(),
                    conf.corner_threshold.to_radians(),
                    conf.allow_diagonal,
                )
            };

            let mut points = task.points;
            points.reverse();

            let denoised = denoise_vec(points, win);
            if denoised.len() < win * 2 {
                app_state::add_ignore_count(2);
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
                    gesture_directions
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                        .bright_white()
                );
            } else {
                app_state::add_ignore_count(2);
                let _ = simulate(&EventType::ButtonPress(Button::Right));
                let _ = simulate(&EventType::ButtonRelease(Button::Right));
            }
        }
    });

    let config_grab = Arc::clone(&config_atom);
    let trails_atm = Arc::new(Mutex::new(CircularQueue::<Point>::with_capacity(2048)));
    let trails_clone = Arc::clone(&trails_atm);
    let st_mu = Arc::new(Mutex::new(Option::<Instant>::None));
    let lp_mu = Arc::new(Mutex::new(Point::zero()));
    let st_clone = Arc::clone(&st_mu);
    let lp_clone = Arc::clone(&lp_mu);

    // --- 入力監視スレッド ---
    thread::spawn(move || {
        if let Err(e) = grab(move |event| {
            if app_state::should_ignore() {
                return Some(event);
            }
            if !app_state::is_active() {
                return Some(event);
            }

            let mut intercept = false;
            match event.event_type {
                EventType::ButtonPress(Button::Right) => {
                    if let Ok(mut st) = st_clone.lock() {
                        *st = Some(Instant::now());
                    }
                    intercept = true;
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
                    intercept = true;
                }
                EventType::MouseMove { x, y } => {
                    let point = Point::new(x, -y);
                    let mut is_tracking = false;
                    let mut prev_lp = Point::zero();

                    if let Ok(st) = st_clone.lock() {
                        is_tracking = st.is_some();
                    }
                    if let Ok(mut lp) = lp_clone.lock() {
                        prev_lp = *lp;
                        *lp = point;
                    }

                    if is_tracking {
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

    let tray_manager = TrayManager::new();
    println!("Gescher is running. Check system tray.");

    loop {
        match tray_manager.update() {
            TrayAction::OpenSettings => {
                let config_ui = Arc::clone(&config_atom);
                thread::spawn(move || {
                    let ui = SettingsWindow::new().unwrap();

                    // 現在の設定を UI に反映
                    {
                        let conf = config_ui.lock().unwrap();
                        ui.set_buffer_size(conf.buffer_size as i32);
                        ui.set_window_size(conf.window_size as i32);
                        ui.set_moving_threshold(conf.moving_threshold as f32);
                        ui.set_angle_threshold(conf.angle_threshold as f32);
                        ui.set_corner_threshold(conf.corner_threshold as f32);
                        ui.set_allow_diagonal(conf.allow_diagonal);
                        ui.set_debug_mode(conf.debug);
                    }

                    let ui_handle = ui.as_weak();
                    let config_save = Arc::clone(&config_ui);

                    ui.on_apply_settings(move || {
                        if let Some(ui) = ui_handle.upgrade() {
                            let mut conf = config_save.lock().unwrap();
                            conf.buffer_size = ui.get_buffer_size() as usize;
                            conf.window_size = ui.get_window_size() as usize;
                            conf.moving_threshold = ui.get_moving_threshold() as f64;
                            conf.angle_threshold = ui.get_angle_threshold() as f64;
                            conf.corner_threshold = ui.get_corner_threshold() as f64;
                            conf.allow_diagonal = ui.get_allow_diagonal();
                            conf.debug = ui.get_debug_mode();

                            if let Err(e) = conf.save(CONFIG_PATH) {
                                println!("Failed to save config: {:?}", e);
                            } else {
                                println!("Settings saved and applied.");
                                logger::set_enabled(conf.debug);
                            }
                            ui.hide().unwrap();
                        }
                    });

                    let ui_handle_close = ui.as_weak();
                    ui.on_close_window(move || {
                        if let Some(ui) = ui_handle_close.upgrade() {
                            ui.hide().unwrap();
                        }
                    });

                    ui.run().unwrap();
                });
            }
            TrayAction::Exit => {
                println!("Exiting...");
                break;
            }
            TrayAction::None => {}
        }
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
