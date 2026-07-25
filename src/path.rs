use crate::{primitives, affine::Affine, canvas::Canvas, color::Color, vec::Vec2};

#[derive(Clone, Copy)]
pub enum PathElement {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo(Vec2, Vec2),
    CurveTo(Vec2, Vec2, Vec2),
    Close
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

pub fn fill_path(canvas: &mut Canvas, path: &Path, transform: Affine, color: Color) {
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

