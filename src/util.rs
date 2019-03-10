use crate::vertex::{TextVertex, Vertex};
use rusttype::gpu_cache::Cache;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};

/// Create the text vertex buffer, this is the buffer that contains text rectangles
pub fn create_text_vb(
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

/// Layout for a specific font in a grid
pub fn layout_grid<'a>(
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

/// Create a grid, lines for easy display
pub fn create_grid(
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
