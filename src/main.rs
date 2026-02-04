mod point;

use crate::point::Point;
use anyhow::Result;
use circular_queue::CircularQueue;
use colored::Colorize;
use rdev::{Button, EventType, listen};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use thousands::Separable;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    let mut start_time: Option<Instant> = None;
    let trails = Arc::new(Mutex::new(CircularQueue::with_capacity(1024)));

    let trails_clone = Arc::clone(&trails);
    if let Err(e) = listen(move |e| match e.event_type {
        EventType::ButtonPress(Button::Right) => {
            start_time = Some(Instant::now());
        }
        EventType::ButtonRelease(Button::Right) => {
            if let Some(start) = start_time
                && let Some(end) = Some(Instant::now())
            {
                let elapsed = end - start;
                println!(
                    "{} {:>6} ms | {:>4} items",
                    "elapsed".bright_green(),
                    elapsed.as_millis().separate_with_commas(),
                    if let Ok(q) = trails_clone.lock() {
                        q.len().separate_with_commas()
                    } else {
                        "-".into()
                    }
                );

                start_time = Option::None;

                if let Ok(mut q) = trails_clone.lock() {
                    q.clear();
                }
            }
        }
        EventType::MouseMove { x, y } => {
            // gesturing
            if start_time.is_some() {
                let point = Point::new(x, y);
                if let Ok(mut q) = trails_clone.lock() {
                    q.push(point);
                }
            }
        }
        _ => {}
    }) {
        println!("Error: {:?}", e);
    };

    if let Ok(mut q) = trails.lock() {
        q.clear();
    }

    Ok(())
}
