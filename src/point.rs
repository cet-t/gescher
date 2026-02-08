use crate::config::Direction;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline]
    pub const fn x(&self) -> f64 {
        self.x
    }

    #[inline]
    pub const fn y(&self) -> f64 {
        self.y
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<f64> for Point {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl std::ops::Div<f64> for Point {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f64) -> Self {
        Self {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

impl Point {
    #[inline]
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag > 0.0 { *self / mag } else { Self::zero() }
    }

    #[inline]
    pub fn dot(&self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn cross(&self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    #[inline]
    pub fn angle(&self, other: Self) -> f64 {
        let dot = self.dot(other);
        let mag_product = self.magnitude() * other.magnitude();
        if mag_product > 0.0 {
            (dot / mag_product).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }

    pub fn direction(&self, threshold: f64) -> Direction {
        let mut dir = Direction::UNDEFINED;
        let norm_v = self.normalized();
        if norm_v.y > threshold {
            dir |= Direction::UP;
        }
        if norm_v.y < -threshold {
            dir |= Direction::DOWN;
        }
        if norm_v.x > threshold {
            dir |= Direction::RIGHT;
        }
        if norm_v.x < -threshold {
            dir |= Direction::LEFT;
        }
        dir
    }
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::zero()
    }
}
