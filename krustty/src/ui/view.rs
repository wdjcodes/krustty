#[derive(Debug)]
pub struct ViewPort {
    /// Number of Rows in the view port
    pub height: usize,
    /// Offset from the end of the grid to the bottom of the viewport.
    pub offset: usize,
    /// Used to detect when at least one row of scroll has been queued to request redraw
    /// Scroll events can adjust by a fraction of a line, so this can be a
    /// fractional offset.
    pub scroll_queued: f64,
}

impl ViewPort {
    pub fn grid_to_viewport(&self, row: usize, max_rows: usize) -> Option<usize> {
        let (start, end) = self.get_rows(max_rows);
        if row >= start && row < end {
            Some(row - start)
        } else {
            None
        }
    }

    /// Queue scroll to be applied in a future render pass
    pub fn queue_scroll(&mut self, rows: f64) -> f64 {
        self.scroll_queued += rows;
        self.scroll_queued
    }

    /// Apply the queued scroll delta to the view_port
    /// Has no effect if less then 1 row has been queued for scroll
    pub fn apply_scroll(&mut self, max_rows: usize) {
        if self.scroll_queued < 1.0 && self.scroll_queued > -1.0 {
            return;
        }
        let scroll_rows = self.scroll_queued.trunc();
        log::debug!("Row: {} + Scroll: {}", self.offset, scroll_rows);
        self.scroll_queued -= scroll_rows;
        self.offset = self
            .offset
            .saturating_add_signed(scroll_rows as isize)
            .clamp(0, max_rows.saturating_sub(self.height));
    }

    /// Gets the bounds of the viewport
    ///
    /// Returns a tuple of `(start, end)` where the viewport covers the rows in
    /// the range `start..end`
    pub fn get_rows(&self, max_rows: usize) -> (usize, usize) {
        let start = max_rows.saturating_sub(self.offset + self.height);
        let end = std::cmp::min(max_rows, start + self.height);
        (start, end)
    }
}
