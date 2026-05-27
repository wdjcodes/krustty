use bitflags::bitflags;
use log::debug;
use std::{
    collections::VecDeque,
    fmt::Display,
    ops::{Index, IndexMut},
};

use crate::{
    color::{DEFAULT_COLORS, Rgb},
    term::cursor::Cursor,
};

bitflags! {
    /// Text styling attributes
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CellFlags: u16 {
        const NONE          = 0;
        const BOLD          = 1 << 0;
        const DIM           = 1 << 1;
        const ITALIC        = 1 << 2;
        const UNDERLINE     = 1 << 3;
        const BLINK         = 1 << 4;
        const INVERSE       = 1 << 5;
        const HIDDEN        = 1 << 6;
        const STRIKETHROUGH = 1 << 7;
    }
}

/// A single character cell on the terminal grid.
/// Memory footprint is kept as small as possible since we will have thousands of these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridCell {
    pub c: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: CellFlags,
}

impl Default for GridCell {
    fn default() -> Self {
        GridCell {
            c: ' ',
            fg: DEFAULT_COLORS.fg.into_format(),
            bg: DEFAULT_COLORS.bg.into_format(),
            flags: CellFlags::NONE,
        }
    }
}

impl GridCell {
    fn is_empty(&self) -> bool {
        self.c == ' ' && self.flags == CellFlags::NONE
    }
}

/// Represents a single horizontal line of text.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<GridCell>,
    /// Indicates if this row wraps onto the next line (useful for window resizing).
    pub is_wrapped: bool,
}

impl Row {
    pub fn new(columns: usize) -> Self {
        Row {
            // Should always be filled to make indexing easier
            cells: vec![Default::default(); columns],
            is_wrapped: false,
        }
    }

    pub fn get_cell(&self, idx: usize) -> &GridCell {
        &self.cells[idx]
    }

    pub fn clear(&mut self) {
        self.cells.fill(Default::default());
        self.is_wrapped = false;
    }
}

impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::with_capacity(self.cells.len());
        for cell in &self.cells {
            out.push(cell.c);
        }
        write!(f, "[{}]{}", if self.is_wrapped { 'w' } else { ' ' }, out)
    }
}

/// Main terminal Grid
#[derive(Debug)]
pub struct Grid {
    /// All rows including active viewport, scrollback, and scrollahead
    pub rows: VecDeque<Row>,
    /// Maximum number of rows to store before they get dropped
    max_rows: usize,
    /// Current number of columns
    pub width: usize,
    /// Current number of rows in the active viewport
    pub height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize, max_rows: usize) -> Self {
        let mut rows = VecDeque::<Row>::with_capacity(max_rows);
        for _ in 0..height {
            rows.push_back(Row::new(width));
        }

        Grid {
            rows,
            max_rows,
            width,
            height,
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize, cursor: &mut Cursor) {
        // Don't need to reflow text if row width hasn't changed
        if cols == self.width {
            self.height = rows;
            return;
        }
        let mut lines: VecDeque<Vec<GridCell>> = VecDeque::with_capacity(self.rows());
        let (cu_row, cu_col) = self.cursor_to_grid_idx(cursor);

        let mut new_row = 0;
        let mut new_col = 0;
        for (i, row) in self.rows.iter_mut().enumerate().rev() {
            let mut r = std::mem::take(&mut row.cells);
            if cu_row == i {
                new_row = lines.len();
                new_col = cu_col;
            }
            if row.is_wrapped {
                let wrap = lines.pop_front().unwrap();
                if new_row == lines.len() && cu_row != i {
                    new_col += r.len();
                }
                r.extend_from_slice(&wrap);
            } else {
                // Truncate any trailing empty cells
                let len = r
                    .iter()
                    .rposition(|c| !c.is_empty())
                    .map_or(row.cells.len(), |idx| idx + 1);

                r.truncate(len);
            }
            lines.push_front(r);
        }

        let mut new_rows = VecDeque::with_capacity(self.max_rows);
        for line in lines.into_iter() {
            let mut start = 0;
            let mut to_copy = line.len();
            loop {
                let mut cells = Vec::with_capacity(cols);
                let slice_len = std::cmp::min(to_copy, cols);
                cells.extend_from_slice(&line[start..start + slice_len]);
                cells.resize(cols, GridCell::default());
                start += slice_len;
                to_copy -= slice_len;
                if to_copy == 0 {
                    new_rows.push_back(Row {
                        cells,
                        is_wrapped: false,
                    });
                    break;
                } else {
                    new_rows.push_back(Row {
                        cells,
                        is_wrapped: true,
                    });
                }
            }
        }
        self.rows = new_rows;
        self.width = cols;
        self.height = rows;
        cursor.set_from_point((rows - new_row, new_col));
        debug!(
            "Rows: {} Cursor.row: {} Cursor.col: {}",
            self.rows(),
            cursor.row(),
            cursor.col(),
        );
    }

    pub fn get_row(&self, idx: usize) -> &Row {
        &self.rows[idx]
    }

    pub fn cursor_to_grid_idx(&self, cursor: &Cursor) -> (usize, usize) {
        let row = self.rows() - cursor.max_row() + cursor.row() - 1;
        let col = cursor.col() - 1;
        (row, col)
    }

    pub fn write_at_cursor(&mut self, cursor: &mut Cursor, cell: GridCell) {
        let (row, col) = self.cursor_to_grid_idx(cursor);
        log::debug!("Cursor: {:?} ({}, {}) '{}'", cursor, row, col, cell.c);
        if cursor.will_wrap {
            self[row - 1].is_wrapped = true;
            cursor.will_wrap = false;
        }
        self[row][col] = cell;
    }

    pub fn push_row(&mut self) {
        if self.rows() >= self.max_rows {
            self.rows.pop_front();
        }
        self.rows.push_back(Row::new(self.width));
    }

    pub fn clear_line(&mut self, cursor: &Cursor) {
        let (row, _) = self.cursor_to_grid_idx(cursor);
        self[row].clear();
    }

    pub fn clear_line_to_end(&mut self, cursor: &Cursor) {
        let (row, col) = self.cursor_to_grid_idx(cursor);
        let width = self.width;
        self[row].cells.truncate(col);
        self[row].cells.resize(width, Default::default());
    }

    pub fn clear_line_to_start(&mut self, cursor: &Cursor) {
        let (row, col) = self.cursor_to_grid_idx(cursor);
        for i in 0..col {
            self[row][i] = Default::default();
        }
    }

    /// Returns the number of rows currently in the grid
    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}

impl Index<usize> for Grid {
    type Output = Row;

    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[index]
    }
}

impl IndexMut<usize> for Grid {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.rows[index]
    }
}

impl Index<usize> for Row {
    type Output = GridCell;

    fn index(&self, index: usize) -> &Self::Output {
        &self.cells[index]
    }
}

impl IndexMut<usize> for Row {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cells[index]
    }
}

impl Index<(usize, usize)> for Grid {
    type Output = GridCell;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        &self[row][col]
    }
}

impl IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (row, col) = index;
        &mut self[row][col]
    }
}
