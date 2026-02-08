use std::f64::consts::PI;
use std::fmt;
use wide::f64x2;

use crate::config::Direction;

pub const RAD_180: f64 = PI;
pub const RAD_135: f64 = RAD_180 * (3. / 4.);
pub const RAD_90: f64 = RAD_180 / 2.0;
pub const RAD_60: f64 = RAD_180 * (1. / 3.);
pub const RAD_45: f64 = RAD_180 / 4.0;
pub const RAD_30: f64 = RAD_180 / 6.0;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    x: f64,
    y: f64,
}

#[macro_export]
macro_rules! point {
    ($x:expr, $y:expr) => {
        $crate::Point { x: $x, y: $y }
    };

    ($x2:expr) => {{
        use wide::f64x2;
        let [x, y] = f64x2::from($x2).as_array();
        $crate::Point { x: *x, y: *y }
    }};
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn x(&self) -> f64 {
        self.x
    }

    #[inline]
    pub const fn y(&self) -> f64 {
        self.y
    }

    #[inline]
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        *self / self.magnitude()
    }

    #[inline]
    pub fn direction(&self, threshold: f64) -> Direction {
        let mut dir = Direction::UNDEFINED;

        let norm_v = self.normalized();
        if norm_v.y() > threshold {
            dir |= Direction::UP;
        }
        if norm_v.y() < -threshold {
            dir |= Direction::DOWN;
        }
        if norm_v.x() > threshold {
            dir |= Direction::RIGHT;
        }
        if norm_v.x() < -threshold {
            dir |= Direction::LEFT;
        }

        dir
    }

    #[inline]
    pub const fn dot(&self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub const fn cross(&self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    #[inline]
    pub fn angle(&self, other: Self) -> f64 {
        (self.dot(other) / self.magnitude().max(f64::EPSILON) / other.magnitude().max(f64::EPSILON))
            .acos()
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline]
    pub const fn up() -> Self {
        Self { x: 0.0, y: 1.0 }
    }

    #[inline]
    pub const fn down() -> Self {
        Self { x: 0.0, y: -1.0 }
    }

    #[inline]
    pub const fn left() -> Self {
        Self { x: -1.0, y: 0.0 }
    }

    #[inline]
    pub const fn right() -> Self {
        Self { x: 1.0, y: 0.0 }
    }

    #[inline]
    pub const fn epsilon() -> Self {
        Self {
            x: f64::EPSILON,
            y: f64::EPSILON,
        }
    }
}

#[inline]
pub const fn is_rangle(angle: f64, angle_threshold: f64) -> bool {
    (angle - RAD_90).abs() < angle_threshold
}

#[inline]
pub const fn is_right(angle: f64) -> bool {
    matches!(angle.signum(), -1.0)
}

impl Default for Point {
    fn default() -> Self {
        Self::zero()
    }
}

impl Default for &Point {
    fn default() -> Self {
        static ZERO: Point = Point::zero();
        &ZERO
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

impl std::ops::Neg for Point {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Add for &Point {
    type Output = Point;

    fn add(self, other: Self) -> Self::Output {
        Self::Output {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self + -other
    }
}

impl std::ops::Sub for &Point {
    type Output = Point;

    fn sub(self, other: Self) -> Self::Output {
        Self::Output {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<Point> for Point {
    type Output = Self;

    fn mul(self, other: Point) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl std::ops::Mul<f64> for Point {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl std::ops::Div<f64> for Point {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
        }
    }
}
