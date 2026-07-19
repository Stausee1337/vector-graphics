use std::{num::NonZeroU32, rc::Rc};

use winit::{event::{ElementState, MouseButton, WindowEvent}, event_loop::EventLoop, window::Window};
use draw::{primitives, Canvas, Qudaratic, Vec2};

mod app;
mod draw;

// const JBM_REGULAR: &'static [u8] = include_bytes!("../JetBrainsMono-2.304/fonts/ttf/JetBrainsMono-Regular.ttf");


fn intersects_circle(point: &Vec2, center: &Vec2, radius: f32) -> bool {
    return (center.x - point.x).abs() <= radius && (center.y - point.y).abs() <= radius;
}

fn main() {
    // let font = skrifa::FontRef::new(JBM_REGULAR).unwrap();
    // let glyph_id = font.charmap().map(0x42u32).unwrap();
    // let outline_glyph = font.outline_glyphs().get(glyph_id).unwrap();
    // outline_glyph.draw(settings, pen);
    
    let event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    let mut mouse_position = Vec2::new(0.0, 0.0);

    let mut points: Vec<Vec2> = vec![];
    let mut control_points = [Vec2::new(100.0, 100.0), Vec2::new(300.0, 50.0), Vec2::new(500.0, 100.0)];

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
                let mut canvas = Canvas::from_raw_pixels(&mut buffer, width, height, width);
                canvas.clear();


                // if points.len() > 2 {
                //     polygon(&mut canvas, &points, 0xffffffff);
                //     let start = points.first().unwrap();
                //     let end = points.last().unwrap();
                //     line(&mut canvas, start, end, 0xff00ff00);
                // }

                // let mut x = points.iter();
                // x.next();
                
                points.clear();
                let quadratic = Qudaratic::new(control_points[0], control_points[1], control_points[2]);
                quadratic.flatten(&mut points);
                if points.len() > 2 {
                    primitives::polygon(&mut canvas, &points, 0xffffffff);
                }

                primitives::line(&mut canvas, control_points[0], control_points[1], 0xff00ff00);
                primitives::line(&mut canvas, control_points[1], control_points[2], 0xff00ff00);

                // const MAX_POINTS: usize = 30;
                // let mut prev = control_points[0];
                // for i in 0..MAX_POINTS {
                //     let t = (i + 1) as f32/MAX_POINTS as f32;
                //     let p = quadratic.evaluate(t);
                //     line(&mut canvas, &prev, &p, 0xff00ff00);
                //     prev = p;
                // }

                for point in control_points.iter() {
                    primitives::circle(&mut canvas, *point, 10.0, 0xffff0000);
                }

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
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            _ => {}
        }
    });

    event_loop.run_app(&mut app).unwrap();
}

