use std::{f32, num::NonZeroU32, rc::Rc};

use skrifa::{FontRef, MetadataProvider, instance::Location, outline::{DrawSettings, OutlinePen}, prelude::Size};
use winit::{event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::EventLoop, window::Window};

use vector_graphics::{
    affine::Affine, canvas::Canvas, color::{Color, colors}, path::{Path, PathElement, fill_path, stroke_path}, stroke::{Join, Stroke}, vec::Vec2
};

mod app;

const JBM_REGULAR: &'static [u8] = include_bytes!("../JetBrainsMono-2.304/fonts/ttf/JetBrainsMono-Regular.ttf");
// const INTER_REGULAR: &'static [u8] = include_bytes!("../Inter/Inter_18pt-Regular.ttf");
// const DEJAVU_SANS: &'static [u8] = include_bytes!("../DejaVuSans.ttf");
// const COMIC_MS: &'static [u8] = include_bytes!("../ComicMono.ttf");

fn intersects_circle(point: Vec2, center: Vec2, radius: f32) -> bool {
    return (center.x - point.x).abs() <= radius && (center.y - point.y).abs() <= radius;
}

const BACKGROUND: Color = Color::new(0xff1e1e1e);

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

    let stroke = Stroke { width: 10.0, join: Join::Miter { miter_limit: 4.0 } };

    for line in CHAR_SHEET {
        for char in line.chars() {
            let glyph_id = charmap.map(char).unwrap();

            let advance = glyph_metrics.advance_width(glyph_id).unwrap_or_default();
            let outline_glyph = outline_glyphs.get(glyph_id).unwrap();

            path.clear();
            let mut outline_pen = SkrifaOutlinePen::new(&mut path);
            outline_glyph.draw(DrawSettings::unhinted(Size::unscaled(), &Location::default()), &mut outline_pen).unwrap();

            fill_path(
                canvas,
                &path,
                transform * Affine::new([1.0, 0.0, 0.0, -1.0, pen_x, pen_y]),
                Color::new(0xa0ffffff));
            
            stroke_path(
                canvas,
                &path,
                &stroke,
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

    let event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    let mut mouse_position = Vec2::new(0.0, 0.0);
    let mut transform = Affine::IDENTITY;

    let mut previous_position: Option<Vec2> = None;

    // let mut path = Path::new();
    // let mut cubic = Cubic::new(Vec2::new(100.0, 100.0), Vec2::new(200.0, 50.0), Vec2::new(400.0, 50.0), Vec2::new(500.0, 100.0));
    // let mut approx = cubic.clone();
    // let mut max_error = approx.evaluate(0.0);
    // let mut active_element: Option<usize> = None;


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
                let mut canvas = Canvas::from_raw_pixels(&mut buffer, width, height, width);
                canvas.clear(BACKGROUND);

                draw_glyph_sheet(&mut canvas, &font, transform * Affine::scale(SHEET_SCALE));



                // path.clear();
                // // how do you do stroking?
                // compute_offset_curve(&mut path, &cubic, 10.0);
                // // are you supposed to just "invert" the curve like this? Probably not?
                // compute_offset_curve(&mut path, &Cubic { p0: cubic.p3, p1: cubic.p2, p2: cubic.p1, p3: cubic.p0 }, 10.0);
                // path.push(PathElement::Close);
                // get_offset_curve(&mut path, cubic.clone(), -5.0);
                // draw_path_hairline(&mut canvas, &path, transform, colors::WHITE);
                // fill_path(&mut canvas, &path, transform, colors::WHITE);

                // path.clear();
                // path.push_cubic(&cubic);
                // draw_path_hairline(&mut canvas, &path, transform, colors::LIME);

                // primitives::line(&mut canvas, cubic.p0, cubic.p1, colors::GRAY);
                // primitives::line(&mut canvas, cubic.p2, cubic.p3, colors::GRAY);

                // primitives::circle(&mut canvas, cubic.p0, 7.0, colors::RED);
                // primitives::circle(&mut canvas, cubic.p3, 7.0, colors::RED);

                // primitives::circle(&mut canvas, cubic.p1, 7.0, colors::AQUA);
                // primitives::circle(&mut canvas, cubic.p2, 7.0, colors::AQUA);

                // if let Some(max_error) = max_error {
                //     primitives::circle(&mut canvas, max_error, 7.0, colors::RED);
                // }

                buffer.present().unwrap();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(previous_position) = &mut previous_position {
                    transform = Affine::translate(mouse_position - *previous_position) * transform;
                    *previous_position = mouse_position;
                    window.request_redraw();
                }
                mouse_position = Vec2::new(position.x as f32, position.y as f32);

                // let point = match active_element {
                //     Some(0) => &mut cubic.p0,
                //     Some(1) => &mut cubic.p1,
                //     Some(2) => &mut cubic.p2,
                //     Some(3) => &mut cubic.p3,
                //     _ => return,
                // };

                // *point = mouse_position;

                // window.request_redraw();


            }
            // WindowEvent::KeyboardInput { event, .. } => {
            //     if event.state != ElementState::Pressed {
            //         return;
            //     }
            //     if let Key::Character(str) = event.logical_key && str == "c" {
            //         (approx, max_error) = get_offset_curve(&cubic, 20.0);
            //         window.request_redraw();
            //     }
            // }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => previous_position = Some(mouse_position),
                    ElementState::Released => previous_position = None,
                }
                // match state {
                //     ElementState::Pressed => {
                //         if intersects_circle(mouse_position, cubic.p0, 10.0) {
                //             active_element = Some(0);
                //         } else if intersects_circle(mouse_position, cubic.p1, 10.0) {
                //             active_element = Some(1);
                //         } else if intersects_circle(mouse_position, cubic.p2, 10.0) {
                //             active_element = Some(2);
                //         } else if intersects_circle(mouse_position, cubic.p3, 10.0) {
                //             active_element = Some(3);
                //         } else {
                //             active_element = None;
                //         }

                //         window.request_redraw();
                //     },
                //     ElementState::Released => active_element = None,
                // }
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

