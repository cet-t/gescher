mod app_state;
mod config;
mod filters;
mod logger;
mod point;
mod tray;

use crate::config::{Config, Direction};
use crate::point::Point;
use crate::tray::TrayManager;
use anyhow::Result;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, grab, simulate};
use std::f64;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use thousands::Separable;

const CONFIG_PATH: &str = "config.yaml";

// 解析に必要なデータをまとめた型
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
    let config = serde_yaml::from_reader::<_, Config>(reader)?;
    let angle_threshold = config.angle_threshold().to_radians();

    logger::set_enabled(config.debug());

    let (tx, rx) = mpsc::channel::<GestureTask>();
    let config_analyze = config.clone();

    // --- 解析専用ワーカースレッド ---
    thread::spawn(move || {
        while let Ok(task) = rx.recv() {
            let win = config_analyze.window_size();
            let mut points = task.points;
            points.reverse();

            // 解析処理 (重い処理はここで行う)
            let denoised = denoise_vec(points, win);
            if denoised.len() < win * 2 {
                // 動いていない場合はクリックを再発行 (このスレッドから実行)
                app_state::add_ignore_count(2);
                let _ = simulate(&EventType::ButtonPress(Button::Right));
                let _ = simulate(&EventType::ButtonRelease(Button::Right));
                continue;
            }

            let dir_threshold = angle_threshold.sin();
            let mut gesture_directions = Vec::new();
            let mut current_direction = Direction::UNDEFINED;
            let mut last_v = Point::zero();

            for i in (0..denoised.len().saturating_sub(win)).step_by(win) {
                let p_start = denoised[i];
                let p_end = denoised[i + win];
                let seg_vec = p_end - p_start;
                if seg_vec.magnitude() > config_analyze.moving_threshold() {
                    let dir = seg_vec.direction(dir_threshold);
                    if dir != Direction::UNDEFINED {
                        if !current_direction.intersects(dir) || current_direction.is_empty() {
                            gesture_directions.push(dir);
                        }
                        current_direction = dir;
                    }
                    if last_v.magnitude() > f64::EPSILON {
                        let seg_angle = last_v.normalized().angle(seg_vec.normalized());
                        if seg_angle > angle_threshold {
                            // コーナー検出などのログ (必要なら)
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

    let trails_atm = Arc::new(Mutex::new(CircularQueue::<Point>::with_capacity(
        config.buffer_size(),
    )));
    let trails_clone = Arc::clone(&trails_atm);
    let start_time_mu = Arc::new(Mutex::new(Option::<Instant>::None));
    let last_point_mu = Arc::new(Mutex::new(Point::zero()));
    let st_clone = Arc::clone(&start_time_mu);
    let lp_clone = Arc::clone(&last_point_mu);

    // --- 入力監視スレッド (OSクリティカル) ---
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
                            // 解析スレッドに丸投げ (ここでの処理時間を最小化)
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
                            // 最小限のフィルタ
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
        if tray_manager.update() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
