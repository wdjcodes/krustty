use winit::event_loop::EventLoopProxy;

use crate::{
    color::{DEFAULT_COLORS, Rgb},
    grid::Grid,
    ui::Event,
};

pub struct Terminal {
    pub grid: Grid,

    pub fg: Rgb,
    pub bg: Rgb,
    event_loop: EventLoopProxy<Event>,
}

impl Terminal {
    pub fn new(event_loop: EventLoopProxy<Event>) -> Self {
        let grid = Grid::new(120, 23, 1000);
        Self {
            grid,
            event_loop,
            fg: DEFAULT_COLORS.white,
            bg: DEFAULT_COLORS.black,
        }
    }

    pub fn print(&mut self, c: char) {
        self.grid.write_at_cursor(c, self.fg, self.bg);
        let _ = self.event_loop.send_event(Event::GridUpdate);
    }

    pub fn clear_to_end(&mut self) {
        let col = self.grid.cursor.col;
        let row = self.grid.cursor.row;
        for i in col..self.grid.width {
            self.grid.rows[row].cells[i].c = ' ';
        }
    }

    pub fn clear_to_start(&mut self) {
        let col = self.grid.cursor.col;
        let row = self.grid.cursor.row;
        for i in 0..=col {
            self.grid.rows[row].cells[i].c = ' ';
        }
    }

    pub fn clear_line(&mut self) {
        let row = self.grid.cursor.row;
        for i in 0..self.grid.width {
            self.grid.rows[row].cells[i].c = ' ';
        }
    }
}
