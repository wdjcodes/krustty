use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct Cursor {
    col: usize,
    row: usize,
    max_col: usize,
    max_row: usize,
    pub(crate) will_wrap: bool,
}

impl Cursor {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            col: 1,
            row: 1,
            max_col: cols,
            max_row: rows,
            will_wrap: false,
        }
    }

    pub fn as_point(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    pub fn set_from_point(&mut self, point: (usize, usize)) {
        self.row = point.0.clamp(1, self.max_row);
        self.col = point.1.clamp(1, self.max_col);
    }

    #[inline]
    pub fn home(&mut self) {
        self.row = 1;
        self.col = 1;
    }

    #[inline]
    pub fn home_col(&mut self) {
        self.col = 1;
    }

    pub fn up(&mut self, count: usize) {
        self.row = self.row.saturating_sub(count).max(1);
        self.log_cursor(format!("up {}", count));
    }

    pub fn down(&mut self, count: usize) {
        self.row = self.row.saturating_add(count).min(self.max_row);
        self.log_cursor(format!("down {}", count));
    }

    pub fn left(&mut self, count: usize) {
        self.col = self.col.saturating_sub(count).max(1);
        self.log_cursor(format!("left {}", count));
    }

    pub fn right(&mut self, count: usize) {
        self.col = self.col.saturating_add(count).min(self.max_col);
        self.log_cursor(format!("right {}", count));
    }

    fn log_cursor(&self, action: String) {
        log::trace!(
            "Cursor({}): {}, {} ({}, {})",
            action,
            self.row,
            self.col,
            self.max_row,
            self.max_col,
        );
    }

    #[inline]
    pub fn col(&self) -> usize {
        self.col
    }

    #[inline]
    pub fn row(&self) -> usize {
        self.row
    }

    #[inline]
    pub fn max_row(&self) -> usize {
        self.max_row
    }

    #[inline]
    pub fn is_row_max(&self) -> bool {
        self.row == self.max_row
    }

    #[inline]
    pub fn is_col_max(&self) -> bool {
        self.col == self.max_col
    }

    pub fn set_col(&mut self, col: usize) {
        self.col = col.clamp(1, self.max_col);
    }

    pub fn resize(&mut self, max_rows: usize, max_cols: usize) {
        self.max_row = std::cmp::max(1, max_rows);
        self.max_col = std::cmp::max(1, max_cols);
        self.row = self.row.min(self.max_row);
        self.col = self.col.min(self.max_col);
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.row, self.col)
    }
}
