use std::str::Split;

use vector_graphics::{affine::Affine, canvas::Canvas, color::{Color}, path::{Path, fill_path, stroke_path}, stroke::{Join, Stroke}, vec::Vec2};

use crate::tiger::ELEMENTS;

pub struct Element {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: Option<f32>,
    pub commands: &'static str
}

mod tiger {
    include!("../assets/ghostscript-tiger.rs");
}

fn parse_commands(commands: &'static str) -> Option<Path> {
    let mut split = commands.split(' ');
    let mut path = Path::new();

    while let Some(cmd) = split.next() {
        match cmd {
            "M" => {
                let p = next_point(&mut split)?;
                path.move_to(p);
            }
            "L" => {
                if path.is_empty() { return None; }
                let p = next_point(&mut split)?;
                path.line_to(p);
            }
            "Q" => {
                if path.is_empty() { return None; }
                let p1 = next_point(&mut split)?;
                let p2 = next_point(&mut split)?;
                path.quad_to(p1, p2);
            }
            "C" => {
                if path.is_empty() { return None; }
                let p1 = next_point(&mut split)?;
                let p2 = next_point(&mut split)?;
                let p3 = next_point(&mut split)?;
                path.curve_to(p1, p2, p3);
            }
            "Z" => {
                if path.is_empty() { return None; }
                path.close();
            }
            _ => return None,
        }
    }

    return Some(path);

    fn next_point<'a>(split: &mut Split<'a, char>) -> Option<Vec2> {
        Some(Vec2 {
            x: next_float(split)?,
            y: next_float(split)?,
        })
    }

    fn next_float<'a>(split: &mut Split<'a, char>) -> Option<f32> {
        split.next()?.parse().ok()
    }
}

struct DrawData {
    fill: Option<Color>,
    stroke: Option<(Stroke, Color)>,
    path: Path,
}

// matrix(1.7656465 0 0 1.7656465 324.90717 255.00943)
// matrix(<a> <b> <c> <d> <e> <f>)
// |a c e|
// |b d f|
// |0 0 1|

fn map_color(color: Color) -> Color {
    Color::from_rgba(color.blue(), color.green(), color.red(), color.alpha())
}

fn parse_element(element: &Element) -> DrawData {
    let path = parse_commands(element.commands).unwrap();
    DrawData {
        fill: element.fill.map(map_color),
        stroke: element.stroke.map(|x| (Stroke { width: element.stroke_width.unwrap_or(1.0), join: Join::Miter { miter_limit: 4.0 }}, map_color(x))),
        path 
    }
}

const WIDTH: usize = 900;
const HEIGHT: usize = 900;
const TRANSFORM: Affine = Affine::new([1.7656465, 0.0, 0.0, 1.7656465, 324.90717, 255.00943]);

fn render_draw_data(canvas: &mut Canvas, data: &DrawData) {
    if let Some(fill) = data.fill {
        fill_path(canvas, &data.path, TRANSFORM, fill);
    }

    if let Some((stroke, color)) = data.stroke {
        stroke_path(canvas, &data.path, &stroke, TRANSFORM, color);
    }
}

fn main() {
    let mut draw_data = vec![];
    for element in ELEMENTS {
        draw_data.push(parse_element(&element));
    }

    let mut pixels = vec![u32::MAX; WIDTH * HEIGHT].into_boxed_slice();
    let mut canvas = Canvas::from_raw_pixels(&mut pixels, WIDTH, HEIGHT, WIDTH);

    for data in &draw_data {
        render_draw_data(&mut canvas, data);
    }

    image::save_buffer_with_format("ghostscript-tiger.png", bytemuck::cast_slice(&pixels), WIDTH as u32, HEIGHT as u32, image::ExtendedColorType::Rgba8, image::ImageFormat::Png).unwrap();
}


