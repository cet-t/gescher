use bitflags::bitflags;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

bitflags! {
    #[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
    pub struct Direction: u8 {
        const UNDEFINED = 0b00000000;
        const UP =        0b00000001;
        const DOWN =      0b00000010;
        const LEFT =      0b00000100;
        const RIGHT =     0b00001000;
    }
}

impl fmt::Display for Direction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.contains(Self::UP) {
            parts.push("UP");
        }
        if self.contains(Self::DOWN) {
            parts.push("DOWN");
        }
        if self.contains(Self::LEFT) {
            parts.push("LEFT");
        }
        if self.contains(Self::RIGHT) {
            parts.push("RIGHT");
        }

        if parts.is_empty() {
            write!(f, "UNDEFINED")
        } else {
            write!(f, "{}", parts.join(" | "))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Gesture {
    #[serde(alias = "type")]
    pub gesture_type: String,

    #[serde(alias = "control")]
    pub control: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(alias = "buffer-size", rename = "buffer-size")]
    pub buffer_size: usize,

    #[serde(alias = "window-size", rename = "window-size")]
    pub window_size: usize,

    #[serde(alias = "moving-threshold", rename = "moving-threshold")]
    pub moving_threshold: f64,

    #[serde(alias = "angle-threshold", rename = "angle-threshold")]
    pub angle_threshold: f64,

    #[serde(alias = "corner-threshold", rename = "corner-threshold")]
    pub corner_threshold: f64,

    #[serde(alias = "allow-diagonal", rename = "allow-diagonal")]
    pub allow_diagonal: bool,

    #[serde(alias = "debug", rename = "debug")]
    pub debug: bool,

    #[serde(alias = "language", rename = "language", default = "default_language")]
    pub language: String,

    #[serde(alias = "gesture", rename = "gesture")]
    pub gestures: Vec<Gesture>,
}

fn default_language() -> String {
    "en".to_string()
}

impl Config {
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        let mut file = File::create(path)?;
        file.write_all(yaml.as_bytes())?;
        Ok(())
    }
}
