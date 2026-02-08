use bitflags::bitflags;
use core::fmt;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct Direction: u8 {
        const UNDEFINED = 0b00000000;
        const UP =        0b00000001;
        const DOWN =      0b00000010;
        const LEFT =      0b00000100;
        const RIGHT =     0b00001000;
    }
}

impl Direction {
    pub fn from_flags(flags: u8) -> Self {
        match flags {
            0b00000001 => Self::UP,
            0b00000101 => Self::UP | Self::LEFT,
            0b00001001 => Self::UP | Self::RIGHT,

            0b00000010 => Self::DOWN,
            0b00000110 => Self::DOWN | Self::LEFT,
            0b00001010 => Self::DOWN | Self::RIGHT,

            0b00000100 => Self::LEFT,

            0b00001000 => Self::RIGHT,

            _ => Self::UNDEFINED,
        }
    }
}

impl fmt::Display for Direction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        if self.contains(Self::UP) {
            s.push_str("Up");
        }
        if self.contains(Self::DOWN) {
            s.push_str("Down");
        }
        if self.contains(Self::LEFT) {
            s.push_str("Left");
        }
        if self.contains(Self::RIGHT) {
            s.push_str("Right");
        }

        write!(f, "{}", s)
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
    #[serde(alias = "buffer-size")]
    buffer_size: usize,

    #[serde(alias = "window-size")]
    window_size: usize,

    #[deprecated]
    #[serde(alias = "sampling-rate")]
    sampling_rate: u32,

    #[serde(alias = "moving-threshold")]
    moving_threshold: f64,

    #[serde(alias = "angle-threshold")]
    angle_threshold: f64,

    #[serde(alias = "gesture")]
    gestures: Vec<Gesture>,
}

#[allow(unused)]
impl Config {
    #[inline]
    pub const fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    #[inline]
    pub const fn window_size(&self) -> usize {
        self.window_size
    }

    #[deprecated]
    #[inline]
    pub const fn sample_rate(&self) -> u32 {
        self.sampling_rate
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
    pub const fn gestures(&self) -> &Vec<Gesture> {
        &self.gestures
    }
}
