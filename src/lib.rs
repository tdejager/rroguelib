#[macro_use]
extern crate glium;

use glium::{glutin, Surface};
use rusttype::gpu_cache::Cache;
use rusttype::{point, Font, Scale, Vector};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;

mod program;
mod util;
mod vertex;

/// Main structure for accessing the roguelib library
pub struct Roguelib<'a> {
    fonts: HashMap<String, RogueFont<'a>>,
    grid_program: glium::Program,
    text_program: glium::Program,
    pub display: glium::Display,
    pub event_loop: glutin::EventsLoop,
}

/// A font to use with associated glium structures
pub struct RogueFont<'a> {
    font: Font<'a>,
    cache: Cache<'a>,
    max_font_height: f32,
    max_font_width: f32,
    scale: Scale,
    texture: glium::texture::Texture2d,
}

/// Create a window to use with the roguelib library
pub fn create_window<S: Into<String>>(title: S) -> glutin::WindowBuilder {
    glutin::WindowBuilder::new()
        .with_dimensions((1920, 1080).into())
        .with_title(title)
}

/// Retrieve physical dimension for the display
pub fn get_physical_dimensions(display: &glium::Display) -> Result<(u32, u32), Box<dyn Error>> {
    let dimensions: (u32, u32) = display
        .gl_window()
        .window()
        .get_inner_size()
        .ok_or("get_inner_size")?
        .to_physical(get_dpi(display))
        .into();
    Ok(dimensions)
}

/// Get dpi factor
pub fn get_dpi(display: &glium::Display) -> f64 {
    display.gl_window().window().get_hidpi_factor()
}

impl<'a> Roguelib<'a> {
    /// Initialize roguelib library stuff
    pub fn new(s: &str) -> Roguelib<'a> {
        let window = create_window(s);
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let event_loop = glutin::EventsLoop::new();
        let display = glium::Display::new(window, context, &event_loop).unwrap();

        // Create the shaders for the grid
        let grid_program = crate::program::create_grid_program(&display);
        // Create the shaders for the text rendering
        let text_program = crate::program::create_text_program(&display);

        Roguelib {
            fonts: HashMap::new(),
            grid_program,
            text_program,
            display,
            event_loop,
        }
    }

    /// Use a font for drawing purposes
    pub fn add_font<S: Into<String>>(&mut self, name: S, font_bytes: &'static [u8], scale: f32) {
        let dpi = get_dpi(&self.display);
        let (width, height) =
            get_physical_dimensions(&self.display).expect("Could not read window dimensions");
        let font = Font::from_bytes(font_bytes).expect("Could not create font");
        let scale = Scale::uniform(scale * dpi as f32);

        // Calculate maximum height
        let v_metrics = font.v_metrics(scale);
        let max_font_height = v_metrics.ascent - v_metrics.descent; // + v_metrics.line_gap;

        // Loop over box char to find the maximum width
        use unicode_normalization::UnicodeNormalization;
        let mut max_font_width = 0.0;
        let box_char: String = "█".into();
        for c in box_char.nfc() {
            let glyph = font.glyph(c).scaled(scale);
            let _bounding_box = glyph
                .clone()
                .positioned(point(0.0, 0.0))
                .pixel_bounding_box()
                .unwrap();
            max_font_width = glyph.h_metrics().advance_width;
        }

        // Create the font texture
        let cache_tex = glium::texture::Texture2d::with_format(
            &self.display,
            glium::texture::RawImage2d {
                data: Cow::Owned(vec![128u8; width as usize * height as usize]),
                width,
                height,
                format: glium::texture::ClientFormat::U8,
            },
            glium::texture::UncompressedFloatFormat::U8,
            glium::texture::MipmapsOption::NoMipmap,
        )
        .expect("Could not create font texture");

        // Create the font cache
        let cache = Cache::builder().dimensions(width, height).build();

        // Save the font
        self.fonts.insert(
            name.into(),
            RogueFont {
                font,
                max_font_height,
                max_font_width,
                scale,
                texture: cache_tex,
                cache,
            },
        );
    }

    /// Draw the specific string in a grid
    pub fn draw<S: Into<String>>(&mut self, font: &str, render_string: S) {
        let (width, _): (u32, u32) =
            get_physical_dimensions(&self.display).expect("Could not retrieve window dimensions");

        let font = self
            .fonts
            .get_mut(font.into())
            .expect("Font does not exist");
        let font_vec = Vector {
            x: font.max_font_width,
            y: font.max_font_height,
        };

        let (grid, vb_grid, ib_grid) = crate::util::create_grid(
            &font_vec,
            &Vector {
                x: 0.0,
                y: font.font.v_metrics(font.scale).ascent,
            },
            &self.display,
        );

        let glyphs =
            crate::util::layout_grid(&font.font, font.scale, width, &grid, &render_string.into());

        //         Queue the glyphs in the program
        for glyph in &glyphs {
            font.cache.queue_glyph(0, glyph.clone());
        }
        //
        // Cache the rects
        let texture = &mut font.texture;
        font.cache
            .cache_queued(|rect, data| {
                texture.main_level().write(
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
            })
            .expect("Could not queue texture data");

        // Set the text uniforms
        let text_uniforms = uniform! {
            tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        };

        // building the uniforms for the grid program
        let uniforms_grid = uniform! {
                matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0f32]
                ]
        };

        // Create the vertex buffer and the grid
        let text_vertex_buffer = crate::util::create_text_vb(&self.display, &glyphs, &font.cache);

        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        // Draw the grid lines
        target
            .draw(
                &vb_grid,
                &ib_grid,
                &self.grid_program,
                &uniforms_grid,
                &Default::default(),
            )
            .expect("Could not draw the grid lines");

        // Draw the text
        target
            .draw(
                &text_vertex_buffer,
                glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &self.text_program,
                &text_uniforms,
                &glium::DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    //                polygon_mode: glium::PolygonMode::Line,
                    ..Default::default()
                },
            )
            .expect("Could not draw text");

        target.finish().expect("Could not execute finish command");
    }
}
