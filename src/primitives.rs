use std::cmp::Ordering;

use crate::{canvas::Canvas, color::Color, vec::Vec2};


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
    let vertical_subsamples = 15; // should be accepted as argument

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

