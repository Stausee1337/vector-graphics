
use crate::{offset, path::{Cubic, Line, Path, PathElement, Quadratic}, vec::Vec2};

#[derive(Clone)]
pub struct Stroke {
    pub width: f32
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
    for element in elements {
        match element {
            &PathElement::MoveTo(position) => {
                start_position = position;
                current_position = position;
                stroker.finish();
            },
            &PathElement::LineTo(endpoint) => {
                stroker.do_line(Line::new(current_position, endpoint));
                current_position = endpoint;
            },
            &PathElement::QuadTo(control, endpoint) => {
                let quad = Quadratic::new(current_position, control, endpoint);
                stroker.do_cubic(quad.as_cubic());
                current_position = endpoint;
            },
            &PathElement::CurveTo(control1, control2, endpoint) => {
                stroker.do_cubic(Cubic::new(current_position, control1, control2, endpoint));
                current_position = endpoint;
            }
            PathElement::Close => {
                stroker.do_line(Line::new(current_position, start_position));
                current_position = start_position;
                stroker.finish();
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
}

enum DestPath {
    Forward, Backward
}

enum IncompleteConstructor {
    LineTo,
    // QuadTo(Vec2),
    CurveTo(Vec2, Vec2)
}

impl IncompleteConstructor {
    fn complete_with_endpoint(self, dest: &mut Path, endpoint: Vec2) {
        match self {
            IncompleteConstructor::LineTo =>
                dest.push(PathElement::LineTo(endpoint)),
            // IncompleteConstructor::QuadTo(control) =>
            //     dest.push(PathElement::QuadTo(control, endpoint)),
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
            backward: Path::new()
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
        }

        self.output.extend(self.forward.elements().copied());

        let mut incomplete_constructor = IncompleteConstructor::LineTo;
        for element in self.backward.elements().rev() {
            match element {
                &PathElement::LineTo(endpoint) => {
                    let prev = std::mem::replace(&mut incomplete_constructor, IncompleteConstructor::LineTo);
                    prev.complete_with_endpoint(&mut self.output, endpoint);
                }
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    let prev = std::mem::replace(&mut incomplete_constructor, IncompleteConstructor::CurveTo(control2, control1));
                    prev.complete_with_endpoint(&mut self.output, endpoint);
                },
                &PathElement::MoveTo(endpoint) => {
                    incomplete_constructor.complete_with_endpoint(&mut self.output, endpoint);
                    break;
                },
                PathElement::QuadTo(..) | PathElement::Close => unreachable!()
            }
        }

        self.output.close();
        self.forward.clear();
        self.backward.clear();
    }

    fn do_join(&mut self, dest: DestPath) {
        if self.working.is_empty() { return; }

        let path = match dest {
            DestPath::Forward => &mut self.forward,
            DestPath::Backward => &mut self.backward,
        };
        let mut elements = self.working.elements().copied();
        if !path.is_empty() {
            // let last_point = last_element.endpoint().unwrap();
            let Some(PathElement::MoveTo(next_point)) = elements.next() else {
                unreachable!();
            };
            path.push(PathElement::LineTo(next_point));
        }
        path.extend(elements);
        self.working.clear();
    }

    fn do_line(&mut self, line: Line) {
        const EPSILON: f32 = 1e-3;
        if (line.p1 - line.p0).length() <= EPSILON {
            return;
        }
        let forward = make_offset_line(&line, -self.stroke.width * 0.5);
        let backward = make_offset_line(&line, self.stroke.width * 0.5);
        self.working.push_line(forward);
        self.do_join(DestPath::Forward);
        self.working.push_line(backward);
        self.do_join(DestPath::Backward);
    }

    fn do_cubic(&mut self, cubic: Cubic) {
        let chord = cubic.p3 - cubic.p0;

        const EPSILON: f32 = 1e-6;
        if chord.cross(cubic.p1 - cubic.p0).abs() < EPSILON && chord.cross(cubic.p2 - cubic.p0).abs() < EPSILON {
            self.do_line(Line::new(cubic.p0, cubic.p1));
            return;
        }

        offset::compute_offset_curve(&mut self.working, cubic, -self.stroke.width * 0.5);
        self.do_join(DestPath::Forward);
        offset::compute_offset_curve(&mut self.working, cubic, self.stroke.width * 0.5);
        self.do_join(DestPath::Backward);
    }
}

fn make_offset_line(line: &Line, offset: f32) -> Line {
    // L(t) = A + (B - A)t
    // L'(t) = (B - A)
    // L_d(t) = L(t) + d*N(t), where N(t) = (y'(t), -x'(t))/|L'(t)|
    // N(t) = (yb - ya, xa - xb).norm()
    let offset_vec = -(line.p1 - line.p0).turn90().norm() * offset;
    let p0 = line.p0 + offset_vec;
    let p1 = line.p1 + offset_vec;
    Line::new(p0, p1)
}

