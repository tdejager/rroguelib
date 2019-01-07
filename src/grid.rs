#[derive(Debug)]
struct Grid {
    /// Width of the grid
    width: f32,
    /// Height of the grid
    height: f32,
    /// Height of a cell in the x direction
    grid_x: f32,
    /// Height of a cell in the y direction
    grid_y: f32,
    /// Number of cells in the y direction
    cells_y: u32,
    /// Number of cells in the x direction
    cells_x: u32,
}

#[derive(Debug)]
struct GridCell {
    cell_height: f32,
    cell_width: f32,
    x: f32,
    y: f32,
}

impl Grid {
    /// Create a new grid function
    fn new(width: f32, height: f32, grid_x: f32, grid_y: f32) -> Grid {
        return Grid {
            width,
            height,
            grid_x,
            grid_y,
            cells_y: (height / grid_y) as u32,
            cells_x: (width / grid_x) as u32,
        };
    }

    /// Return the grid cell by a 1D index
    fn by_index(&self, index: u32) -> GridCell {
        let y_idx = index / self.cells_x;
        let x_idx = index % self.cells_x;

        // Returns the top left corner of the grid cell
        return GridCell {
            cell_width: self.grid_x,
            cell_height: self.grid_y,
            x: x_idx as f32 * self.grid_x,
            y: y_idx as f32 * self.grid_y,
        };
    }
}
