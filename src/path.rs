use crate::{affine::Affine, canvas::Canvas, color::Color, primitives, stroke::{self, Stroke}, vec::Vec2};

#[derive(Clone, Copy)]
pub enum PathElement {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo(Vec2, Vec2),
    CurveTo(Vec2, Vec2, Vec2),
    Close
}

impl PathElement {
    pub fn endpoint(&self) -> Option<Vec2> {
        match self {
            &PathElement::MoveTo(p) => Some(p),
            &PathElement::LineTo(p) => Some(p),
            &PathElement::QuadTo(_, p) => Some(p),
            &PathElement::CurveTo(_, _, p) => Some(p),
            PathElement::Close => None,
        }
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

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the first non-[`PathElement::MoveTo`] [`PathElement`] if it exists, [`None`]
    /// otherwise.
    pub fn first_element(&self) -> Option<&PathElement> {
        for element in self.elements() {
            if let PathElement::MoveTo(..) = element {
                return Some(element);
            }
        }
        None
    }

    /// Retruns the last [`PathElement`] if it exists, [`None`] otherwise.
    pub fn last_element(&self) -> Option<&PathElement> {
        self.elements.last()
    }

    /// Returns the first point in the [`Path`] (equivalent to the [`Path::startpoint`]) as a 
    /// [`Vec2`] if it exists, [`None`] otherwise.
    pub fn first_point(&self) -> Option<Vec2> {
        match self.elements.first() {
            Some(&PathElement::MoveTo(m)) => Some(m),
            None => None,
            _ => unreachable!()
        }
    }

    /// Returns the last point in the [`Path`] as a [`Vec2`] if it exists, [`None`] otherwise.
    ///
    /// NOTE: this is not equivalent to the paths endpoint, since this function does not consider 
    /// [`PathElement::Close`]. For the endpoint use use [`Path::endpoint`] for that.
    pub fn last_point(&self) -> Option<Vec2> {
        for element in self.elements().rev() {
            if let Some(point) = element.endpoint() {
                return Some(point);
            }
        }
        None
    }

    /// Returns the point the [`Path`] starts at (equivalent to [`Path::first_point`]) as a [`Vec2`] 
    /// if it exists, [`None`] otherwise
    pub fn startpoint(&self) -> Option<Vec2> {
        self.first_point()
    }

    /// Returns the point the [`Path`] ends at as a [`Vec2`] if it exists, [`None`] otherwise.
    pub fn endpoint(&self) -> Option<Vec2> {
        use PathElement::{MoveTo, LineTo, QuadTo, CurveTo, Close};

        let mut elements = self.elements().rev();
        while let Some(element) = elements.next() {
            match element {
                &(MoveTo(p) | LineTo(p) | QuadTo(_, p) | CurveTo(_, _, p)) => return Some(p),
                Close => break,
            }
        }

        while let Some(element) = elements.next() {
            if let &MoveTo(p) = element {
                return Some(p);
            }
        }

        None
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn push_line(&mut self, line: Line) {
        if self.elements.is_empty() {
            self.elements.push(PathElement::MoveTo(line.p0));
        } else {
            self.elements.push(PathElement::LineTo(line.p0));
        }
        if line.p0.is_nan() || line.p1.is_nan() {
            eprintln!("push_line with nan point found");
        }
        self.elements.push(PathElement::LineTo(line.p1));
    }

    pub fn push_quad(&mut self, quad: Quadratic) {
        if self.elements.is_empty() {
            self.elements.push(PathElement::MoveTo(quad.p0));
        } else {
            self.elements.push(PathElement::LineTo(quad.p0));
        }
        if quad.p0.is_nan() || quad.p1.is_nan() || quad.p2.is_nan() {
            eprintln!("push_quad with nan point found");
        }
        self.elements.push(PathElement::QuadTo(quad.p1, quad.p2));
    }

    pub fn push_cubic(&mut self, cubic: Cubic) {
        if self.elements.is_empty() {
            self.elements.push(PathElement::MoveTo(cubic.p0));
        } else {
            self.elements.push(PathElement::LineTo(cubic.p0));
        }
        if cubic.p0.is_nan() || cubic.p1.is_nan() || cubic.p2.is_nan() || cubic.p3.is_nan() {
            eprintln!("push_cubic with nan point found");
        }
        self.elements.push(PathElement::CurveTo(cubic.p1, cubic.p2, cubic.p3));
    }

    pub fn close(&mut self) {
        if self.elements.is_empty() { return; }
        self.elements.push(PathElement::Close);
    }

    pub fn push(&mut self, element: PathElement) {
        if !matches!(element, PathElement::MoveTo(..)) && self.elements.is_empty() {
            panic!("push of non-MoveTo element into empty path");
        }
        self.elements.push(element);
    }

    pub fn extend(&mut self, elements: impl IntoIterator<Item = PathElement>) {
        let mut elements = elements.into_iter();
        if self.elements.is_empty() {
            match elements.next() {
                Some(m @ PathElement::MoveTo(..)) => self.elements.push(m),
                Some(_) => panic!("push of non-MoveTo element into empty path (via extend)"),
                None => return,
            }
        }
        self.elements.extend(elements);
    }

    pub fn elements(&self) -> impl Iterator<Item = &PathElement> + DoubleEndedIterator {
        self.elements.iter()
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
pub struct Line {
    pub p0: Vec2,
    pub p1: Vec2,
}

impl Line {
    pub fn new(p0: Vec2, p1: Vec2) -> Self {
        Line { p0, p1 }
    }

    pub fn evaluate(&self, t: f32) -> Vec2 {
        self.p0.lerp(self.p1, t)
    }
}

#[derive(Clone, Copy)]
pub struct Quadratic {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
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

    pub fn as_cubic(&self) -> Cubic {
        const TWO_THIRDS: f32 = 2.0/3.0;
        let c1 = self.p0 + (self.p1 - self.p0)*TWO_THIRDS;
        let c2 = self.p2 - (self.p2 - self.p1)*TWO_THIRDS;
        Cubic::new(self.p0, c1, c2, self.p2)
    }
}

#[derive(Debug, Clone, Copy)]
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

pub fn transform_path(src: &Path, transform: Affine) -> Path {
    let mut dest = Path::new();
    for element in src.elements() {
        match element {
            &PathElement::MoveTo(p0) =>
                dest.push(PathElement::MoveTo(transform * p0)),
            &PathElement::LineTo(p0) =>
                dest.push(PathElement::LineTo(transform * p0)),
            &PathElement::QuadTo(p0, p1) =>
                dest.push(PathElement::QuadTo(transform * p0, transform * p1)),
            &PathElement::CurveTo(p0, p1, p2) =>
                dest.push(PathElement::CurveTo(transform * p0, transform * p1, transform * p2)),
            PathElement::Close =>
                dest.push(PathElement::Close)
        }
    }

    dest
}

pub fn promote_path(src: &Path) -> Path {
    let mut dest = Path::new();

    for elements in src.subpaths() {
        let Some(&PathElement::MoveTo(mut current_position)) = elements.first() else {
            unreachable!();
        };

        for element in elements {
            match element {
                &PathElement::MoveTo(endpoint) => {
                    dest.push(PathElement::MoveTo(endpoint));
                    current_position = endpoint;
                }
                &PathElement::LineTo(endpoint) => {
                    // TODO: promote lines into cubic beziers as well
                    dest.push(PathElement::LineTo(endpoint));
                    current_position = endpoint;
                }
                &PathElement::QuadTo(control, endpoint) => {
                    let cubic = Quadratic::new(current_position, control, endpoint).as_cubic();
                    dest.push(PathElement::CurveTo(cubic.p1, cubic.p2, cubic.p3));
                    current_position = endpoint;
                }
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    dest.push(PathElement::CurveTo(control1, control2, endpoint));
                    current_position = endpoint;
                }
                PathElement::Close =>
                    dest.push(PathElement::Close)
            }
        }
    }

    dest
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
                    if endpoint.is_nan() { panic!("nan found in LineTo"); }
                    let tendpoint = transform * endpoint;
                    points.push(current_position);
                    points.push(tendpoint);
                    current_position = tendpoint;
                },
                &PathElement::QuadTo(control_point, endpoint) => {
                    if control_point.is_nan() || endpoint.is_nan() { panic!("nan found in QuadTo"); }
                    let tendpoint = transform * endpoint;
                    let quadratic = Quadratic::new(current_position, transform * control_point, tendpoint);
                    quadratic.flatten(&mut points);
                    current_position = tendpoint;
                },
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    if control1.is_nan() || control1.is_nan() || endpoint.is_nan() { panic!("nan found in CurveTo"); }
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

pub fn stroke_path(canvas: &mut Canvas, path: &Path, stroke: &Stroke, transform: Affine, color: Color) {
    
    let stroked = stroke::expand_stroke(&transform_path(path, transform), stroke);
    fill_path(canvas, &stroked, Affine::IDENTITY, color);
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

