use std::ops::{Add, Div, Mul, Neg, Sub};

pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: u32,
    pub height: u32,
    pub stride: u32
}

impl<'a> Canvas<'a> {
    pub fn from_raw_pixels(pixels: &'a mut [u32], width: usize, height: usize, stride: usize) -> Canvas<'a> {
        assert!(pixels.len() >= width * height);
        Canvas { 
            pixels,
            width: i32::try_from(width).unwrap() as u32,
            height: i32::try_from(height).unwrap() as u32,
            stride: u32::try_from(stride).unwrap()
        }
    }

    pub fn clear(&mut self) {
        let (_, height, stride) = (self.width as usize, self.height as usize, self.stride as usize);
        for y in 0..height {
            let line = &mut self.pixels[y * stride..(y + 1)*stride];
            line.fill(0);
        }
    }

    pub fn width(&self) -> i32 {
        self.width as i32
    }

    pub fn height(&self) -> i32 {
        self.height as i32
    }

    pub fn at(&self, x: i32, y: i32) -> &Color {
        assert!(x >= 0 && (x as u32) < self.width);
        assert!(y >= 0 && (y as u32) < self.height);
        bytemuck::cast_ref(&self.pixels[y as usize * self.stride as usize + x as usize])
    }

    pub fn at_mut(&mut self, x: i32, y: i32) -> &mut Color {
        assert!(x >= 0 && (x as u32) < self.width);
        assert!(y >= 0 && (y as u32) < self.height);
        bytemuck::cast_mut(&mut self.pixels[y as usize * self.stride as usize + x as usize])
    }
}

struct Hermite {
    p0: Vec2,
    v0: Vec2,
    p1: Vec2,
    v1: Vec2
}

impl Hermite {
    fn as_cubic(&self) -> Cubic {
        // H(t)        =        p0 B0(t) +   (p0 + (1/3)v0) B1(t) +   (p1 - (1/3)v1) B2(t) +        p1 B3(t)
        Cubic {
            p0: self.p0,
            p1: self.p0 + self.v0/3.0,
            p2: self.p1 - self.v1/3.0,
            p3: self.p1,
        }
    }
}

pub trait Scalar {
    fn sq(self) -> Self;
}

impl Scalar for f32 {
    fn sq(self) -> Self {
        self * self
    }
}

struct OffsetCubic {
    orig: Cubic,
    approx: Cubic,
    offset: f32,
    coeffs: [f32; 7],
    deriv1_coeffs: [f32; 6],
    deriv2_coeffs: [f32; 5],
}

impl OffsetCubic {
    fn eval_curve_and_derivative(cubic: &Cubic, offset: f32, t: f32) -> (Vec2, Vec2) {
        let derivative = cubic.derivative().evaluate(t);
        let point = cubic.evaluate(t) + Vec2::new(derivative.y, -derivative.x).norm() * offset;
        let deriv = derivative * (1.0 + offset * cubic.curvature(t));
        (point, deriv)
    }

    fn form_cubic_and_offset(cubic: Cubic, offset: f32) -> Self {
        let (p0, v0) = Self::eval_curve_and_derivative(&cubic, offset, 0.0);
        let (p1, v1) = Self::eval_curve_and_derivative(&cubic, offset, 1.0);

        let hermite = Hermite { p0, p1, v0, v1 };

        let i0 = cubic.p0 - hermite.p0;
        let i1 = cubic.p1 - hermite.p0 - hermite.v0/3.0;
        let i2 = cubic.p2 - hermite.p1 + hermite.v1/3.0;
        let i3 = cubic.p3 - hermite.p1;

        let Vec2 { x: x0, y: y0 } = i0;
        let Vec2 { x: x1, y: y1 } = i1;
        let Vec2 { x: x2, y: y2 } = i2;
        let Vec2 { x: x3, y: y3 } = i3;

        let coeffs = [
            x0.sq() + y0.sq(),
            x0*x1 + y0*y1,
            0.4*x0*x2 + 0.6*x1.sq() + 0.4*y0*y2 + 0.6*y1.sq(),
            0.1*x0*x3 + 0.9*x1*x2 + 0.1*y0*y3 + 0.9*y1*y2,
            0.4*x1*x3 + 0.6*x2.sq() + 0.4*y1*y3 + 0.6*y2.sq(),
            x2*x3 + y2*y3,
            x3.sq() + y3.sq(),
        ];
    
        let deriv1_coeffs = [
            coeffs[1] - coeffs[0],
            coeffs[2] - coeffs[1],
            coeffs[3] - coeffs[2],
            coeffs[4] - coeffs[3],
            coeffs[5] - coeffs[4],
            coeffs[6] - coeffs[5]
        ];

        let deriv2_coeffs = [
            deriv1_coeffs[1] - deriv1_coeffs[0],
            deriv1_coeffs[2] - deriv1_coeffs[1],
            deriv1_coeffs[3] - deriv1_coeffs[2],
            deriv1_coeffs[4] - deriv1_coeffs[3],
            deriv1_coeffs[5] - deriv1_coeffs[4],
        ];


        Self { orig: cubic, approx: hermite.as_cubic(), offset, coeffs, deriv1_coeffs, deriv2_coeffs }
    }

    fn eval(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu*uu;
        let b1 = 6.0*uu*uu*u*t;
        let b2 = 15.0*uu*uu*tt;
        let b3 = 20.0*uu*u*tt*t;
        let b4 = 15.0*uu*tt*tt;
        let b5 = 6.0*u*tt*tt*t;
        let b6 = tt*tt*tt;

        b0 * self.coeffs[0]
            + b1 * self.coeffs[1]
            + b2 * self.coeffs[2]
            + b3 * self.coeffs[3]
            + b4 * self.coeffs[4]
            + b5 * self.coeffs[5]
            + b6 * self.coeffs[6]
    }

    fn eval_deriv1(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu*u;
        let b1 = 5.0*uu*uu*t;
        let b2 = 10.0*uu*u*tt;
        let b3 = 10.0*uu*t*tt;
        let b4 = 5.0*u*tt*tt;
        let b5 = tt*tt*t;

        6.0 * (
            b0 * self.deriv1_coeffs[0]
            + b1 * self.deriv1_coeffs[1]
            + b2 * self.deriv1_coeffs[2]
            + b3 * self.deriv1_coeffs[3]
            + b4 * self.deriv1_coeffs[4]
            + b5 * self.deriv1_coeffs[5]
        )
    }

    fn eval_deriv2(&self, t: f32) -> f32 {
        let u = 1.0 - t;
        let tt = t.sq();
        let uu = u.sq();

        let b0 = uu*uu;
        let b1 = 4.0*uu*u*t;
        let b2 = 6.0*uu*tt;
        let b3 = 4.0*u*t*tt;
        let b4 = tt*tt;

        6.0 * 5.0 * (
            b0 * self.deriv2_coeffs[0]
            + b1 * self.deriv2_coeffs[1]
            + b2 * self.deriv2_coeffs[2]
            + b3 * self.deriv2_coeffs[3]
            + b4 * self.deriv2_coeffs[4]
        )
    }

    fn eval_error(&self, t: f32) -> f32 {
        (self.eval(t).abs() - self.offset.sq()).abs()
    }

    fn compute_max_error(&self) -> Option<(f32, f32)> {
        const N_SAMPLES: usize = 51;
        let mut samples: [f32; N_SAMPLES] = [0.0; N_SAMPLES];
        let dt = 1.0 / (N_SAMPLES - 1) as f32;
        for i in 0..N_SAMPLES {
            samples[i] = self.eval_deriv1(dt * i as f32);
        }
    

        let mut i = 0;
        let mut x0s = vec![];
        while i < N_SAMPLES - 1 {
            let s0 = samples[i];
            let s1 = samples[i + 1];
            if s0 * s1 <= 0.0 {
                let t0 = i as f32 * dt;
                x0s.push(t0 + dt*0.5);
            }
            i += 1;
        }

        if x0s.is_empty() { return None; }

        let mut local_max_err = (-1.0, 0.0);
        for x0 in x0s {
            let t_max = self.solve_newton(x0);
            let max = (t_max, self.eval_error(t_max));

            if max.1 > local_max_err.1 {
                local_max_err = max;
            }
        }

        Some(local_max_err)
    }

    fn solve_newton(&self, x0: f32) -> f32 {
        const EPSILON: f32 = 1e-3;
        const MAX_ITER: usize = 4;

        let mut x = x0;
        let mut y;
        let mut y_dot;
        let mut i = 0;
        loop {
            y = self.eval_deriv1(x);
            y_dot = self.eval_deriv2(x);
            x = x - y/y_dot;
            i += 1;

            if y.abs() < EPSILON || i >= MAX_ITER {
                return x;
            }
        }
    }

    fn split(&self, t_subdiv: f32) -> (OffsetCubic, OffsetCubic) {
        let (cubic1, cubic2) = self.orig.split_at(t_subdiv);
        // let cubic1 = self.orig.subsegment(0.0, t_subdiv);
        // let cubic2 = self.orig.subsegment(t_subdiv, 1.0);
        // TODO: can probably be optimized by the information we already have
        let oc1 = Self::form_cubic_and_offset(cubic1, self.offset);
        let oc2 = Self::form_cubic_and_offset(cubic2, self.offset);
        (oc1, oc2)
    }
}

struct Approx<'a> {
    dest: &'a mut Path,
}

impl<'a> Approx<'a> {
    const MAX_SUBDIV: usize = 10;

    fn approx(dest: &'a mut Path, source: Cubic, offset: f32) {
        let oc = OffsetCubic::form_cubic_and_offset(source, offset);
        let mut approx = Approx { dest };

        approx.make_offset_recursively(oc, 0);
    }

    fn make_offset_recursively(&mut self, oc: OffsetCubic, level: usize) {
        let (t_subdiv, max_err) = self.get_max_error(&oc);
        if max_err < 1.0 || level >= Self::MAX_SUBDIV {
            self.dest.push_cubic(&oc.approx);
            return;
        }
        let (oc1, oc2) = oc.split(t_subdiv);
        self.make_offset_recursively(oc1, level + 1);
        self.make_offset_recursively(oc2, level + 1);
    }

    fn get_max_error(&self, oc: &OffsetCubic) -> (f32, f32) {
        if let Some((t_subdiv, max_err)) = oc.compute_max_error() {
            return (t_subdiv, max_err.sqrt());
        }
        (0.5, oc.eval_error(0.5).sqrt())
    }
}

pub fn get_offset_curve(dest: &mut Path, cubic: Cubic, offset: f32) {
    // let oc = OffsetCubic::form_cubic_and_offset(cubic.clone(), offset);
    // let p_max_error = if let Some((t_max, max_err)) = oc.compute_max_error() {
    //     println!("{t_max}, {}", max_err.sqrt());
    //     Some(oc.approx.evaluate(t_max))
    // } else {
    //     None
    // };
    // // println!("{max_err}");
    // // let t_max = 0.0;
    // (oc.approx, p_max_error)
    Approx::approx(dest, cubic, offset)
}

#[derive(Clone, Copy)]
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

    pub fn lerp(self, other: Self, t: f32) -> Self { 
        self + (other - self)*t.clamp(0.0, 1.0)
    }

    pub fn norm(self) -> Vec2 {
        self/self.length()
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

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        Vec2 { x: -self.x, y: -self.y }
    }
}

#[derive(Clone, Copy)]
pub struct Affine([f32; 6]);

impl Affine {
    pub const IDENTITY: Affine = Affine::scale(1.0);

    pub const fn new(coeffs: [f32; 6]) -> Affine {
        Affine(coeffs)
    }

    pub const fn scale(scale: f32) -> Affine {
        Affine([scale, 0.0, 0.0, scale, 0.0, 0.0])
    }

    pub const fn translate(translation: Vec2) -> Affine {
        Affine([1.0, 0.0, 0.0, 1.0, translation.x, translation.y])
    }

    pub fn rotate(angle: f32) -> Affine {
        let (s, c) = angle.sin_cos();
        Affine([c, -s, s, c, 0.0, 0.0])
    }
}

impl Mul for Affine {
    type Output = Affine;

    fn mul(self, rhs: Self) -> Self::Output {
        // |a b e|   |t u x|
        // |c d f| * |v w y|
        // |0 0 1|   |0 0 1|
        // 
        // |at + bv  au + bw  ax + by + e|
        // |ct + dv  cu + dw  cx + dy + f|
        // |   0        0          1     |

        Self::new([
            self.0[0] * rhs.0[0] + self.0[1] * rhs.0[2],
            self.0[0] * rhs.0[1] + self.0[1] * rhs.0[3],
            self.0[2] * rhs.0[0] + self.0[3] * rhs.0[2],
            self.0[2] * rhs.0[1] + self.0[3] * rhs.0[3],
            self.0[0] * rhs.0[4] + self.0[1] * rhs.0[5] + self.0[4],
            self.0[2] * rhs.0[4] + self.0[3] * rhs.0[5] + self.0[5],
        ])
    }
}

impl Mul<Vec2> for Affine {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        // |a b e|   |x|
        // |c d f| * |y|
        // |0 0 0|   |1|
        // 
        // |ax + by + e|
        // |cx + dy + f|
        // |     0     |
        Vec2 {
            x: self.0[0] * rhs.x + self.0[1] * rhs.y + self.0[4],
            y: self.0[2] * rhs.x + self.0[3] * rhs.y + self.0[5],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, bytemuck::Pod)]
#[repr(transparent)]
pub struct Color(u32);

unsafe impl bytemuck::Zeroable for Color { }

impl Color {
    pub const fn new(color: u32) -> Color {
        Color(color)
    }

    pub const fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color((alpha as u32) << 0o30 | (red as u32) << 0o20 | (green as u32) << 0o10 | blue as u32)
    }

    pub const fn alpha(self) -> u8 {
        (self.0 >> 0o30) as u8
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 0o20) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 0o10) as u8
    }

    pub const fn blue(self) -> u8 {
        (self.0 >> 0o00) as u8
    }

    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self((self.0 & 0x00FFFFFF) | (alpha as u32) << 0o30)
    }

    pub fn blend(self, other: Color) -> Color {
        let a1 = self.alpha();
        let r1 = self.red()   as u32;
        let g1 = self.green() as u32;
        let b1 = self.blue()  as u32;

        let a2 = other.alpha() as u32;
        let r2 = other.red()   as u32;
        let g2 = other.green() as u32;
        let b2 = other.blue()  as u32;

        let r = ((r1*(255 - a2) + r2*a2)/255).min(255) as u8;
        let g = ((g1*(255 - a2) + g2*a2)/255).min(255) as u8;
        let b = ((b1*(255 - a2) + b2*a2)/255).min(255) as u8;

        Color::from_rgba(r, g, b, a1)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

pub mod colors {
    use super::Color;

    pub const WHITE: Color = Color::new(0xffffffff);
    pub const GRAY : Color = Color::new(0xff7f7f7f);
    pub const RED  : Color = Color::new(0xffff0000);
    pub const LIME : Color = Color::new(0xff00ff00);
    pub const AQUA : Color = Color::new(0xff00ffff);
}

pub struct Line {
    p0: Vec2,
    p1: Vec2,
}

impl Line {
    pub fn new(p0: Vec2, p1: Vec2) -> Self {
        Line { p0, p1 }
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        self.p0.lerp(self.p1, t)
    }
}

pub struct Quadratic {
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
}

impl Quadratic {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Quadratic { p0, p1, p2 }
    }

    pub fn error(&self) -> f32 {
        let control = self.p1 - self.p0;
        let chord  = self.p2 - self.p0;

        let lambda = (control.dot(chord)/chord.dot(chord)).clamp(0.0, 1.0);
        let control_flat = chord * lambda;

        0.5 * (control - control_flat).length()
    }

    pub fn split(&self) -> (Quadratic, Quadratic) {
        let split_point = self.evaluate(0.5);
        let control1 = self.p0.lerp(self.p1, 0.5);
        let control2 = self.p1.lerp(self.p2, 0.5);
        (Quadratic::new(self.p0, control1, split_point), Quadratic::new(split_point, control2, self.p2))
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        let s = 1.0 - t;
        self.p0*s*s + self.p1*2.0*s*t + self.p2*t*t
    }

    pub fn derivative(&self) -> Line {
        Line::new((self.p1 - self.p0)*2.0, (self.p2 - self.p1)*2.0)
    }

    pub fn flatten(&self, points: &mut Vec<Vec2>) {
        if self.error() <= 0.25 {
            points.push(self.p0);
            points.push(self.p1);
            points.push(self.p2);
            return;
        }

        let (q0, q1) = self.split();
        q0.flatten(points);
        q1.flatten(points);
    }
}

#[derive(Clone)]
pub struct Cubic {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
}

impl Cubic {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Cubic {
        Cubic { p0, p1, p2, p3 }
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        let t2 = t * t;
        let u = 1.0 - t;
        let u2 = u * u;
        self.p0*u2*u + self.p1*3.0*u2*t + self.p2*3.0*u*t2 + self.p3*t2*t
    }

    pub fn derivative(&self) -> Quadratic {
        Quadratic::new((self.p1 - self.p0)*3.0, (self.p2 - self.p1)*3.0, (self.p3 - self.p2)*3.0)
    }

    pub fn curvature(&self, t: f32) -> f32 {
        let derivative1 = self.derivative();
        let derivative2 = derivative1.derivative();

        let d1 = derivative1.evaluate(t);
        let d2 = derivative2.evaluate(t);

        // κ = (x'y'' - y'x'')/(x'² + y'²)^(3 / 2)

        let num = d1.x * d2.y - d1.y * d2.x;
        let denom = d1.x.hypot(d1.y);
        num/(denom * denom * denom)
    }

    pub fn split(&self) -> (Cubic, Cubic) {
        let split_point = self.evaluate(0.5);
        let s = self.p1.lerp(self.p2, 0.5);
        let q1 = self.p0.lerp(self.p1, 0.5);
        let q2 = q1.lerp(s, 0.5);
        let r2 = self.p2.lerp(self.p3, 0.5);
        let r1 = r2.lerp(s, 0.5);

        (Cubic::new(self.p0, q1, q2, split_point), Cubic::new(split_point, r1, r2, self.p3))
    }

    pub fn split_at(&self, t: f32) -> (Cubic, Cubic) {
        let split_point = self.evaluate(t);
        let s = self.p1.lerp(self.p2, t);
        let q1 = self.p0.lerp(self.p1, t);
        let q2 = q1.lerp(s, t);
        let r2 = self.p2.lerp(self.p3, t);
        let r1 = s.lerp(r2, t);

        (Cubic::new(self.p0, q1, q2, split_point), Cubic::new(split_point, r1, r2, self.p3))
    }

    pub fn error(&self) -> f32 {
        let control1 = self.p1 - self.p0;
        let control2 = self.p2 - self.p0;
        let chord  = self.p3 - self.p0;
        let len_squared = chord.dot(chord);

        let get_dist = |control: Vec2| -> f32 {
            let lambda = (control.dot(chord)/len_squared).clamp(0.0, 1.0);
            let control_flat = chord * lambda;
            (control - control_flat).length()
        };

        0.5 * get_dist(control1).max(get_dist(control2))
    }

    pub fn flatten(&self, points: &mut Vec<Vec2>) {
        fn inner(curve: &Cubic, points: &mut Vec<Vec2>, level: usize) {
            if curve.error() <= 0.25 || level > 10 {
                points.push(curve.p0);
                points.push(curve.p1);
                points.push(curve.p2);
                points.push(curve.p3);
                return;
            }

            let (q0, q1) = curve.split();
            inner(&q0, points, level + 1);
            inner(&q1, points, level + 1);
        }
        inner(self, points, 0);
    }
}

#[derive(Default)]
pub struct Path {
    elements: Vec<PathElement>
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn push_cubic(&mut self, cubic: &Cubic) {
        if self.elements.is_empty() {
            self.elements.push(PathElement::MoveTo(cubic.p0));
        } else {
            self.elements.push(PathElement::LineTo(cubic.p0));
        }
        self.elements.push(PathElement::CurveTo(cubic.p1, cubic.p2, cubic.p3));
    }

    pub fn push(&mut self, element: PathElement) {
        if !matches!(element, PathElement::MoveTo(..)) && self.elements.is_empty() {
            panic!("push of non-MoveTo element into empty path");
        }
        self.elements.push(element);
    }

    pub fn subpaths(&self) -> impl Iterator<Item = &[PathElement]> {
        let mut i = 0;
        std::iter::from_fn(move || {
            if i >= self.elements.len() {
                return None;
            }
            
            let start = i;
            i += 1;
            while i < self.elements.len() && !matches!(self.elements[i], PathElement::MoveTo(..)) {
                i += 1;
            }
            Some(&self.elements[start..i])
        })
    }
}

#[derive(Clone, Copy)]
pub enum PathElement {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo(Vec2, Vec2),
    CurveTo(Vec2, Vec2, Vec2),
    Close
}

pub fn draw_path(canvas: &mut Canvas, path: &Path, transform: Affine, color: Color) {
    let mut points = vec![];
    let mut runs = vec![];

    for subpath in path.subpaths() {
        if subpath.len() < 2 { continue; }

        // NOTE: instead of transforming all of the points during flattening (which I guess is
        // fine, though), we could multiply the error threshold in the flattening by the transforms
        // scale obtained through single value decomposition

        let PathElement::MoveTo(mut current_position) = subpath[0] else { unreachable!() };
        current_position = transform * current_position;

        let start = points.len();
        for element in subpath {
            match element {
                &PathElement::LineTo(endpoint) => {
                    let tendpoint = transform * endpoint;
                    points.push(current_position);
                    points.push(tendpoint);
                    current_position = tendpoint;
                },
                &PathElement::QuadTo(control_point, endpoint) => {
                    let tendpoint = transform * endpoint;
                    let quadratic = Quadratic::new(current_position, transform * control_point, tendpoint);
                    quadratic.flatten(&mut points);
                    current_position = tendpoint;
                },
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    let tendpoint = transform * endpoint;
                    let cubic = Cubic::new(current_position, transform * control1, transform * control2, tendpoint);
                    cubic.flatten(&mut points);
                    current_position = tendpoint;
                }
                PathElement::Close => {
                    if points.len() > start {
                        runs.push((start, points.len() - 1));
                    }
                    break;
                },
                PathElement::MoveTo(..) => (),
            }
        }
    }

    primitives::polygon(canvas, &points, &runs, color);
}

pub fn draw_path_hairline(canvas: &mut Canvas, path: &Path, transform: Affine, color: Color) {
    let mut points = vec![];

    for subpath in path.subpaths() {
        if subpath.len() < 2 { continue; }

        // NOTE: instead of transforming all of the points during flattening (which I guess is
        // fine, though), we could multiply the error threshold in the flattening by the transforms
        // scale obtained through single value decomposition
        points.clear();

        let PathElement::MoveTo(mut current_position) = subpath[0] else { unreachable!() };
        current_position = transform * current_position;

        let start = current_position;
        for element in subpath {
            match element {
                &PathElement::LineTo(endpoint) => {
                    let tendpoint = transform * endpoint;
                    points.push(current_position);
                    points.push(tendpoint);
                    current_position = tendpoint;
                },
                &PathElement::QuadTo(control_point, endpoint) => {
                    let tendpoint = transform * endpoint;
                    let quadratic = Quadratic::new(current_position, transform * control_point, tendpoint);
                    quadratic.flatten(&mut points);
                    current_position = tendpoint;
                },
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    let tendpoint = transform * endpoint;
                    let cubic = Cubic::new(current_position, transform * control1, transform * control2, tendpoint);
                    cubic.flatten(&mut points);
                    current_position = tendpoint;
                }
                PathElement::Close => {
                    points.push(start);
                }
                PathElement::MoveTo(..) => (),
            }
        }

        let mut i = 0;
        while i < points.len() - 1 {
            primitives::line(canvas, points[i], points[i + 1], color);
            i += 1;
        }
    }
}

pub mod primitives {
    use super::{Canvas, Vec2, Color};
    use std::{cmp::Ordering};

    pub fn circle(canvas: &mut Canvas, center: Vec2, radius: f32, color: Color) {
        let y0 = (center.y - radius) as i32;
        let x0 = (center.x - radius) as i32;

        let diameter = (2.0 * radius).ceil() as i32;
        let radius2 = radius * radius;

        let width = canvas.width();
        let height = canvas.height();

        for iy in 0..diameter {
            let fy = iy as f32;

            for ix in 0..diameter {
                let fx = ix as f32;

                let oy = (fy - radius).abs();
                let ox = (fx - radius).abs();
                if ox*ox + oy*oy >= radius2 {
                    continue;
                }
                let y = y0 + iy;
                let x = x0 + ix;
                if x >= 0 && y >= 0 && x < width && y < height {
                    *canvas.at_mut(x, y) = color;
                }
            }
        }
    }

    pub fn line(canvas: &mut Canvas, start: Vec2, end: Vec2, color: Color) {
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();

        if dx == 0.0 && dy == 0.0 {
            let x = start.x as i32;
            let y = start.y as i32;
            if x >= 0 && y >= 0 && x < canvas.width() && y < canvas.height() {
                *canvas.at_mut(start.x as i32, start.y as i32) = color;
            }
        } else if dx >= dy {
            generic_line(canvas, start.x as i32, end.x as i32, start.y as i32, end.y as i32, |canvas, x, y| {
                if x >= 0 && y >= 0 && x < canvas.width() && y < canvas.height() {
                    *canvas.at_mut(x, y) = color;
                }
            });
        } else {
            generic_line(canvas, start.y as i32, end.y as i32, start.x as i32, end.x as i32, |canvas, y, x| {
                if x >= 0 && y >= 0 && x < canvas.width() && y < canvas.height() {
                    *canvas.at_mut(x, y) = color;
                }
            });
        }
    }

    fn generic_line(
        canvas: &mut Canvas,
        c0: i32, c1: i32,
        u0: i32, u1: i32,
        pixel: impl Fn(&mut Canvas, i32, i32)) {

        let (c0, c1, u0, u1) = if c1 >= c0 {
            (c0 as i32, c1 as i32, u0 as i32, u1 as i32) 
        } else {
            (c1 as i32, c0 as i32, u1 as i32, u0 as i32)
        };
        let inc = if u1 < u0 { -1 } else { 1 };

        let dc = c1 - c0;
        let du = (u1 - u0).abs();
        let two_dc = 2 * dc;
        let two_du = 2 * du;

        let mut u = u0;
        let mut decision = two_du - dc;

        for c in c0..=c1 {
            pixel(canvas, c, u);
            if decision <= 0 {
                decision += two_du;
            } else {
                decision += two_du - two_dc;
                u += inc;
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Edge {
        y_min: f32,
        y_max: f32,
        x_hit: f32,
        m_inv: f32,
        direction: i32
    }

    pub fn polygon(canvas: &mut Canvas, points: &[Vec2], runs: &[(usize, usize)], color: Color) {
        assert!(points.len() > 2);
        // TODO: pixel coverage based anti-aliasing without vertical supersampling
        let vertical_subsamples = 5; // should be accepted as argument

        fn make_edge(start: Vec2, end: Vec2, vertical_subsamples: f32) -> Edge {
            let (start_x, start_y) = (start.x, start.y * vertical_subsamples);
            let (end_x, end_y) = (end.x, end.y * vertical_subsamples);

            let (y_min, y_max, x_hit, direction) = if start_y < end_y {
                (start_y as f32, end_y as f32, start_x, 1)
            } else {
                (end_y as f32, start_y as f32, end_x, -1)
            };

            let m_inv = (end_x - start_x)/(end_y - start_y);

            Edge {
                y_min,
                y_max,
                x_hit,
                m_inv,
                direction
            }
        }

        let mut edges = Vec::<Edge>::new();

        for &(start, end) in runs {
            let mut i = start;
            while i < end {
                edges.push(make_edge(points[i], points[i + 1], vertical_subsamples as f32));
                i += 1;
            }

            edges.push(make_edge(points[end], points[start], vertical_subsamples as f32));
        }

        edges.sort_by(|a, b| match b.y_min.partial_cmp(&a.y_min) {
            Some(Ordering::Equal) => b.x_hit.partial_cmp(&a.x_hit).unwrap_or(Ordering::Equal),
            Some(other) => other,
            None => Ordering::Greater
        });

        let mut scanline = vec![0u8; canvas.width as usize];
        let mut active_edges = Vec::<Edge>::with_capacity(edges.len());

        let width = canvas.width();
        let stride = canvas.stride as usize;

        let mut y = edges.last().unwrap().y_min as i32;
        while !edges.is_empty() || !active_edges.is_empty() {
            scanline.fill(0);
            for _ in 0..vertical_subsamples {
                if y >= canvas.height() * vertical_subsamples {
                    break;
                }
                let scan_y = y as f32 + 0.5;

                active_edges.retain(|edge| !(edge.y_max <= scan_y));

                while let Some(edge) = edges.last() && edge.y_min <= scan_y {
                    let mut edge = edges.pop().unwrap();
                    if edge.y_max > scan_y {
                        edge.x_hit += edge.m_inv * (scan_y - edge.y_min);
                        active_edges.push(edge);
                    }
                }

                if y >= 0 {
                    active_edges.sort_by(|a, b| a.x_hit.partial_cmp(&b.x_hit).unwrap());
                    draw_active_edges(&active_edges, &mut scanline, width, (255 / vertical_subsamples) as u8);
                }

                y += 1;

                for edge in active_edges.iter_mut() {
                    if edge.m_inv == f32::INFINITY {
                        continue;
                    }
                    edge.x_hit += edge.m_inv;
                }
            }
            if y < 0 {
                continue;
            }
            if y >= canvas.height() * vertical_subsamples {
                break;
            }

            let y = (y / vertical_subsamples) as usize;

            let row: &mut [Color] = bytemuck::cast_slice_mut(&mut canvas.pixels[y * stride..(y + 1) * stride]);
            for x in 0..scanline.len() {
                let alpha = ((scanline[x] as u32 * color.alpha() as u32)/255) as u8;
                row[x] = row[x].blend(color.with_alpha(alpha));
            }
        }
    }

    fn draw_active_edges(aet: &[Edge], scanline: &mut [u8], width: i32, max_weight: u8) {
        let mut current_x = 0.0;
        let mut winding = 0;

        for edge in aet {
            if winding == 0 {
                current_x = edge.x_hit;
                winding += edge.direction;
                continue;
            }
            let x_hit = edge.x_hit;
            let mut x0 = current_x as i32;
            let mut x1 = x_hit as i32;
            winding += edge.direction;

            // TODO: support for evenodd fill rule
            if winding == 0 {
                if x1 >= 0 && x0 < width {
                    if x0 >= 0 {
                        scanline[x0 as usize] = scanline[x0 as usize].saturating_add(((1.0 - current_x.fract()) * max_weight as f32).abs() as u8);
                    } else {
                        x0 = -1;
                    }

                    if x1 < width {
                        scanline[x1 as usize] = scanline[x1 as usize].saturating_add((x_hit.fract() * max_weight as f32).abs() as u8);
                    } else {
                        x1 = width;
                    }


                    for x in (x0+1)..x1 {
                        scanline[x as usize] = scanline[x as usize].saturating_add(max_weight);
                    }
                }
            }
        }
    }    
}
