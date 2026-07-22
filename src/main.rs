use std::{f32, num::NonZeroU32, rc::Rc};

use skrifa::{FontRef, MetadataProvider, instance::Location, outline::{DrawSettings, OutlinePen}, prelude::Size};
use winit::{event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::EventLoop, window::Window};
use draw::{colors, primitives, draw_path, Canvas, Path, PathElement, Cubic, Vec2};

use crate::draw::Affine;

mod app;
mod draw;

const JBM_REGULAR: &'static [u8] = include_bytes!("../JetBrainsMono-2.304/fonts/ttf/JetBrainsMono-Regular.ttf");


fn intersects_circle(point: &Vec2, center: &Vec2, radius: f32) -> bool {
    return (center.x - point.x).abs() <= radius && (center.y - point.y).abs() <= radius;
}

pub struct SkrifaOutlinePen<'a> {
    path: &'a mut Path
}

impl<'a> SkrifaOutlinePen<'a> {
    fn new(path: &'a mut Path) -> Self {
        SkrifaOutlinePen { path }
    }
}

impl<'a> OutlinePen for SkrifaOutlinePen<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.push(PathElement::MoveTo(Vec2::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.push(PathElement::LineTo(Vec2::new(x, y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.push(PathElement::QuadTo(Vec2::new(cx0, cy0), Vec2::new(x, y))); 
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.push(PathElement::CurveTo(Vec2::new(cx0, cy0), Vec2::new(cx1, cy1), Vec2::new(x, y))); 
    }

    fn close(&mut self) {
        self.path.push(PathElement::Close); 
    }
}

const CHAR_SHEET: [&'static str; 3] = ["ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz", "0123456789?!{}()+-"];

fn meassure_glyph_sheet(font: &FontRef) -> Vec2 {
    let charmap = font.charmap();

    let location = Location::default();

    let metrics = font.metrics(Size::unscaled(), &Location::default());
    let line_height = metrics.ascent - metrics.descent + metrics.leading;
    let glyph_metrics = font.glyph_metrics(Size::unscaled(), &location);
    let mut pen_x = 0_f32;
    let mut pen_y = metrics.ascent;

    let mut width = 0.0;

    for line in CHAR_SHEET {
        for char in line.chars() {
            let glyph_id = charmap.map(char).unwrap();
            let advance = glyph_metrics.advance_width(glyph_id).unwrap_or_default();
            pen_x += advance;
        }
        width = pen_x.max(width);

        pen_y += line_height;
        pen_x = 0.0;
    }
    Vec2::new(width, pen_y)
}

fn draw_glyph_sheet(canvas: &mut Canvas, font: &FontRef, transform: Affine) {
    let charmap = font.charmap();
    let outline_glyphs = font.outline_glyphs();

    let location = Location::default();

    let metrics = font.metrics(Size::unscaled(), &Location::default());
    let line_height = metrics.ascent - metrics.descent + metrics.leading;
    let glyph_metrics = font.glyph_metrics(Size::unscaled(), &location);
    let mut pen_x = 0_f32;
    let mut pen_y = metrics.ascent;

    let mut path = Path::new();
    let mut width = 0.0;

    for line in CHAR_SHEET {
        for char in line.chars() {
            let glyph_id = charmap.map(char).unwrap();

            let advance = glyph_metrics.advance_width(glyph_id).unwrap_or_default();
            let outline_glyph = outline_glyphs.get(glyph_id).unwrap();

            path.clear();
            let mut outline_pen = SkrifaOutlinePen::new(&mut path);
            outline_glyph.draw(DrawSettings::unhinted(Size::unscaled(), &Location::default()), &mut outline_pen).unwrap();

            draw_path(
                canvas,
                &path,
                transform * Affine::new([1.0, 0.0, 0.0, -1.0, pen_x, pen_y]),
                colors::WHITE);

            pen_x += advance;
        }
        width = pen_x.max(width);

        pen_y += line_height;
        pen_x = 0.0;
    }

}

const SHEET_SCALE: f32 = 0.1;

fn main() {
    let font = FontRef::new(JBM_REGULAR).unwrap();
    // let sheet_size = meassure_glyph_sheet(&font) * SHEET_SCALE;
    // let glyph_id = font.charmap().map(0x6au32).unwrap();
    // let outline_glyph = font.outline_glyphs().get(glyph_id).unwrap();

    // let mut path = Path::new();
    // let mut outline_pen = SkrifaOutlinePen::new(&mut path);
    // outline_glyph.draw(DrawSettings::unhinted(Size::unscaled(), &Location::default()), &mut outline_pen).unwrap();
    // 
    let event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    let mut mouse_position = Vec2::new(0.0, 0.0);
    let mut transform = Affine::IDENTITY;

    // let mut points: Vec<Vec2> = vec![];
    let mut control_points = [Vec2::new(100.0, 100.0), Vec2::new(200.0, 50.0), Vec2::new(400.0, 50.0), Vec2::new(500.0, 100.0)];

    let mut active_element: Option<usize> = None;

    let mut app = app::WinitAppBuilder::with_init(
        |elwt| {
            let window = elwt.create_window(Window::default_attributes());
            Rc::new(window.unwrap())
        },
        |_elwt, window| softbuffer::Surface::new(&context, window.clone()).unwrap(),
    )
    
    .with_event_handler(|window, surface, window_id, event, elwt| {
        // elwt.set_control_flow(ControlFlow::Poll);

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
                // let screen_size = Vec2::new(width as f32, height as f32);
                let mut canvas = Canvas::from_raw_pixels(&mut buffer, width, height, width);
                canvas.clear();

                // primitives::circle(&mut canvas, Vec2::new(50.0, 50.0), 7.0, colors::LIME);
                draw_glyph_sheet(&mut canvas, &font, transform * Affine::scale(SHEET_SCALE));

                // draw_path(
                //     &mut canvas,
                //     &path,
                //     Affine::translate(Vec2::new(0.0, 0.0)) * Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 0.0]),
                //     colors::AQUA);
 
                // points.clear();
                // let cubic = Cubic::new(control_points[0], control_points[1], control_points[2], control_points[3]);
                // cubic.flatten(&mut points, Affine::IDENTITY);
                // if points.len() > 2 {
                //     primitives::polygon(&mut canvas, &points, &[(0, points.len() - 1)], colors::WHITE);
                // }

                // primitives::line(&mut canvas, cubic.p0, cubic.p1, colors::LIME);
                // primitives::line(&mut canvas, cubic.p2, cubic.p3, colors::LIME);

                // primitives::circle(&mut canvas, cubic.p0, 7.0, colors::RED);
                // primitives::circle(&mut canvas, cubic.p3, 7.0, colors::RED);

                // primitives::circle(&mut canvas, cubic.p1, 7.0, colors::AQUA);
                // primitives::circle(&mut canvas, cubic.p2, 7.0, colors::AQUA);

                buffer.present().unwrap();
            }
            WindowEvent::CursorMoved { position, .. } => {
                mouse_position = Vec2::new(position.x as f32, position.y as f32);

                let point = match active_element {
                    Some(idx) => &mut control_points[idx],
                    None => return,
                };

                *point = mouse_position;

                window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        for (idx, point) in control_points.iter().enumerate() {
                            if intersects_circle(&mouse_position, point, 10.0) {
                                active_element = Some(idx);
                                return;
                            }
                        }
                        window.request_redraw();
                    },
                    ElementState::Released => active_element = None,
                }
            }
            WindowEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_dx, dy), .. } => {
                transform = Affine::translate(mouse_position) * Affine::scale((1.2f32).powf(dy)) * Affine::translate(-mouse_position) * transform;
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            _ => {}
        }
    });

    event_loop.run_app(&mut app).unwrap();
}

