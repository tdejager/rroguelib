#[macro_use]
extern crate glium;
extern crate rusttype;
extern crate unicode_normalization;

use rusttype::gpu_cache::CacheBuilder;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};

use glium::{glutin, Surface};
use rusttype::gpu_cache::Cache;
use std::borrow::Cow;
use std::error::Error;

mod program;
mod vertex;

use self::vertex::{TextVertex, Vertex};

fn create_text_vb(
    display: &glium::Display,
    glyphs: &Vec<PositionedGlyph>,
    cache: &Cache,
) -> glium::VertexBuffer<TextVertex> {
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
            })
            .collect();

        glium::VertexBuffer::new(display, &vertices).expect("Could not create text vertex buffer")
    };

    vertex_buffer
}

fn layout_grid<'a>(
    font: &'a Font,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    use unicode_normalization::UnicodeNormalization;
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);

    let mut caret = point(0.0, v_metrics.ascent);

    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

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

/// Rescale from 0..1 to -1..1
fn rescale(f: f32) -> f32 {
    -1 as f32 + (f / 1.0 as f32) * 2 as f32
}

fn create_grid(
    grid_x: f32,
    grid_y: f32,
    display: &glium::Display,
) -> (glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>) {
    let mut vertices: Vec<Vertex> = Vec::new();
    let (screen_width, screen_height) = {
        let (w, h) = display.get_framebuffer_dimensions();
        (w as f32, h as f32)
    };

    for y in 0..(screen_height / grid_y) as u32 {
        vertices.push(Vertex {
            position: [-1.0, -rescale(y as f32 * grid_y / screen_height)],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [1.0, -rescale(y as f32 * grid_y / screen_height)],
            color: [1.0, 1.0, 1.0],
        });
    }

    for x in 0..(screen_width / grid_x) as u32 {
        vertices.push(Vertex {
            position: [rescale(x as f32 * grid_x / screen_width), -1.0],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [rescale(x as f32 * grid_x / screen_width), 1.0],
            color: [1.0, 1.0, 1.0],
        });
    }

    let vb = glium::VertexBuffer::new(display, &vertices).unwrap();
    let indices: Vec<u16> = (0..screen_height as u16).collect();
    let ib =
        glium::IndexBuffer::new(display, glium::index::PrimitiveType::LinesList, &indices).unwrap();

    return (vb, ib);
}

fn main() -> Result<(), Box<Error>> {
    let window = glutin::WindowBuilder::new()
        .with_dimensions((1920, 1080).into())
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

    println!("Pre dpi width {:?}", display.gl_window().get_inner_size());

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
    let scale = Scale::uniform(20.0 * dpi_factor as f32);
    let v_metrics = font.v_metrics(scale);
    let font_height = v_metrics.ascent + v_metrics.descent;

    use unicode_normalization::UnicodeNormalization;
    let box_char: String = "█".into();

    let v_metrics = font.v_metrics(scale);
    let max_font_height = v_metrics.ascent - v_metrics.descent; // + v_metrics.line_gap;
    let mut max_font_width = 0.0;

    for c in box_char.nfc() {
        let glyph = font.glyph(c).scaled(scale);
        let bounding_box = glyph
            .clone()
            .positioned(point(0.0, 0.0))
            .pixel_bounding_box()
            .unwrap();
        max_font_width = glyph.h_metrics().advance_width;
    }

    // Create the font cache
    let (cache_width, cache_height) = (width as u32, height as u32);
    let mut cache = Cache::builder()
        .dimensions(cache_width, cache_height)
        .build();

    let text: String = "Halllooo ben jij bas? █ ╦═══╦═══════".into();

    let glyphs = layout_grid(&font, scale, width, &text);

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

    let grid_program = program::create_grid_program(&display);
    let text_programs = program::create_text_program(&display);

    let text_uniforms = uniform! {
        tex: cache_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
    };

    let mut text_vertex_buffer = create_text_vb(&display, &glyphs, &cache);
    let (mut vb_grid, mut ib_grid) = create_grid(max_font_width, max_font_height, &display);

    loop {
        event_loop.poll_events(|event| {
            use self::glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => finished = true,
                    WindowEvent::Resized(logical_size) => {
                        let dpi_factor = display.gl_window().get_hidpi_factor();

                        // Reset the vertex buffers
                        text_vertex_buffer = create_text_vb(&display, &glyphs, &cache);
                        let grid = create_grid(max_font_width, max_font_height, &display);

                        vb_grid = grid.0;
                        ib_grid = grid.1;
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

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        // Draw the grid lines
        target.draw(
            &vb_grid,
            &ib_grid,
            &grid_program,
            &uniforms_grid,
            &Default::default(),
        )?;

        // Draw the text
        target.draw(
            &text_vertex_buffer,
            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &text_programs,
            &text_uniforms,
            &glium::DrawParameters {
                blend: glium::Blend::alpha_blending(),
                //                polygon_mode: glium::PolygonMode::Line,
                ..Default::default()
            },
        )?;

        target.finish()?;
    }

    Ok(())
}
