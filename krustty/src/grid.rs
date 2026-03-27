use bitflags::bitflags;
use std::collections::VecDeque;

use crate::color::{DEFAULT_COLORS, Rgb};

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
pub struct Cell {
    pub c: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            c: ' ',
            fg: DEFAULT_COLORS.white,
            bg: DEFAULT_COLORS.bright_black,
            flags: CellFlags::NONE,
        }
    }
}

/// Represents a single horizontal line of text.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// Indicates if this row wraps onto the next line (useful for window resizing).
    #[expect(unused)]
    is_wrapped: bool,
}

impl Row {
    pub fn new(columns: usize) -> Self {
        Row {
            cells: vec![Cell::default(); columns],
            is_wrapped: false,
        }
    }

    pub fn get_cell(&self, idx: usize) -> &Cell {
        &self.cells[idx]
    }
}

#[derive(Default, Debug)]
pub struct Cursor {
    pub col: usize,
    pub row: usize,
}

/// Main terminal Grid
#[derive(Debug)]
pub struct Grid {
    /// All rows including active viewport, scrollback, and scrollahead
    pub rows: VecDeque<Row>,
    /// Maximum number of rows to store before they get dropped
    #[expect(unused)]
    max_rows: usize,
    /// Current number of columns
    pub width: usize,
    /// Current number of rows in the active viewport
    pub height: usize,
    /// Current offset into history from which viewport starts
    pub view_offset: usize,
    /// The offset into the grid where the cursor currently is
    pub cursor: Cursor,
}

impl Grid {
    pub fn new(width: usize, height: usize, max_rows: usize) -> Self {
        let mut rows = VecDeque::<Row>::with_capacity(max_rows);
        for _ in 0..height {
            rows.push_front(Row::new(width));
        }

        Grid {
            rows,
            max_rows,
            width,
            height,
            view_offset: 0,
            cursor: Default::default(),
        }
    }

    #[expect(unused)]
    pub fn scroll_up(&mut self) {
        if self.rows.len() - self.height > self.view_offset {
            self.view_offset += 1;
        }
    }

    #[expect(unused)]
    pub fn scroll_down(&mut self) {
        self.view_offset = self.view_offset.saturating_sub(1);
    }

    pub fn write_at_cursor(&mut self, c: char, fg: Rgb, bg: Rgb) {
        let cell = &mut self.rows[self.cursor.row].cells[self.cursor.col];
        cell.c = c;
        cell.fg = fg;
        cell.bg = bg;

        self.advance_cursor(1);
    }

    pub fn advance_cursor(&mut self, cols: usize) {
        if self.cursor.col + cols < self.width {
            self.cursor.col += cols;
        } else {
            self.rows.push_front(Row::new(self.width));
            self.cursor.col = 0;
        }
    }

    pub fn line_feed(&mut self) {
        if self.cursor.row == 0 {
            self.rows.push_front(Row::new(self.width));
        } else {
            self.cursor.row -= 1;
        }
    }

    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    pub fn left(&mut self) {
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    pub fn get_row(&self, idx: usize) -> &Row {
        &self.rows[idx]
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}
