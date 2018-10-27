#[macro_use]
extern crate glium;
extern crate rusttype;
extern crate unicode_normalization;

use rusttype::gpu_cache::CacheBuilder;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};

use glium::{glutin, Surface};
use std::error::Error;
use rusttype::gpu_cache::Cache;
use std::borrow::Cow;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

implement_vertex!(Vertex, position, color);


#[derive(Copy, Clone)]
struct TextVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    colour: [f32; 4],
}

implement_vertex!(TextVertex, position, tex_coords, colour);


fn create_grid_program(display: &glium::Display) -> glium::program::Program {
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

fn create_text_program(display: &glium::Display) -> glium::program::Program {
    let program = program!(
        display,
        140 => {
            vertex: "
                #version 140

                in vec2 position;
                in vec2 tex_coords;
                in vec4 colour;

                out vec2 v_tex_coords;
                out vec4 v_colour;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_tex_coords = tex_coords;
                    v_colour = colour;
                }
            ",

            fragment: "
                #version 140
                uniform sampler2D tex;
                in vec2 v_tex_coords;
                in vec4 v_colour;
                out vec4 f_colour;

                void main() {
                    f_colour = v_colour * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);
                }
            "
        });

    program.unwrap()
}

fn create_text_vb(display: &glium::Display, glyphs: &Vec<PositionedGlyph>, cache: &Cache) -> glium::VertexBuffer<TextVertex>
{
    let vertex_buffer = {
        let colour = [1.0, 1.0, 1.0, 1.0];
        let (screen_width, screen_height) = {
            let (w, h) = display.get_framebuffer_dimensions();
            (w as f32, h as f32)
        };
        let origin = point(0.0, 0.0);
        let vertices: Vec<TextVertex> = glyphs
            .iter()
            .flat_map(|g| {
                if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
                    let gl_rect = Rect {
                        min: origin
                            + (vector(
                            screen_rect.min.x as f32 / screen_width - 0.5,
                            1.0 - screen_rect.min.y as f32 / screen_height - 0.5,
                        )) * 2.0,
                        max: origin
                            + (vector(
                            screen_rect.max.x as f32 / screen_width - 0.5,
                            1.0 - screen_rect.max.y as f32 / screen_height - 0.5,
                        )) * 2.0,
                    };
                    arrayvec::ArrayVec::<[TextVertex; 6]>::from([
                        TextVertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            colour,
                        },
                        TextVertex {
                            position: [gl_rect.min.x, gl_rect.min.y],
                            tex_coords: [uv_rect.min.x, uv_rect.min.y],
                            colour,
                        },
                        TextVertex {
                            position: [gl_rect.max.x, gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            colour,
                        },
                        TextVertex {
                            position: [gl_rect.max.x, gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            colour,
                        },
                        TextVertex {
                            position: [gl_rect.max.x, gl_rect.max.y],
                            tex_coords: [uv_rect.max.x, uv_rect.max.y],
                            colour,
                        },
                        TextVertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            colour,
                        },
                    ])
                } else {
                    arrayvec::ArrayVec::new()
                }
            }).collect();

        glium::VertexBuffer::new(display, &vertices).expect("Could not create text vertex buffer")
    };

    vertex_buffer
}


fn layout_grid<'a>(font: &'a Font, scale: Scale, width: u32, text: &str)
                   -> Vec<PositionedGlyph<'a>>
{
    use unicode_normalization::UnicodeNormalization;
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);

    let mut caret = point(0.0, v_metrics.ascent);

    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    println!("advance height {}", advance_height - v_metrics.line_gap);

    for c in text.nfc() {
        let base_glyph = font.glyph(c);
        let mut glyph = base_glyph.scaled(scale).positioned(caret);

        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph = glyph.into_unpositioned().positioned(caret);
            } else {
                caret.x += glyph.unpositioned().h_metrics().advance_width as f32;
                println!("Glyph {} height {}", c, bb.height());
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
    width: f32,
    height: f32,
    grid_x: f32,
    grid_y: f32,
    display: &glium::Display,
) -> (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>) {
    let mut vertices: Vec<Vertex> = Vec::new();

    for y in 0..(height/ grid_y) as u32 {
        vertices.push(Vertex {
            position: [-1.0, -rescale(y as f32 * grid_y / height)],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [1.0, -rescale(y as f32 * grid_y / height)],
            color: [1.0, 1.0, 1.0],
        });
    }

    for x in 0..(width / grid_x) as u32 {
        vertices.push(Vertex {
            position: [rescale(x as f32 * grid_x / width), -1.0],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [rescale(x as f32 * grid_x / width), 1.0],
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

    let font_data = include_bytes!("../fonts/consola.ttf");
    let font = Font::from_bytes(font_data as &[u8])?;

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
    let uniforms_grid = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ]
    };

    //let max_font_height = font.glyph("█").scaled(Scale::uniform(24.0 * dpi_factor as f32))

    let v_metrics = font.v_metrics(Scale::uniform(24.0 * dpi_factor as f32));
    let font_height = v_metrics.ascent + v_metrics.descent;

    use unicode_normalization::UnicodeNormalization;
    let box_char: String = "█".into();

    let mut max_font_height = 0.0;
    let mut max_font_width = 0.0;

    for c in box_char.nfc() {
        let v_metrics = font.v_metrics(Scale::uniform(24.0 * dpi_factor as f32));
        let glyph = font.glyph(c).scaled(Scale::uniform(24.0 * dpi_factor as f32));
        let bounding_box = glyph.clone().positioned(point(0.0, 0.0)).pixel_bounding_box().unwrap();

        max_font_height = (v_metrics.ascent - v_metrics.descent);
        max_font_width = glyph.h_metrics().advance_width;
    };

//    let max_font_height = font.glyph("█".into().nfc()[0]).scaled(Scale::uniform(24.0 * dpi_factor as f32)).positioned(point(0.0, 0.0)).pixel_bounding_box().unwrap().height();

    println!("font-height: {} max font heigt: {}", font_height, max_font_height);

    let grid_program = create_grid_program(&display);
    println!("Creating a grid with w: {} and h: {}", width, height);
    let (vb_grid, ib_grid) = create_grid(
        width as f32,
        height as f32,
        max_font_width,
        max_font_height,
        &display,
    );

    // Create the font cache
    let (cache_width, cache_height) = ((512.0 * dpi_factor) as u32, (512.0 * dpi_factor) as u32);
    let mut cache = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

    let text: String = "Halllooo ben jij bas? █".into();

    let glyphs = layout_grid(&font,
                             Scale::uniform(24.0 * dpi_factor as f32), width, &text);

    // Create the texture
    let cache_tex = glium::texture::Texture2d::with_format(
        &display,
        glium::texture::RawImage2d {
            data: Cow::Owned(vec![128u8; cache_width as usize * cache_height as usize]),
            width: cache_width,
            height: cache_height,
            format: glium::texture::ClientFormat::U8,
        },
        glium::texture::UncompressedFloatFormat::U8,
        glium::texture::MipmapsOption::NoMipmap,
    )?;

    for glyph in &glyphs {
        cache.queue_glyph(0, glyph.clone());
    }

    cache.cache_queued(|rect, data| {
        cache_tex.main_level().write(
            glium::Rect {
                left: rect.min.x,
                bottom: rect.min.y,
                width: rect.width(),
                height: rect.height(),
            },
            glium::texture::RawImage2d {
                data: Cow::Borrowed(data),
                width: rect.width(),
                height: rect.height(),
                format: glium::texture::ClientFormat::U8,
            },
        );
    })?;


    let text_programs = create_text_program(&display);
    let text_vertex_buffer = create_text_vb(&display, &glyphs, &cache);
    let text_uniforms = uniform! {
            tex: cache_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        };


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
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        // Draw the grid lines
        target.draw(&vb_grid,
                    &ib_grid,
                    &grid_program,
                    &uniforms_grid,
                    &Default::default())?;

        // Draw the text
        target.draw(
            &text_vertex_buffer,
            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &text_programs,
            &text_uniforms,
            &glium::DrawParameters {
                blend: glium::Blend::alpha_blending(),
                ..Default::default()
            },
        )?;

        target.finish()?;
    }

    Ok(())
}

