use bitflags::bitflags;
use core::fmt;
use serde::{Deserialize, Serialize};

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
    #[serde(alias = "type", rename = "trigger")]
    pub trigger: String,

    #[serde(alias = "control")]
    pub control: Option<String>,

    #[serde(alias = "command")]
    pub command: Option<String>,

    #[serde(alias = "target")]
    pub target_bin: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(alias = "buffer-size")]
    buffer_size: usize,

    #[serde(alias = "window-size")]
    window_size: usize,

    #[serde(alias = "moving-threshold")]
    moving_threshold: f64,

    #[serde(alias = "angle-threshold")]
    angle_threshold: f64,

    #[serde(alias = "corner-threshold")]
    corner_threshold: f64,

    #[serde(alias = "allow-diagonal")]
    allow_diagonal: bool,

    #[serde(alias = "debug")]
    debug: bool,

    #[serde(alias = "gesture")]
    gestures: Vec<Gesture>,
}

#[allow(unused)]
impl Config {
    #[inline]
    pub const fn debug(&self) -> bool {
        self.debug
    }

    #[inline]
    pub const fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    #[inline]
    pub const fn window_size(&self) -> usize {
        self.window_size
    }

    #[inline]
    pub const fn moving_threshold(&self) -> f64 {
        self.moving_threshold
    }

    #[inline]
    pub const fn angle_threshold(&self) -> f64 {
        self.angle_threshold
    }

    #[inline]
    pub const fn corner_threshold(&self) -> f64 {
        self.corner_threshold
    }

    #[inline]
    pub const fn allow_diagonal(&self) -> bool {
        self.allow_diagonal
    }

    #[inline]
    pub const fn gestures(&self) -> &Vec<Gesture> {
        &self.gestures
    }
}
