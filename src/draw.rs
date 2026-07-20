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

#[derive(Clone, Copy, PartialEq, Eq, bytemuck::Pod)]
#[repr(transparent)]
pub struct Color(pub u32);

unsafe impl bytemuck::Zeroable for Color { }

impl Color {
    pub fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color((alpha as u32) << 0o30 | (red as u32) << 0o20 | (green as u32) << 0o10 | blue as u32)
    }

    pub fn alpha(self) -> u8 {
        (self.0 >> 0o30) as u8
    }

    pub fn red(self) -> u8 {
        (self.0 >> 0o20) as u8
    }

    pub fn green(self) -> u8 {
        (self.0 >> 0o10) as u8
    }

    pub fn blue(self) -> u8 {
        (self.0 >> 0o00) as u8
    }

    pub fn with_alpha(self, alpha: u8) -> Self {
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

    pub fn split(&self) -> (Cubic, Cubic) {
        let split_point = self.evaluate(0.5);
        let s = self.p1.lerp(self.p2, 0.5);
        let q1 = self.p0.lerp(self.p1, 0.5);
        let q2 = q1.lerp(s, 0.5);
        let r2 = self.p2.lerp(self.p3, 0.5);
        let r1 = r2.lerp(s, 0.5);

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

    pub fn flatten(&self, points: &mut Vec<Vec2>, offset: Vec2) {
        if self.error() <= 0.25 {
            points.push(self.p0 + offset);
            points.push(self.p1 + offset);
            points.push(self.p2 + offset);
            points.push(self.p3 + offset);
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
    CurveTo(Vec2, Vec2, Vec2),
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
                &PathElement::CurveTo(control1, control2, endpoint) => {
                    let quadratic = Cubic::new(current_position, control1, control2, endpoint);
                    quadratic.flatten(&mut points, offset);
                    current_position = endpoint;
                }
                PathElement::Close => {
                    runs.push((start, points.len() - 1));
                    break;
                },
                PathElement::MoveTo(..) => (),
            }
        }
    }

    primitives::polygon(canvas, &points, &runs, color);
}

pub mod primitives {
    use super::{Canvas, Vec2, Color};
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
        y_min: f32,
        y_max: f32,
        x_hit: f32,
        m_inv: f32,
        direction: i32
    }

    pub fn polygon(canvas: &mut Canvas, points: &[Vec2], runs: &[(usize, usize)], color: u32) {
        assert!(points.len() > 2);
        let color = Color(color);
        // TODO: pixel coverage based anti-aliasing without vertical supersampling
        let vertical_subsamples = 5;

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
