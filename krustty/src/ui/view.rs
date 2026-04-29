#[derive(Debug)]
pub struct ViewPort {
    /// Number of Rows in the view port
    pub height: usize,
    /// Caches the number of rows in the grid.
    /// This serves as an upper bound as to how far the user can scroll, and prevents
    /// having to lock the terminal for scroll events. It should be updated and the
    /// `ViewPort.start` should be bounded to this value.
    pub max_rows: usize,
    /// Offset from the start of grid buffer to the first row of the viewport.
    /// Scroll events can adjust by a fraction of a line, so this can be a
    /// fractional offset.
    pub start: f64,
    /// Used to detect when at least on row of scroll has been queued to request redraw
    pub scroll_queued: f64,
}
