use std::cmp::Ordering;

use crate::{canvas::Canvas, color::Color, vec::Vec2};


pub fn circle(canvas: &mut Canvas, center: Vec2, radius: f32, color: Color) {
    // maybe approximate into line segements and draw with scanline rasterization
    // dphi = 2 * arccos(1 - epsilon/R)
    // segments = angle/dphi
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

pub fn polygon(canvas: &mut Canvas, points: &[Vec2], runs: &[usize], color: Color) {
    assert!(!runs.is_empty());

    // TODO: pixel coverage based anti-aliasing without vertical supersampling
    let vertical_subsamples = 15;

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

    let mut edges = vec![];
    let mut offset = 0;
    for &length in runs {
        assert!(length >= 3);
        let end = offset + length;

        let mut i = offset;
        while i < end - 1 {
            edges.push(make_edge(points[i], points[i + 1], vertical_subsamples as f32));
            i += 1;
        }

        edges.push(make_edge(points[end - 1], points[offset], vertical_subsamples as f32));
        offset = end;
    }

    edges.sort_by(|a, b| match f32::total_cmp(&b.y_min, &a.y_min) {
        Ordering::Equal => f32::total_cmp(&b.x_hit, &a.x_hit),
        other => other,
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

            // FIXME?: a lot of implementations don't start an actual sort step, instead they 
            // "insort" the new active edges, which might be more performant (would have to be
            // profiled). Some analytical anti aliasing implementions don't require active edges to
            // be sorted in x-direction entirely, so this might not be worth improving.
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

pub fn anti_line(canvas: &mut Canvas, p0: Vec2, p1: Vec2, color: Color) {
    anti_line_generic::<false>(canvas, p0, p1, color);
}

pub fn anti_polyline(canvas: &mut Canvas, points: &[Vec2], color: Color) {
    assert!(points.len() >= 2);
    anti_line_generic::<false>(canvas, points[0], points[1], color);

    let mut i = 1;
    while i < points.len() - 1 {
        anti_line_generic::<true>(canvas, points[i], points[i + 1], color);
        i += 1;
    }
}

/// Draws an anti aliased line segment between point p0, p1 using Xiaolin Wu's line algorithm. Code
/// adapted from wikipedia sample https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm.
///
/// If the generic `POLYLINE` paramter is true, the function does not draw starting point, instead
/// leaving that to the to the previous invocation.
fn anti_line_generic<const POLYLINE: bool>(canvas: &mut Canvas, p0: Vec2, p1: Vec2, color: Color) {
    // TODO: the result is ok, but I think still shows some artifacting, especially at joins in
    // polylines. There is not much that can be done about this, except for not drawing the curve
    // as segments. One approach though, might be to split the bezier curves into cricular arcs, 
    // instead of lines and using the circle algorithm corresponding to this line algorithm. Since
    // bezier curves are typically better approximated by arcs, this would reduce the number of
    // joins, and such artifacting.
    let Vec2 { x: mut x0, y: mut y0 } = p0;
    let Vec2 { x: mut x1, y: mut y1 } = p1;
    let steep = (y1 - y0).abs() > (x1 - x0).abs();

    if steep {
        (x0, y0) = (y0, x0);
        (x1, y1) = (y1, x1);
    }

    let reversed = x0 > x1;
    if reversed {
        (x0, x1) = (x1, x0);
        (y0, y1) = (y1, y0);
    }

    let dx = x1 - x0;
    let dy = y1 - y0;

    let gradient;
    const EPSILON: f32 = 1e-6;
    if dx.abs() < EPSILON {
        gradient = 1.0;
    } else {
        gradient = dy / dx;
    }

    let xend = x0.floor();
    let yend = y0 + gradient * (xend - x0);
    let xgap = 1.0 - (x0 - xend);
    let xend = xend as i32;
    let xpxl1 = xend;
    let ypxl1 = yend.floor() as i32;

    if !POLYLINE || reversed {
        if steep {
            plot(canvas, color, ypxl1,   xpxl1, rfpart(yend) * xgap);
            plot(canvas, color, ypxl1+1, xpxl1,  fpart(yend) * xgap);
        } else {
            plot(canvas, color, xpxl1, ypxl1,  rfpart(yend) * xgap);
            plot(canvas, color, xpxl1, ypxl1+1, fpart(yend) * xgap);
        }
    }
    let mut intery = yend + gradient;

    let xend = x1.ceil();
    let yend = y1 + gradient * (xend - x1);
    let xgap = 1.0 - (xend - x1);
    let xend = xend as i32;
    let xpxl2 = xend;
    let ypxl2 = yend.floor() as i32;

    if !POLYLINE || !reversed {
        if steep {
            plot(canvas, color, ypxl2,   xpxl2, rfpart(yend) * xgap);
            plot(canvas, color, ypxl2+1, xpxl2,  fpart(yend) * xgap);
        } else {
            plot(canvas, color, xpxl2, ypxl2,  rfpart(yend) * xgap);
            plot(canvas, color, xpxl2, ypxl2+1, fpart(yend) * xgap);
        }
    }

    if steep {
        for x in xpxl1+1..=xpxl2-1 {
            plot(canvas, color, intery.floor() as i32   , x, rfpart(intery));
            plot(canvas, color, intery.floor() as i32 +1, x,  fpart(intery));
            intery += gradient;
        }
    } else {
        for x in xpxl1+1..=xpxl2-1 {
            plot(canvas, color, x, intery.floor() as i32   , rfpart(intery));
            plot(canvas, color, x, intery.floor() as i32 +1,  fpart(intery));
            intery += gradient;
        }
    }

    fn fpart(x: f32) -> f32 {
        x.fract()
    }

    fn rfpart(x: f32) -> f32 {
        1.0 - fpart(x)
    }

    fn plot(canvas: &mut Canvas, color: Color, x: i32, y: i32, alpha: f32) {
        if x < 0 || y < 0 || x >= canvas.width() || y >= canvas.height() {
            return;
        }
        let pixel = canvas.at_mut(x, y);

        let alpha = (((alpha * 255.0) as u32 * color.alpha() as u32)/255) as u8;
        *pixel = pixel.blend(color.with_alpha(alpha));
    }
}

