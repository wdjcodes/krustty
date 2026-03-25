use winit::event_loop::EventLoopProxy;

use crate::{grid::Grid, ui::Event};

pub struct Terminal {
    pub grid: Grid,

    event_loop: EventLoopProxy<Event>,
}

impl Terminal {
    pub fn new(event_loop: EventLoopProxy<Event>) -> Self {
        let grid = Grid::new(120, 23, 1000);
        Self { grid, event_loop }
    }

    pub fn print(&mut self, c: char) {
        self.grid.write_at_cursor(c);
        let _ = self.event_loop.send_event(Event::GridUpdate);
    }
}
