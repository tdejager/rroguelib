use rusttype::gpu_cache::CacheBuilder;
use rusttype::{point, vector, Font, PositionedGlyph, Rect, Scale};

/// Load font from font data path
fn load_font(font_data: &str) {
    //let font_data = include_bytes!(font_data);
    Font::from_bytes(font_data.as_bytes());
}
