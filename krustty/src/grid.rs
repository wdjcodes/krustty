use bitflags::bitflags;
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
            fg: DEFAULT_COLORS.white,
            bg: DEFAULT_COLORS.bright_black,
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

        for (i, row) in self.rows.iter_mut().enumerate() {
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
                let wrap = lines.pop_back().unwrap();
                if cursor_line.row == lines.len() && self.cursor.row != i {
                    cursor_line.col += r.len();
                }
                r.extend_from_slice(&wrap);
            }
            lines.push_back(r);
        }

        let mut cursor_placed = false;
        let mut new_rows = VecDeque::with_capacity(self.max_rows);
        for (i, line) in lines.into_iter().enumerate().rev() {
            let mut start = 0;
            let mut to_copy = line.len();
            loop {
                self.cursor.row += 1;
                let mut cells = Vec::with_capacity(width);
                let slice_len = std::cmp::min(to_copy, width);
                if i == cursor_line.row {
                    println!("Copy: {} Len: {}", to_copy, slice_len);
                }
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
                    new_rows.push_front(Row {
                        cells,
                        is_wrapped: false,
                    });
                    break;
                } else {
                    new_rows.push_front(Row {
                        cells,
                        is_wrapped: true,
                    });
                }
            }
        }
        self.rows = new_rows;
        self.width = width;
        self.height = height;
        println!(
            "Rows: {} Cursor.row: {} Cursor.col: {}",
            self.rows(),
            self.cursor.row,
            self.cursor.col,
        );
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
        if self.cursor.will_wrap {
            self.advance_cursor(1);
        }
        if let Some(cell) = self.rows[self.cursor.row].cells.get_mut(self.cursor.col) {
            cell.c = c;
            cell.fg = fg;
            cell.bg = bg;
        } else {
            self.rows[self.cursor.row].cells.push(Cell {
                c,
                fg,
                bg,
                flags: CellFlags::NONE,
            });
        }
        self.advance_cursor(1);
    }

    pub fn advance_cursor(&mut self, cols: usize) {
        self.cursor.will_wrap = false;
        if self.cursor.col + cols > self.width {
            if self.cursor.row == 0 {
                self.rows[0].is_wrapped = true;
                self.rows.push_front(Row::new(self.width));
            } else {
                self.cursor.row -= 1;
            }
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
