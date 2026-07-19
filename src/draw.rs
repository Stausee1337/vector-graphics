use std::ops::{Add, Mul, Sub};

pub struct Canvas<'a> {
    pub pixels: &'a mut [u32],
    pub width: usize,
    pub height: usize,
    pub stride: usize
}

impl<'a> Canvas<'a> {
    pub fn clear(&mut self) {
        self.pixels.fill(0);
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

pub struct Qudaratic {
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
}

impl Qudaratic {

    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        Qudaratic { p0, p1, p2 }
    }

    pub fn error(&self) -> f32 {
        let control = self.p1 - self.p0;
        let chord  = self.p2 - self.p0;

        let lambda = (control.dot(chord)/chord.dot(chord)).clamp(0.0, 1.0);
        let control_flat = chord * lambda;

        0.5 * (control - control_flat).length()
    }

    pub fn split(&self) -> (Qudaratic, Qudaratic) {
        let split_point = self.evaluate(0.5);
        let control1 = self.p0.lerp(self.p1, 0.5);
        let control2 = self.p1.lerp(self.p2, 0.5);
        (Qudaratic::new(self.p0, control1, split_point), Qudaratic::new(split_point, control2, self.p2))
    }

    pub fn evaluate(&self, mut t: f32) -> Vec2 {
        t = t.clamp(0.0, 1.0);
        let s = 1.0 - t;
        self.p0*s*s + self.p1*2.0*s*t + self.p2*t*t
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

pub mod primitives {
    use super::{Canvas, Vec2};
    use std::{cmp::Ordering};

    pub fn circle(canvas: &mut Canvas, center: Vec2, radius: f32, color: u32) {
        let y0 = (center.y - radius) as i32;
        let x0 = (center.x - radius) as i32;

        let diameter = (2.0 * radius).ceil() as i32;
        let radius2 = radius * radius;

        let Canvas { ref mut pixels, width, height, stride } = *canvas;
        let width = width as i32;
        let height = height as i32;

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
                if y < 0 || x < 0 || y >= height || x >= width {
                    continue;
                }
                pixels[y as usize * stride + x as usize] = color;
            }
        }
    }

    pub fn line(canvas: &mut Canvas, start: Vec2, end: Vec2, color: u32) {
        let dx = (end.x - start.x).abs() as usize;
        let dy = (end.y - start.y).abs() as usize;

        if dx == 0 && dy == 0 {
            canvas.pixels[start.y as usize * canvas.stride + start.x as usize] = color;
        } else if dx >= dy {
            generic_line(canvas, start.x as usize, end.x as usize, start.y as usize, end.y as usize, |canvas, x, y| {
                if x < canvas.width && y < canvas.height {
                    canvas.pixels[y * canvas.stride + x] = color;
                }
            });
        } else {
            generic_line(canvas, start.y as usize, end.y as usize, start.x as usize, end.x as usize, |canvas, y, x| {
                if x < canvas.width && y < canvas.height {
                    canvas.pixels[y * canvas.stride + x] = color;
                }
            });
        }
    }

    fn generic_line(
        canvas: &mut Canvas,
        c0: usize, c1: usize,
        u0: usize, u1: usize,
        pixel: impl Fn(&mut Canvas, usize, usize)) {

        let (c0, c1, u0, u1) = if c1 >= c0 {
            (c0 as isize, c1 as isize, u0 as isize, u1 as isize) 
        } else {
            (c1 as isize, c0 as isize, u1 as isize, u0 as isize)
        };
        let inc = if u1 < u0 { -1 } else { 1 };

        let dc = c1 - c0;
        let du = (u1 - u0).abs();
        let two_dc = 2 * dc;
        let two_du = 2 * du;

        let mut u = u0;
        let mut decision = two_du - dc;

        for c in c0..=c1 {
            pixel(canvas, c as usize, u as usize);
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
        y_min: usize,
        y_max: usize,
        x_hit: f32,
        m_inv: f32
    }

    pub fn polygon(canvas: &mut Canvas, points: &[Vec2], color: u32) {
        assert!(points.len() > 2);

        let mut x = points.iter();
        x.next();

        fn make_edge(start: &Vec2, end: &Vec2) -> Edge {
            let (y_min, y_max, x_hit) = if start.y < end.y {
                (start.y as usize, end.y as usize, start.x)
            } else {
                (end.y as usize, start.y as usize, end.x)
            };

            let m_inv = (end.x - start.x)/(end.y - start.y);

            Edge { y_min, y_max, x_hit, m_inv }
        }

        let mut edges = Vec::<Edge>::with_capacity(points.len() + 1);
        for (start, end) in points.iter().zip(x) {
            edges.push(make_edge(start, end));
        }
        edges.push(make_edge(points.first().unwrap(), points.last().unwrap()));

        edges.sort_by(|a, b| match b.y_min.cmp(&a.y_min) {
            Ordering::Equal => b.x_hit.partial_cmp(&a.x_hit).unwrap_or(Ordering::Equal),
            other => other 
        });

        // let x = edges.iter().map(|edge| edge.y_min).collect::<Vec<_>>();
        // println!("{:?}", x);

        let mut active_edges = Vec::<Edge>::with_capacity(edges.len());

        let mut y = edges.last().unwrap().y_min;
        while !edges.is_empty() || !active_edges.is_empty() {
            if y >= canvas.height {
                return;
            }
            let scanline = &mut canvas.pixels[y * canvas.stride..(y + 1) * canvas.stride];

            while let Some(edge) = edges.last() && edge.y_min == y {
                active_edges.push(edges.pop().unwrap());
            }
            active_edges.retain(|edge| edge.y_max > y); 

            active_edges.sort_by(|a, b| a.x_hit.partial_cmp(&b.x_hit).unwrap());


            let mut aet: &[Edge] = &active_edges;

            let mut x = 0;
            let mut count = 0;
            while x < canvas.width && !aet.is_empty() {
                let next_x = aet[0].x_hit as usize;

                if count % 2 == 1 {
                    let next_x = next_x.min(canvas.width);
                    (&mut scanline[x..next_x]).fill(color);
                }

                x = next_x;
                let mut advance = 0;
                for edge in aet {
                    let x_hit = edge.x_hit as usize;
                    if x_hit > x {
                        break;
                    }
                    advance += 1;
                }
                count += advance;
                aet = &aet[advance..];
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
