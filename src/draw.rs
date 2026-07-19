use std::ops::{Add, Mul, Sub};

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

    pub fn at_mut(&mut self, x: i32, y: i32) -> &mut u32 {
        assert!(x >= 0 && (x as u32) < self.width);
        assert!(y >= 0);
        assert!((y as u32) < self.height);
        &mut self.pixels[y as usize * self.stride as usize + x as usize]
    }
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
        self.dot(self).sqrt()
    }

    pub fn lerp(self, other: Self, t: f32) -> Self { 
        self + (other - self)*t.clamp(0.0, 1.0)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, lambda: f32) -> Vec2 {
        Vec2 { x: self.x * lambda, y: self.y * lambda }
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

    pub fn evaluate(&self, mut t: f32) -> Vec2 {
        t = t.clamp(0.0, 1.0);
        let s = 1.0 - t;
        self.p0*s*s + self.p1*2.0*s*t + self.p2*t*t
    }


    pub fn flatten(&self, points: &mut Vec<Vec2>, offset: Vec2) {
        if self.error() <= 0.25 {
            points.push(self.p0 + offset);
            points.push(self.p1 + offset);
            points.push(self.p2 + offset);
            return;
        }

        let (q0, q1) = self.split();
        q0.flatten(points, offset);
        q1.flatten(points, offset);
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
    Close
}

pub fn draw_path(canvas: &mut Canvas, path: &Path, offset: Vec2, color: u32) {
    let mut points = vec![];
    let mut runs = vec![];

    for subpath in path.subpaths() {
        if subpath.len() < 2 { continue; }

        let PathElement::MoveTo(mut current_position) = subpath[0] else { unreachable!() };

        let start = points.len();
        for element in subpath {
            match element {
                &PathElement::LineTo(endpoint) => {
                    points.push(current_position + offset);
                    points.push(endpoint + offset);
                    current_position = endpoint;
                },
                &PathElement::QuadTo(control_point, endpoint) => {
                    let quadratic = Quadratic::new(current_position, control_point, endpoint);
                    quadratic.flatten(&mut points, offset);
                    current_position = endpoint;
                },
                PathElement::Close => {
                    runs.push((start, points.len() - 1));
                    break;
                },
                _ => ()
            }
        }
    }

    primitives::polygon(canvas, &points, &runs, color);
}

pub mod primitives {
    use super::{Canvas, Vec2};
    use std::{cmp::Ordering};

    pub fn circle(canvas: &mut Canvas, center: Vec2, radius: f32, color: u32) {
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

    pub fn line(canvas: &mut Canvas, start: Vec2, end: Vec2, color: u32) {
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
        y_min: i32,
        y_max: i32,
        x_hit: f32,
        m_inv: f32
    }

    pub fn polygon(canvas: &mut Canvas, points: &[Vec2], runs: &[(usize, usize)], color: u32) {
        assert!(points.len() > 2);


        fn make_edge(start: Vec2, end: Vec2) -> Edge {
            let (y_min, y_max, x_hit) = if start.y < end.y {
                (start.y as i32, end.y as i32, start.x)
            } else {
                (end.y as i32, start.y as i32, end.x)
            };

            let m_inv = (end.x - start.x)/(end.y - start.y);

            Edge { y_min, y_max, x_hit, m_inv }
        }

        let mut edges = Vec::<Edge>::new();

        for &(start, end) in runs {
            let mut i = start;
            while i < end {
                edges.push(make_edge(points[i], points[i + 1]));
                i += 1;
            }

            edges.push(make_edge(points[end], points[start]));
        }

        edges.sort_by(|a, b| match b.y_min.cmp(&a.y_min) {
            Ordering::Equal => b.x_hit.partial_cmp(&a.x_hit).unwrap_or(Ordering::Equal),
            other => other 
        });

        let mut active_edges = Vec::<Edge>::with_capacity(edges.len());

        let width = canvas.width();
        let stride = canvas.stride as usize;

        let mut y = edges.last().unwrap().y_min;
        while !edges.is_empty() || !active_edges.is_empty() {
            if y >= canvas.height() {
                break;
            }

            while let Some(edge) = edges.last() && edge.y_min == y {
                active_edges.push(edges.pop().unwrap());
            }
            active_edges.retain(|edge| edge.y_max > y); 

            if y >= 0 {
                active_edges.sort_by(|a, b| a.x_hit.partial_cmp(&b.x_hit).unwrap());

                let scanline = {
                    let y = y as usize;
                    &mut canvas.pixels[y * stride..(y + 1) * stride]
                };

                let mut aet: &[Edge] = &active_edges;
                let mut x = aet.first().map(|edge| edge.x_hit as i32).unwrap_or(0);
                let mut count = 0;
                while x < width && !aet.is_empty() {
                    let next_x = aet[0].x_hit as i32;

                    if count % 2 == 1 {
                        let x = x.clamp(0, width);
                        let next_x = next_x.clamp(0, width);
                        (&mut scanline[x as usize..next_x as usize]).fill(color);
                    }

                    x = next_x;
                    let mut advance = 0;
                    for edge in aet {
                        let x_hit = edge.x_hit as i32;
                        if x_hit > x {
                            break;
                        }
                        advance += 1;
                    }
                    count += advance;
                    aet = &aet[advance..];
                }
            }

            y += 1;

            if y > 10_000 {
                eprintln!("bailing iteration, {}, {}", edges.len(), active_edges.len());
                return;
            }

            for edge in active_edges.iter_mut() {
                if edge.m_inv == f32::INFINITY {
                    continue;
                }
                edge.x_hit += edge.m_inv;
            }
        }
    }
}
