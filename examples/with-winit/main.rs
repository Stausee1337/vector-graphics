use std::{f32, num::NonZeroU32, rc::Rc};

use skrifa::{FontRef, MetadataProvider, instance::Location, outline::{DrawSettings, OutlinePen}, prelude::Size};
use winit::{event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::EventLoop, window::Window};

use vector_graphics::{
    affine::Affine, canvas::Canvas, color::{Color, colors}, path::{Path, fill_path, stroke_path}, stroke::{Join, Stroke}, vec::Vec2
};

mod app;

const JBM_REGULAR: &'static [u8] = include_bytes!("../../assets/JetBrainsMono-2.304/JetBrainsMono-Regular.ttf");

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
        self.path.move_to(Vec2::new(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(Vec2::new(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(Vec2::new(cx0, cy0), Vec2::new(x, y));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(Vec2::new(cx0, cy0), Vec2::new(cx1, cy1), Vec2::new(x, y)); 
    }

    fn close(&mut self) {
        self.path.close(); 
    }
}

const CHAR_SHEET: [&'static str; 3] = [
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "abcdefghijklmnopqrstuvwxyz",
    "0123456789?!{}()+-"
];

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
const BACKGROUND: Color = Color::new(0xff1e1e1e);

fn main() {
    let font = FontRef::new(JBM_REGULAR).unwrap();

    let event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    let mut mouse_position = Vec2::new(0.0, 0.0);
    let mut transform = Affine::IDENTITY;

    let mut previous_position: Option<Vec2> = None;


    let mut app = app::WinitAppBuilder::with_init(
        |elwt| {
            let window = elwt.create_window(Window::default_attributes());
            Rc::new(window.unwrap())
        },
        |_elwt, window| softbuffer::Surface::new(&context, window.clone()).unwrap(),
    )
    
    .with_event_handler(|window, surface, window_id, event, elwt| {
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

                buffer.present().unwrap();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(previous_position) = &mut previous_position {
                    transform = Affine::translate(mouse_position - *previous_position) * transform;
                    *previous_position = mouse_position;
                    window.request_redraw();
                }
                mouse_position = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => previous_position = Some(mouse_position),
                    ElementState::Released => previous_position = None,
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

