use glium::implement_vertex;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

implement_vertex!(Vertex, position, color);

#[derive(Copy, Clone)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub colour: [f32; 4],
}

implement_vertex!(TextVertex, position, tex_coords, colour);
