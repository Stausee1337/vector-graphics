use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn lerp(self, other: Self, t: f32) -> Self { 
        self + (other - self)*t.clamp(0.0, 1.0)
    }

    pub fn norm(self) -> Vec2 {
        self/self.length()
    }

    pub fn turn90(self) -> Vec2 {
        Vec2 { x: -self.y, y: self.x }
    }

    pub fn is_nan(self) -> bool {
        self.x.is_nan() || self.y.is_nan()
    }

    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - other.x * self.y
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        Vec2 { x: -self.x, y: -self.y }
    }
}


impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, lambda: f32) -> Vec2 {
        Vec2 { x: self.x * lambda, y: self.y * lambda }
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, rhs: f32) -> Vec2 {
        self * rhs.recip()
    }
}

