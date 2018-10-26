#[macro_use]
extern crate glium;
extern crate rusttype;
extern crate unicode_normalization;

use rusttype::gpu_cache::CacheBuilder;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};

use glium::{glutin, Surface};
use std::error::Error;
use rusttype::gpu_cache::Cache;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

implement_vertex!(Vertex, position, color);

fn create_program(display: &glium::Display) -> glium::program::Program {
    let program = program!(display,
     140 => {
        vertex: "
                #version 140
                uniform mat4 matrix;
                in vec2 position;
                in vec3 color;
                out vec3 vColor;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 140
                in vec3 vColor;
                out vec4 f_color;
                void main() {
                    f_color = vec4(vColor, 1.0);
                }
            "
            }
    );

    program.unwrap()
}


fn layout_grid<'a>(font: &'a Font, scale: Scale, width: u32, text: &str)
                   -> Vec<PositionedGlyph<'a>>
{
    use unicode_normalization::UnicodeNormalization;
    let mut result = Vec::new();
    let mut caret = point(0.0, 0.0);

    let v_metrics = font.v_metrics(scale);
    let advance_height = 100.0;

    for c in text.nfc() {
        let base_glyph = font.glyph(c);
        let mut glyph = base_glyph.scaled(scale).positioned(caret);

        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph = glyph.into_unpositioned().positioned(caret);
            } else {
                caret.x += glyph.unpositioned().h_metrics().advance_width as f32;
            }
        }
        result.push(glyph);
    }

    return result;
}

fn rescale(f: f32) -> f32 {
    -1 as f32 + (f / 1.0 as f32) * 2 as f32
}

fn create_grid(
    width: u32,
    height: u32,
    grid_x: u32,
    grid_y: u32,
    display: &glium::Display,
) -> (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>) {
    let mut vertices: Vec<Vertex> = Vec::new();

    for y in (0..height).step_by(grid_y as usize) {
        vertices.push(Vertex {
            position: [-1.0, -rescale(y as f32 / height as f32)],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [1.0, -rescale(y as f32 / height as f32)],
            color: [1.0, 1.0, 1.0],
        });
    }

    for x in (0..width).step_by(grid_x as usize) {
        vertices.push(Vertex {
            position: [rescale(x as f32 / width as f32), -1.0],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [rescale(x as f32 / width as f32), 1.0],
            color: [1.0, 1.0, 1.0],
        });
    }


    let vb = glium::VertexBuffer::new(display, &vertices).unwrap();
    let indices: Vec<u16> = (0..height as u16).collect();
    //    let indices : Vec<u16> = vec![0, 1, 2, 3, 4, 5]; //2, 3, 3, 4, 4, 5, 5, 6];
    let ib =
        glium::IndexBuffer::new(display, glium::index::PrimitiveType::LinesList, &indices).unwrap();

    return (vb, ib);
}

fn main() -> Result<(), Box<Error>> {
    let window = glutin::WindowBuilder::new()
        .with_dimensions((512, 512).into())
        .with_title("RogueLike test");

    let context = glutin::ContextBuilder::new().with_vsync(true);

    let mut event_loop = glutin::EventsLoop::new();
    let display = glium::Display::new(window, context, &event_loop)?;

    // Get the dpi factor
    let dpi_factor = display.gl_window().get_hidpi_factor();

    let (width, height): (u32, u32) = display
        .gl_window()
        .get_inner_size()
        .ok_or("get_inner_size")?
        .to_physical(dpi_factor)
        .into();

    println!("Running with a dpi factor of {}", dpi_factor);
    println!("Pre dpi width {:?}", display.gl_window().get_inner_size());
    println!("Width after dpi transform {}", width);

    let mut finished = false;

    // building the uniforms
    let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ]
    };

    println!("Creating a grid with w: {} and h: {}", width, height);
    let (vb, ib) = create_grid(
        width,
        height,
        12 * dpi_factor as u32,
        12 * dpi_factor as u32,
        &display,
    );

    let (cache_width, cache_height) = ((512.0 * dpi_factor) as u32, (512.0 * dpi_factor) as u32);
    let mut cache = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

    let program = create_program(&display);

    loop {
        event_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => finished = true,
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

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target.draw(&vb, &ib, &program, &uniforms, &Default::default())?;
        target.finish()?;
    }

    Ok(())
}

