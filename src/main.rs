use std::{cmp::Ordering, num::NonZeroU32, rc::Rc};

use winit::{event::{ElementState, MouseButton, WindowEvent}, event_loop::EventLoop, window::Window};

mod app;

struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
    stride: usize
}

struct Point {
    x: usize,
    y: usize,
}

impl Point {
    fn new(x: usize, y: usize) -> Point {
        Point { x, y }
    }

    fn hit_test(&self, x: usize, y: usize) -> bool {
        const RADIUS: usize = 10;
        return self.x.abs_diff(x) <= RADIUS && self.y.abs_diff(y) <= RADIUS;
    }
}

fn main() {

    let event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    let mut mouse_x = 0;
    let mut mouse_y = 0;

    let mut points: Vec<Point> = vec![];

    let mut active_element: Option<usize> = None;

    let mut app = app::WinitAppBuilder::with_init(
        |elwt| {
            let window = elwt.create_window(Window::default_attributes());
            Rc::new(window.unwrap())
        },
        |_elwt, window| softbuffer::Surface::new(&context, window.clone()).unwrap(),
    )
    .with_event_handler(|window, surface, window_id, event, elwt| {
        // elwt.set_control_flow(ControlFlow::Wait);

        if window_id != window.id() {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                let Some(surface) = surface else {
                    eprintln!("RedrawRequested fired before Resumed or after Suspended");
                    return;
                };
                let size = window.inner_size();
                surface
                    .resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();
                let width = buffer.width().get() as usize;
                let height = buffer.height().get() as usize;
                let mut canvas = Canvas {
                    pixels: &mut buffer,
                    width, height, stride: width
                };

                clear(&mut canvas);

                if points.len() > 2 {
                    polygon(&mut canvas, &points, 0xffffffff);
                    let start = points.first().unwrap();
                    let end = points.last().unwrap();
                    line(&mut canvas, start.x, start.y, end.x, end.y, 0xff00ff00);
                }

                let mut x = points.iter();
                x.next();

                for (start, end) in points.iter().zip(x) {
                    line(&mut canvas, start.x, start.y, end.x, end.y, 0xff00ff00);
                }

                for point in points.iter() {
                    circle(&mut canvas, point.x, point.y, 10, 0xffff0000);
                }

                buffer.present().unwrap();
            }
            WindowEvent::CursorMoved { position, .. } => {
                mouse_x = position.x as usize;
                mouse_y = position.y as usize;

                let element = match active_element {
                    Some(idx) => &mut points[idx],
                    None => return,
                };

                element.x = mouse_x;
                element.y = mouse_y;

                window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        for (idx, element) in points.iter().enumerate() {
                            if element.hit_test(mouse_x, mouse_y) {
                                active_element = Some(idx);
                                return;
                            }
                        }
                        active_element = Some(points.len());
                        points.push(Point::new(mouse_x, mouse_y));
                        window.request_redraw();
                    },
                    ElementState::Released => active_element = None,
                }
            }
            WindowEvent::MouseWheel { delta, ..  } => {

            }
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            _ => {}
        }
    });

    event_loop.run_app(&mut app).unwrap();
}

fn clear(canvas: &mut Canvas) {
    canvas.pixels.fill(0);
}

fn circle(canvas: &mut Canvas, cx: usize, cy: usize, radius: usize, color: u32) {
    let y0 = cy - radius;
    let x0 = cx - radius;

    let diameter = 2 * radius;
    let radius2 = radius * radius;

    let Canvas { ref mut pixels, width, height, stride } = *canvas;

    for dy in 0..diameter {
        for dx in 0..diameter {
            let oy = dy.abs_diff(radius);
            let ox = dx.abs_diff(radius);
            if ox*ox + oy*oy >= radius2 {
                continue;
            }
            let y = y0 + dy;
            let x = x0 + dx;
            if y >= height || x >= width {
                continue;
            }
            pixels[y * stride + x] = color;
        }
    }
}

fn line(canvas: &mut Canvas, sx: usize, sy: usize, ex: usize, ey: usize, color: u32) {
    let dx = ex.abs_diff(sx);
    let dy = ey.abs_diff(sy);

    if dx == 0 && dy == 0 {
        canvas.pixels[sy * canvas.stride + sx] = color;
    } else if dx >= dy {
        generic_line(canvas, sx, ex, sy, ey, |canvas, x, y| {
            if x < canvas.width && y < canvas.height {
                canvas.pixels[y * canvas.stride + x] = color;
            }
        });
    } else {
        generic_line(canvas, sy, ey, sx, ex, |canvas, y, x| {
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

fn polygon(canvas: &mut Canvas, points: &[Point], color: u32) {
    assert!(points.len() > 2);

    let mut x = points.iter();
    x.next();

    fn make_edge(start: &Point, end: &Point) -> Edge {
        let (y_min, y_max, x_hit) = if start.y < end.y {
            (start.y, end.y, start.x as f32)
        } else {
            (end.y, start.y, end.x as f32)
        };

        let m_inv = (end.x as f32 - start.x as f32)/(end.y as f32 - start.y as f32);

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

