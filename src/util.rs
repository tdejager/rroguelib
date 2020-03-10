use crate::vertex::{TextVertex, Vertex};
use rusttype::gpu_cache::Cache;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale, Vector};
use unicode_normalization::UnicodeNormalization;

/// Create the text vertex buffer, this is the buffer that contains text rectangles
pub(crate) fn create_text_vb(
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
                    // Scale between 0..1 to -1..1
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

                    // Associate vertices with texture coords
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
pub(crate) fn layout_grid<'a>(
    font: &Font<'a>,
    scale: Scale,
    _width: u32,
    grid: &LineGrid,
    text: &str,
) -> Vec<PositionedGlyph<'a>> {
    let mut result = Vec::new();
    let mut index = grid.totals.x;
    for c in text.nfc() {
        let base_glyph = font.glyph(c);
        let grid_pos = grid.coordinates_for(index);
        let glyph = base_glyph
            .scaled(scale)
            .positioned(point(grid_pos.x, grid_pos.y));
        result.push(glyph);
        index += 1;
    }
    return result;
}

/// Rescale from 0..1 to -1..1
fn rescale(f: f32) -> f32 {
    -1 as f32 + (f / 1.0 as f32) * 2 as f32
}

#[derive(Debug, Copy, Clone)]
pub struct LineGrid {
    screen_dimensions: Vector<f32>,
    grid_dimensions: Vector<f32>,
    grid_padding: Vector<f32>,
    totals: Vector<u32>,
}

impl LineGrid {
    pub fn new(
        screen_dimensions: &Vector<f32>,
        grid_dimensions: &Vector<f32>,
        grid_padding: &Vector<f32>,
    ) -> LineGrid {
        let total_y = f32::floor((screen_dimensions.y - grid_padding.y) / grid_dimensions.y);
        let total_x = f32::floor((screen_dimensions.x - grid_padding.x) / grid_dimensions.x);

        LineGrid {
            screen_dimensions: screen_dimensions.clone(),
            grid_dimensions: grid_dimensions.clone(),
            grid_padding: grid_padding.clone(),
            totals: Vector {
                x: total_x as u32,
                y: total_y as u32,
            },
        }
    }

    /// Retrieve coordinates for a specific index
    pub fn coordinates_for(&self, index: u32) -> Vector<f32> {
        let x = index % self.totals.x;
        let y = index / self.totals.x;

        Vector {
            x: x as f32 * self.grid_dimensions.x + self.grid_padding.x,
            y: y as f32 * self.grid_dimensions.y + self.grid_padding.y,
        }
    }
}

/// Create a grid, lines for easy display
pub(crate) fn create_grid(
    grid_size: &Vector<f32>,
    grid_padding: &Vector<f32>,
    display: &glium::Display,
) -> (
    LineGrid,
    glium::VertexBuffer<Vertex>,
    glium::IndexBuffer<u16>,
) {
    let mut vertices: Vec<Vertex> = Vec::new();

    // Get the screen dimensions
    let (screen_width, screen_height) = {
        let (w, h) = display.get_framebuffer_dimensions();
        (w as f32, h as f32)
    };

    // Create the rogue grid for grid calculations
    let rogue_grid = LineGrid::new(
        &Vector {
            x: screen_width,
            y: screen_height,
        },
        grid_size,
        grid_padding,
    );

    // Create vertex at ends of screen to create grid cells
    for y in 0..((screen_height - grid_padding.y) / grid_size.y) as u32 {
        vertices.push(Vertex {
            position: [
                -1.0,
                -rescale((y as f32 * grid_size.y + grid_padding.y) / screen_height),
            ],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [
                1.0,
                -rescale((y as f32 * grid_size.y + grid_padding.y) / screen_height),
            ],
            color: [1.0, 1.0, 1.0],
        });
    }

    // Create vertex at ends of screen to create the grid cells
    for x in 0..((screen_width - grid_padding.x) / grid_size.x) as u32 {
        vertices.push(Vertex {
            position: [
                rescale((x as f32 * grid_size.x + grid_padding.x) / screen_width),
                -1.0,
            ],
            color: [1.0, 1.0, 1.0],
        });
        vertices.push(Vertex {
            position: [
                rescale((x as f32 * grid_size.x + grid_padding.x) / screen_width),
                1.0,
            ],
            color: [1.0, 1.0, 1.0],
        });
    }

    let vb = glium::VertexBuffer::new(display, &vertices).unwrap();
    let indices: Vec<u16> = (0..screen_height as u16).collect();
    let ib =
        glium::IndexBuffer::new(display, glium::index::PrimitiveType::LinesList, &indices).unwrap();

    return (rogue_grid, vb, ib);
}
