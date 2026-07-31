
use crate::{offset, path::{Cubic, Line, Path, PathElement, Quadratic}, vec::Vec2};

#[derive(Clone, Copy)]
pub enum Join {
    Bevel,
    Miter {
        miter_limit: f32
    },
    Round,
}

#[derive(Clone, Copy)]
pub struct Stroke {
    pub width: f32,
    pub join: Join
}

pub fn expand_stroke(path: &Path, stroke: &Stroke) -> Path {
    let mut stroker = Stroker::new(stroke.clone());

    if path.is_empty() {
        return stroker.output;
    }

    let mut elements = path.elements();
    let Some(&PathElement::MoveTo(mut start_position)) = elements.next() else {
        unreachable!();
    };
    
    let mut current_position = start_position;
    stroker.current_pos = current_position;
    for element in elements {
        match element {
            &PathElement::MoveTo(position) => {
                start_position = position;
                current_position = position;
                stroker.finish();
                stroker.current_pos = current_position;
            },
            &PathElement::LineTo(endpoint) => {
                let tangent = endpoint - current_position;
                stroker.do_join(tangent);
                stroker.do_line(Line::new(current_position, endpoint));
                current_position = endpoint;
            },
            &PathElement::QuadTo(control, endpoint) => {
                let quad = Quadratic::new(current_position, control, endpoint);
                let tangent = control - current_position;
                stroker.do_join(tangent);
                stroker.do_cubic(quad.as_cubic());
                current_position = endpoint;
            },
            &PathElement::CurveTo(control1, control2, endpoint) => {
                let cubic = Cubic::new(current_position, control1, control2, endpoint);
                let tangent = control1 - current_position;
                stroker.do_join(tangent);
                stroker.do_cubic(cubic);
                current_position = endpoint;
            }
            PathElement::Close => {
                let tangent = start_position - current_position;
                stroker.do_join(tangent);
                stroker.do_line(Line::new(current_position, start_position));
                current_position = start_position;
                stroker.finish_closed();
            },
        }
    }

    stroker.finish();


    stroker.output
}

pub struct Stroker {
    stroke: Stroke,
    output: Path,
    working: Path,
    forward: Path,
    backward: Path,
    current_pos: Vec2,
    current_tangent: Vec2,
}

enum IncompleteConstructor {
    MoveTo,
    LineTo,
    QuadTo(Vec2),
    CurveTo(Vec2, Vec2)
}

impl IncompleteConstructor {
    fn complete_with_endpoint(self, dest: &mut Path, endpoint: Vec2) {
        match self {
            IncompleteConstructor::MoveTo =>
                dest.push(PathElement::MoveTo(endpoint)),
            IncompleteConstructor::LineTo =>
                dest.push(PathElement::LineTo(endpoint)),
            IncompleteConstructor::QuadTo(control) =>
                dest.push(PathElement::QuadTo(control, endpoint)),
            IncompleteConstructor::CurveTo(control1, control2) =>
                dest.push(PathElement::CurveTo(control1, control2, endpoint)),
        }
    }
}

impl Stroker {
    pub fn new(stroke: Stroke) -> Stroker {
        Stroker {
            stroke,
            output: Path::new(),
            working: Path::new(),
            forward: Path::new(),
            backward: Path::new(),
            current_pos: Vec2::new(0.0, 0.0),
            current_tangent: Vec2::new(0.0, 0.0),
        }
    }

    pub fn output(&self) -> &Path {
        &self.output
    }

    fn finish(&mut self) {
        if self.forward.is_empty() || self.backward.is_empty() {
            return;
        }

        {
            let start = self.forward.startpoint().unwrap();
            self.forward.push(PathElement::LineTo(start));

            let start = self.backward.startpoint().unwrap();
            self.backward.push(PathElement::LineTo(start));
            // self.do_join(tangent_next);
        }

        self.output.extend(self.forward.elements().copied());
        extend_reversed(&mut self.output, &self.backward);

        self.output.close();
        self.forward.clear();
        self.backward.clear();
    }

    fn finish_closed(&mut self) {
        if self.forward.is_empty() || self.backward.is_empty() {
            return;
        }

        self.forward.close();

        self.output.extend(self.forward.elements().copied());
        extend_reversed(&mut self.output, &self.backward);
        self.output.close();

        self.forward.clear();
        self.backward.clear();
    }

    fn do_join(&mut self, mut tangent_next: Vec2) {

        if tangent_next.length_squared() <= 1e-6 {
            eprintln!("extremely small tangent");
            return;
        }
        tangent_next = tangent_next.norm();
        let tangent_prev = self.current_tangent;

        if tangent_prev.is_nan() {
            eprintln!("current tangent is nan");
            return;
        }

        let normal = tangent_next.turn90();
        let fw_next = self.current_pos - normal * self.stroke.width * 0.5;
        let bw_next = self.current_pos + normal * self.stroke.width * 0.5;

        let (Some(fw_prev), Some(bw_prev)) = (self.forward.endpoint(), self.backward.endpoint()) else {
            self.forward.move_to(fw_next);
            self.backward.move_to(bw_next);
            return;
        };

        let ab = tangent_prev;
        let cd = tangent_next;
        let cross = ab.cross(cd);
        let dot = ab.dot(cd);
        let hypot = cross.hypot(dot);

        let join_threshold = 0.5 / self.stroke.width;
        if dot > 0.0 && cross.abs() < hypot * join_threshold {
            return;
        }

        match self.stroke.join {
            Join::Bevel => {
                self.forward.push(PathElement::LineTo(fw_next));
                self.backward.push(PathElement::LineTo(bw_next));
            }
            Join::Miter { miter_limit } => {
                if dot.abs() <= 1.0 - 2. / (miter_limit * miter_limit) {
                    if cross > 0.0 {
                        let fp_last = fw_prev;
                        let fp_this = fw_next;
                        let h = ab.cross(fp_this - fp_last) / cross;
                        let miter_pt = fp_this - cd * h;
                        self.forward.line_to(miter_pt);
                        self.backward.line_to(self.current_pos);
                    } else if cross < 0.0 {
                        let fp_last = bw_prev;
                        let fp_this = bw_next;
                        let h = ab.cross(fp_this - fp_last) / cross;
                        let miter_pt = fp_this - cd * h;
                        self.backward.line_to(miter_pt);
                        self.forward.line_to(self.current_pos);
                    }
                }

                self.forward.push(PathElement::LineTo(fw_next));
                self.backward.push(PathElement::LineTo(bw_next));
            }
            Join::Round => todo!()
        }
    }

    fn do_line(&mut self, line: Line) {
        const EPSILON: f32 = 1e-3;
        if (line.p1 - line.p0).length() <= EPSILON {
            return;
        }
        let forward = make_offset_line(&line, -self.stroke.width * 0.5);
        let backward = make_offset_line(&line, self.stroke.width * 0.5);
        self.forward.line_to(forward.p1);
        self.backward.line_to(backward.p1);
        self.current_pos = line.p1;
        self.current_tangent = (line.p1 - line.p0).norm();
    }

    fn do_cubic(&mut self, cubic: Cubic) {
        let chord = cubic.p3 - cubic.p0;

        const EPSILON: f32 = 1e-6;
        if chord.cross(cubic.p1 - cubic.p0).abs() < EPSILON && chord.cross(cubic.p2 - cubic.p0).abs() < EPSILON {
            self.do_line(Line::new(cubic.p0, cubic.p1));
            return;
        }

        {
            self.working.clear();
            offset::compute_offset_curve(&mut self.working, cubic, -self.stroke.width * 0.5).unwrap();
            let elements = self.working.elements();
            if self.forward.is_empty() {
                self.forward.extend(elements.copied());
            } else {
                self.forward.extend(elements.skip(1).copied());
            }
        }
        {
            self.working.clear();
            offset::compute_offset_curve(&mut self.working, cubic, self.stroke.width * 0.5).unwrap();
            let elements = self.working.elements();
            if self.backward.is_empty() {
                self.backward.extend(elements.copied());
            } else {
                self.backward.extend(elements.skip(1).copied());
            }
        }

        self.current_pos = cubic.p3;
        self.current_tangent = (cubic.p3 - cubic.p2).norm();
    }
}

fn extend_reversed(dest: &mut Path, src: &Path) {
    let mut incomplete_constructor = IncompleteConstructor::MoveTo;
    for element in src.elements().rev() {
        match element {
            &PathElement::LineTo(endpoint) => {
                let prev = std::mem::replace(&mut incomplete_constructor, IncompleteConstructor::LineTo);
                prev.complete_with_endpoint(dest, endpoint);
            }
            &PathElement::QuadTo(control, endpoint) => {
                let prev = std::mem::replace(&mut incomplete_constructor, IncompleteConstructor::QuadTo(control));
                prev.complete_with_endpoint(dest, endpoint);
            }
            &PathElement::CurveTo(control1, control2, endpoint) => {
                let prev = std::mem::replace(&mut incomplete_constructor, IncompleteConstructor::CurveTo(control2, control1));
                prev.complete_with_endpoint(dest, endpoint);
            },
            &PathElement::MoveTo(endpoint) => {
                incomplete_constructor.complete_with_endpoint(dest, endpoint);
                break;
            },
            PathElement::Close => unreachable!()
        }
    }
}

fn make_offset_line(line: &Line, offset: f32) -> Line {
    // L(t) = A + (B - A)t
    // L'(t) = (B - A)
    // L_d(t) = L(t) + d*N(t), where N(t) = (y'(t), -x'(t))/|L'(t)|
    // N(t) = (yb - ya, xa - xb).norm()
    let offset_vec = (line.p1 - line.p0).turn90().norm() * offset;
    let p0 = line.p0 + offset_vec;
    let p1 = line.p1 + offset_vec;
    Line::new(p0, p1)
}

