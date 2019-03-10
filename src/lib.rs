#[macro_use]
extern crate glium;
extern crate rusttype;

use glium::{glutin, Surface};
use rusttype::gpu_cache::Cache;
use rusttype::{point, Font, Scale};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;

mod fonts;
mod program;
mod util;
mod vertex;

/// Main structure for accessing the roguelib library
pub struct Roguelib<'a> {
    fonts: HashMap<String, RogueFont<'a>>,
    grid_program: glium::Program,
    text_program: glium::Program,
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

pub fn create_window<S: Into<String>>(title: S) -> glutin::WindowBuilder {
    glutin::WindowBuilder::new()
        .with_dimensions((1920, 1080).into())
        .with_title(title)
}

/// Retrieve physical dimension for the display
pub fn get_physical_dimensions(display: &glium::Display) -> Result<(u32, u32), Box<Error>> {
    let dimensions: (u32, u32) = display
        .gl_window()
        .get_inner_size()
        .ok_or("get_inner_size")?
        .to_physical(get_dpi(display))
        .into();
    Ok(dimensions)
}
/// Get dpi factor
pub fn get_dpi(display: &glium::Display) -> f64 {
    display.gl_window().get_hidpi_factor()
}

impl<'a> Roguelib<'a> {
    /// Initialize roguelib library stuff
    pub fn new(display: &glium::Display) -> Roguelib<'a> {

        let context = glutin::ContextBuilder::new().with_vsync(true);
        let event_loop = glutin::EventsLoop::new();
        let grid_program = crate::program::create_grid_program(&display);
        let text_program = crate::program::create_text_program(&display);

        Roguelib {
            fonts: HashMap::new(),
            grid_program,
            text_program,
        }
    }

    /// Use a font for drawing purposes
    pub fn add_font<S: Into<String>>(&mut self, display: &glium::Display, name: S, font_bytes: &'static [u8], scale: f32) {
        let dpi = get_dpi(display);
        let (width, height) = get_physical_dimensions(display)
            .expect("Could not read window dimensions");
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
            display,
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

    pub fn get_font(&mut self, name: String) -> Option<&mut RogueFont<'a>>{
        self.fonts.get_mut(&name)
    }

    /// Draw the specific string in a grid
    pub fn draw<S: Into<String>>(&self, display: &glium::Display, font: &'a mut RogueFont<'a>, render_string: S) {
        let (width, _): (u32, u32) = get_physical_dimensions(display)
            .expect("Could not retrieve window dimensions");

        let glyphs = util::layout_grid(&font.font, font.scale, width, &render_string.into());

        // Queue the glyphs in the program
        for glyph in &glyphs {
            font.cache.queue_glyph(0, glyph.clone());
        }

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
        let mut text_vertex_buffer =
            crate::util::create_text_vb(display, &glyphs, &font.cache);
        let (mut vb_grid, mut ib_grid) =
            crate::util::create_grid(font.max_font_width, font.max_font_height, display);

        let mut target = display.draw();
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
