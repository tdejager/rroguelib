use glium::glutin;
use std::error::Error;

use roguelib::Roguelib;

fn main() -> Result<(), Box<dyn Error>> {
    let window = roguelib::create_window("roguelike");

    let mut event_loop = glutin::EventsLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(window, context, &event_loop).unwrap();
    let mut roguelib = Roguelib::new(&display);

    let font_data = include_bytes!("../../fonts/square.ttf");

    roguelib.add_font(&display, "default", font_data, 18.0);

    // Receive the inputs
    let mut finished = false;
    loop {
        let display = &display;
        event_loop.poll_events(|event| {
            use self::glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => finished = true,
                    WindowEvent::Resized(_logical_size) => {
                        //                        let _dpi_factor = display.gl_window().get_hidpi_factor();
                        //
                        //                        // Reset the vertex buffers
                        //                        text_vertex_buffer = crate::util::create_text_vb(display, &glyphs, &cache);
                        //                        let grid =
                        //                            crate::util::create_grid(max_font_width, max_font_height, display);
                        //
                        //                        vb_grid = grid.0;
                        //                        ib_grid = grid.1;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keypress),
                                ..
                            },
                        ..
                    } => match keypress {
                        VirtualKeyCode::Escape => finished = true,
                        _ => (),
                    },
                    _ => {}
                }
            }
        });
        if finished {
            break;
        }

        roguelib.draw(&display, "default", "abcdefg@■");
    }

    Ok(())
}
