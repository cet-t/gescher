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
    pub fn angle(&self, other: Self) -> f64 {
        let dot = self.dot(other);
        let mag_product = self.magnitude() * other.magnitude();
        if mag_product > 0.0 {
            (dot / mag_product).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }

    pub fn direction(&self, threshold: f64, allow_diagonal: bool) -> Direction {
        let mut dir = Direction::UNDEFINED;
        let norm_v = self.normalized();

        if allow_diagonal {
            // 斜めを許可する場合：両方のフラグが立つ可能性がある
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
        } else {
            // 斜めを禁止する場合：
            // 45度付近の「どっちつかず」な方向を UNDEFINED にするため、
            // 支配的な成分がもう一方の成分に対して十分大きいかチェックする。
            // 角度しきい値 (threshold = sin(angle)) を利用
            let x_abs = norm_v.x.abs();
            let y_abs = norm_v.y.abs();

            // Xが支配的かつ、Y成分がしきい値未満（＝ほぼ水平）
            if x_abs > y_abs && y_abs < threshold {
                if norm_v.x > 0.0 {
                    dir = Direction::RIGHT;
                } else {
                    dir = Direction::LEFT;
                }
            }
            // Yが支配的かつ、X成分がしきい値未満（＝ほぼ垂直）
            else if y_abs > x_abs && x_abs < threshold {
                if norm_v.y > 0.0 {
                    dir = Direction::UP;
                } else {
                    dir = Direction::DOWN;
                }
            }
            // それ以外（中途半端な斜め）は UNDEFINED になり、
            // main.rs のループで「方向なし」として無視される。
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
