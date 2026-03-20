use bitflags::bitflags;
use std::collections::VecDeque;

/// Represents the different ways a color can be defined in the terminal
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Color {
    /// The default foreground/background color specified by the user's theme.
    #[default]
    Default,
    /// Standard 16 ANSI colors (e.g., Black, Red, Green, etc.).
    #[expect(unused)]
    Named(u8),
    /// 256-color palette (xterm).
    #[expect(unused)]
    Indexed(u8),
    /// 24-bit True Color (RGB).
    #[expect(unused)]
    Rgb { r: u8, g: u8, b: u8 },
}

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
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            c: ' ', // Empty space by default
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::NONE,
        }
    }
}

/// Represents a single horizontal line of text.
#[derive(Clone, Debug)]
pub struct Row {
    cells: Vec<Cell>,
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

/// Main terminal Grid
#[derive(Debug)]
pub struct Grid {
    /// All rows including active viewport, scrollback, and scrollahead
    rows: VecDeque<Row>,
    /// Maximum number of rows to store before they get dropped
    #[expect(unused)]
    max_rows: usize,
    /// Current number of columns
    width: usize,
    /// Current number of rows in the active viewport
    height: usize,
    /// Current offset into history from which viewport starts
    view_offset: usize,
    /// The offset into the grid where the cursor currently is
    cursor: (usize, usize),
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
            cursor: (0, 0),
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

    pub fn write_at_cursor(&mut self, c: char) {
        println!("C: {} Cursor: {:?}", c, self.cursor);
        self.rows[self.cursor.0].cells[self.cursor.1].c = c;
        self.advance_cursor();
    }

    fn advance_cursor(&mut self) {
        if self.cursor.1 < self.width - 1 {
            self.cursor.1 += 1;
        } else {
            self.rows.push_front(Row::new(self.width));
            self.cursor.1 = 0;
        }
    }

    pub fn get_row(&self, idx: usize) -> &Row {
        &self.rows[idx]
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}
