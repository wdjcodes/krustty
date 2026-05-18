use bitflags::bitflags;
use log::debug;
use std::{collections::VecDeque, fmt::Display};

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
            fg: DEFAULT_COLORS.fg.into_format(),
            bg: DEFAULT_COLORS.bg.into_format(),
            flags: CellFlags::NONE,
        }
    }
}

impl Cell {
    fn is_empty(&self) -> bool {
        self.c == ' ' && self.flags == CellFlags::NONE
    }
}

/// Represents a single horizontal line of text.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// Indicates if this row wraps onto the next line (useful for window resizing).
    is_wrapped: bool,
}

impl Row {
    pub fn new(columns: usize) -> Self {
        Row {
            cells: Vec::with_capacity(columns),
            is_wrapped: false,
        }
    }

    pub fn get_cell(&self, idx: usize) -> &Cell {
        &self.cells[idx]
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

#[derive(Default, Debug)]
pub struct Cursor {
    pub col: usize,
    pub row: usize,
    /// When set writing another character will cause a soft line wrap
    pub will_wrap: bool,
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
    /// The offset into the grid where the cursor currently is
    pub cursor: Cursor,
    /// Template to use when creating a new cell
    pub template_cell: Cell,
}

impl Grid {
    pub fn new(width: usize, height: usize, max_rows: usize) -> Self {
        let mut rows = VecDeque::<Row>::with_capacity(max_rows);
        rows.push_back(Row::new(width));

        Grid {
            rows,
            max_rows,
            width,
            height,
            cursor: Default::default(),
            template_cell: Cell {
                c: ' ',
                fg: DEFAULT_COLORS.fg.into_format(),
                bg: DEFAULT_COLORS.bg.into_format(),
                flags: CellFlags::NONE,
            },
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        // Don't need to reflow text if row width hasn't changed
        if width == self.width {
            self.height = height;
            return;
        }
        let mut lines: VecDeque<Vec<Cell>> = VecDeque::with_capacity(self.rows());
        let mut cursor_line = Cursor {
            col: 0,
            row: 0,
            will_wrap: false,
        };

        for (i, row) in self.rows.iter_mut().enumerate().rev() {
            let mut r = std::mem::take(&mut row.cells);
            if self.cursor.row == i {
                cursor_line.row = lines.len();
                cursor_line.col = self.cursor.col;
            }
            if row.is_wrapped {
                // Truncate any trailing empty cells
                let len = r
                    .iter()
                    .rposition(|c| !c.is_empty())
                    .map_or(row.cells.len(), |idx| idx + 1);
                r.truncate(len);
                let wrap = lines.pop_front().unwrap();
                if cursor_line.row == lines.len() && self.cursor.row != i {
                    cursor_line.col += r.len();
                }
                r.extend_from_slice(&wrap);
            }
            lines.push_front(r);
        }

        let mut cursor_placed = false;
        let mut new_rows = VecDeque::with_capacity(self.max_rows);
        for (i, line) in lines.into_iter().enumerate() {
            let mut start = 0;
            let mut to_copy = line.len();
            loop {
                self.cursor.row += 1;
                let mut cells = Vec::with_capacity(width);
                let slice_len = std::cmp::min(to_copy, width);
                if !cursor_placed && cursor_line.row == i {
                    cursor_placed = true;
                    if slice_len >= cursor_line.col {
                        self.cursor.row = 0;
                        self.cursor.col = cursor_line.col;
                    } else {
                        cursor_line.col -= slice_len;
                    }
                }
                cells.extend_from_slice(&line[start..start + slice_len]);
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
        self.width = width;
        self.height = height;
        debug!(
            "Rows: {} Cursor.row: {} Cursor.col: {}",
            self.rows(),
            self.cursor.row,
            self.cursor.col,
        );
    }

    pub fn write_at_cursor(&mut self, c: char) {
        if self.cursor.will_wrap {
            self.advance_cursor(1);
        }
        let mut cell = self.template_cell;
        cell.c = c;
        if self.rows[self.cursor.row].cells.len() > self.cursor.col {
            self.rows[self.cursor.row].cells[self.cursor.col] = cell;
        } else {
            self.rows[self.cursor.row].cells.push(cell);
        }
        self.advance_cursor(1);
    }

    pub fn advance_cursor(&mut self, cols: usize) {
        self.cursor.will_wrap = false;
        if self.cursor.col + cols > self.width {
            if self.cursor.row == self.rows().saturating_sub(1) {
                self.rows[0].is_wrapped = true;
                self.rows.push_back(Row::new(self.width));
            }
            self.cursor.row += 1;
            self.cursor.col = 0;
        } else if self.cursor.col + cols == self.width {
            self.cursor.col = self.width;
            self.cursor.will_wrap = true;
        } else {
            let row = &mut self.rows[self.cursor.row];
            self.cursor.col = std::cmp::min(row.cells.len(), self.cursor.col + cols);
        }
    }

    pub fn line_feed(&mut self) {
        if self.cursor.row == self.rows().saturating_sub(1) {
            self.rows.push_back(Row::new(self.width));
        }
        self.cursor.row += 1;
        self.cursor.will_wrap = false;
    }

    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor.row = self
            .rows()
            .saturating_sub(self.height.saturating_sub(row - 1));
        self.cursor.col = col.saturating_sub(1);
        self.log_cursor(format!("set {},{}", row, col));
    }

    pub fn cursor_up(&mut self, count: usize) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_sub(count)
            .clamp(self.rows().saturating_sub(self.height - 1), self.rows() - 1);
        self.log_cursor(format!("up {}", count));
    }

    pub fn cursor_down(&mut self, count: usize) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(count)
            .clamp(0, self.rows() - 1);
        self.log_cursor(format!("down {}", count));
    }

    pub fn cursor_left(&mut self, count: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(count);
        self.log_cursor(format!("left {}", count));
    }

    pub fn cursor_right(&mut self, count: usize) {
        self.cursor.col = self
            .cursor
            .col
            .saturating_add(count)
            .clamp(0, self.width - 1);
        self.log_cursor(format!("right {}", count));
    }

    fn log_cursor(&self, action: String) {
        log::trace!(
            "Cursor({}): {}, {} ({}, {}, {})",
            action,
            self.cursor.row,
            self.cursor.col,
            self.height,
            self.width,
            self.rows(),
        );
    }

    pub fn get_row(&self, idx: usize) -> &Row {
        &self.rows[idx]
    }

    pub fn clear_screen(&mut self) {
        let row = self.cursor.row + 1;
        for i in row..(row + self.height) {
            if i < self.rows() {
                self.clear_line(i);
            } else {
                self.line_feed();
            }
        }
        self.cursor.row = row + self.height - 1;
        self.cursor.col = 0;
    }

    pub fn clear_screen_to_end(&mut self) {
        let row = self.cursor.row;
        self.clear_line_to_end();
        for i in row..self.rows() {
            self.clear_line(i);
        }
    }

    pub fn clear_current_line(&mut self) {
        self.clear_line(self.cursor.row);
    }

    pub fn clear_line(&mut self, row: usize) {
        self.rows[row].cells.fill(Default::default());
    }

    pub fn clear_line_to_end(&mut self) {
        let col = self.cursor.col;
        let row = &mut self.rows[self.cursor.row];
        row.cells.truncate(col);
    }

    pub fn clear_to_start(&mut self) {
        let col = self.cursor.col;
        let row = self.cursor.row;
        for i in 0..std::cmp::min(col + 1, self.rows[row].cells.len()) {
            self.rows[row].cells[i].c = ' ';
        }
    }

    /// Returns the number of rows currently in the grid
    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    pub fn set_fg(&mut self, fg: Rgb) {
        self.template_cell.fg = fg;
    }

    pub fn set_bg(&mut self, bg: Rgb) {
        self.template_cell.bg = bg;
    }

    pub fn set_inverse(&mut self, inverse: bool) {
        if inverse {
            self.template_cell.flags |= CellFlags::INVERSE;
        } else {
            self.template_cell.flags &= !CellFlags::INVERSE;
        }
    }
}
