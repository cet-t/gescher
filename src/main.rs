mod config;
mod filters;
mod point;

use crate::config::{Config, Direction};
use crate::point::Point;
use anyhow::Result;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, listen};
use std::f64;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use thousands::Separable;

const CONFIG_PATH: &str = "config.yaml";

#[inline]
fn denoise(points: CircularQueue<Point>, window_size: usize) -> Vec<Point> {
    let mut res = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let window = points.iter().skip(i).take(window_size).collect::<Vec<_>>();
        let actual_size = window.len() as f64;
        let value = window.into_iter().fold(Point::zero(), |acc, &p| acc + p) / actual_size;
        res.push(value);
    }
    res
}

fn main() -> Result<()> {
    let file = File::open(CONFIG_PATH)?;
    let reader = BufReader::new(&file);
    let config = serde_yaml::from_reader::<_, Config>(reader)?;
    let angle_threshold = config.angle_threshold().to_radians();

    println!(
        "{}",
        format!("*-------------- {} --------------*", "config".cyan()).bold()
    );
    println!(
        "{:<20} {:>6}",
        "buffer-size".green().bold(),
        config.buffer_size().separate_with_commas().bright_white()
    );

    println!(
        "{:<20} {:>6} ({:>3})",
        "angle-threshold".green().bold(),
        angle_threshold
            .to_degrees()
            .separate_with_commas()
            .bright_white(),
        angle_threshold
    );

    println!(
        "{:<20} {:>6}",
        "window-size".green().bold(),
        config.window_size().separate_with_commas().bright_white()
    );

    println!(
        "{:<20} {:>6}",
        "moving-threshold".green().bold(),
        config
            .moving_threshold()
            .separate_with_commas()
            .bright_white()
    );
    println!("{}", "*--------------------------------------*".bold());

    // println!(
    //     "{:<20} {}",
    //     "gestures".green().bold(),
    //     config
    //         .gestures
    //         .iter()
    //         .map(|g| format!(
    //             "{:<10} {}",
    //             format!("{:?}", g.gesture_type).bright_white(),
    //             g.control.white()
    //         ))
    //         .collect::<Vec<_>>()
    //         .join(&format!("\n{}", String::from_iter([' '; 12 + 1].iter())))
    // );

    let mut start_time: Option<Instant> = None;
    let trails_atm = Arc::new(Mutex::new(CircularQueue::<Point>::with_capacity(
        config.buffer_size(),
    )));

    let mut last_point = Point::zero(); //f64x2::from([0.0, 0.0]);

    let trails_atm_clone = Arc::clone(&trails_atm);
    if let Err(e) = listen(move |e| match e.event_type {
        EventType::ButtonPress(Button::Right) => {
            start_time = Some(Instant::now());
        }
        EventType::ButtonRelease(Button::Right) => {
            if let Some(start) = start_time {
                let elapsed = Instant::now() - start;

                if let Ok(mut q) = trails_atm_clone.lock() {
                    println!(
                        "{:<20} {:>6} {:>6} items",
                        "elapsed".bright_green().bold(),
                        elapsed.as_millis().separate_with_commas().bright_white(),
                        q.len().separate_with_commas().bright_white()
                    );

                    let points = denoise(q.clone(), config.window_size());

                    let &start0 = points.get(5).unwrap_or_default();
                    let &start1 = points.get(0).unwrap_or_default();
                    let start_norm = (start1 - start0).normalized();
                    let start_dir = (start1 - start0).direction(angle_threshold);
                    println!("start: {} direction: {}", start_norm, start_dir);

                    let &end0 = points.get(points.len() - 5).unwrap_or_default();
                    let &end1 = points.get(points.len() - 1).unwrap_or_default();
                    let end_norm = (end1 - end0).normalized();
                    let end_dir = (end1 - end0).direction(angle_threshold);
                    println!("end: {} direction: {}", end_norm, end_dir);

                    let angle = start_norm.angle(end_norm);
                    println!("angle: {}", angle.to_degrees());

                    let cross = start_norm.cross(end_norm);
                    println!("cross: {}", cross);

                    let is_rangle = point::is_rangle(angle, angle_threshold);
                    println!("is_rangle: {}", is_rangle);

                    // let is_start_up = start_norm.dot(Point::up()) < angle_threshold;
                    // let is_start_right = start_norm.dot(Point::right()) < angle_threshold;

                    match start_dir {
                        Direction::UP => {
                            println!("UP");
                        }
                        Direction::DOWN => {
                            println!("DOWN");
                        }
                        Direction::LEFT => {
                            println!("LEFT");
                        }
                        Direction::RIGHT => {
                            println!("RIGHT");
                        }
                        _ => {}
                    }

                    // if is_rangle {
                    //     let dir = match angle.signum() {
                    //         -1.0 => Direction::LEFT,
                    //         1.0 => Direction::RIGHT,
                    //         _ => Direction::UNDEFINED,
                    //     };

                    //     println!(
                    //         "{:<20} {:>6} {:>6}",
                    //         "rangle".bright_green().bold(),
                    //         angle.separate_with_commas().bright_white(),
                    //         dir.to_string().bright_white()
                    //     );
                    // } else {
                    //     let start = start_norm.normalized();
                    //     let end = end_norm.normalized();
                    //     let is_up = start.y() > 0.0 && end.y() > 0.0;
                    //     let is_down = start.y() < 0.0 && end.y() < 0.0;
                    //     let is_left = start.x() < 0.0 && end.x() < 0.0;
                    //     let is_right = start.x() > 0.0 && end.x() > 0.0;

                    //     let dir = if is_up {
                    //         Direction::UP
                    //     } else if is_down {
                    //         Direction::DOWN
                    //     } else if is_left {
                    //         Direction::LEFT
                    //     } else if is_right {
                    //         Direction::RIGHT
                    //     } else {
                    //         Direction::UNDEFINED
                    //     };

                    //     println!(
                    //         "{:<20} {:>6} {:>6}",
                    //         "straight".bright_green().bold(),
                    //         angle.separate_with_commas().bright_white(),
                    //         dir.to_string().bright_white()
                    //     );
                    // }

                    if let Ok(file) = File::create("trajectory.csv") {
                        let mut writer = BufWriter::new(file);
                        points.iter().for_each(|&p| {
                            let _ = writeln!(writer, "{}", p);
                        });
                    }

                    q.clear();
                }

                start_time = Option::None;
            }
        }
        EventType::MouseMove { x, y } => {
            let point = Point::new(x, -y);
            let distance = (last_point - point).magnitude();

            if start_time.is_some() && distance > config.moving_threshold() {
                last_point = point;
                if let Ok(mut q) = trails_atm_clone.lock() {
                    if let Some(_) = q.push(point.into()) {
                        println!("popped");
                    }
                }
            }
        }
        _ => {}
    }) {
        println!("Error: {:?}", e);
    };

    Ok(())
}
