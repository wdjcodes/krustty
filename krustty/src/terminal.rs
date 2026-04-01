use vte::Perform;
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy};

use crate::{
    color::{DEFAULT_COLORS, Rgb},
    grid::Grid,
    ui::Event,
};

pub struct Terminal {
    pub grid: Grid,

    pub fg: Rgb,
    pub bg: Rgb,
    pub event_loop: EventLoopProxy<Event>,
}

impl Terminal {
    pub fn new(event_loop: EventLoopProxy<Event>, width: usize, height: usize) -> Self {
        let grid = Grid::new(width, height, 1000);
        Self {
            grid,
            event_loop,
            fg: DEFAULT_COLORS.white,
            bg: DEFAULT_COLORS.black,
        }
    }

    pub fn print(&mut self, c: char) {
        self.grid.write_at_cursor(c, self.fg, self.bg);
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

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        self.print(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.grid.carriage_return();
                self.grid.line_feed();
            }
            b'\x0B' | b'\x0C' => {
                self.grid.line_feed();
            }
            b'\r' => {
                self.grid.carriage_return();
            }
            b'\x08' => {
                // Backspace (BS)
                self.grid.left();
            }
            b'\t' => {
                self.grid.advance_cursor(4);
            }
            //others Still need to be implemented
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'm' => {
                for param in params {
                    let code = param.first().unwrap_or(&255);
                    match code {
                        0 => {
                            self.fg = DEFAULT_COLORS.white;
                            self.bg = DEFAULT_COLORS.black;
                        }
                        // Foreground
                        30 => self.fg = DEFAULT_COLORS.black,
                        31 => self.fg = DEFAULT_COLORS.red,
                        32 => self.fg = DEFAULT_COLORS.green,
                        33 => self.fg = DEFAULT_COLORS.yellow,
                        34 => self.fg = DEFAULT_COLORS.blue,
                        35 => self.fg = DEFAULT_COLORS.purple,
                        36 => self.fg = DEFAULT_COLORS.cyan,
                        37 => self.fg = DEFAULT_COLORS.white,
                        39 => self.fg = DEFAULT_COLORS.white,
                        // Background
                        40 => self.bg = DEFAULT_COLORS.black,
                        41 => self.bg = DEFAULT_COLORS.red,
                        42 => self.bg = DEFAULT_COLORS.green,
                        43 => self.bg = DEFAULT_COLORS.yellow,
                        44 => self.bg = DEFAULT_COLORS.blue,
                        45 => self.bg = DEFAULT_COLORS.purple,
                        46 => self.bg = DEFAULT_COLORS.cyan,
                        47 => self.bg = DEFAULT_COLORS.white,
                        49 => self.bg = DEFAULT_COLORS.black,
                        // Bright Foreground
                        90 => self.fg = DEFAULT_COLORS.bright_black,
                        91 => self.fg = DEFAULT_COLORS.bright_red,
                        92 => self.fg = DEFAULT_COLORS.bright_green,
                        93 => self.fg = DEFAULT_COLORS.bright_yellow,
                        94 => self.fg = DEFAULT_COLORS.bright_blue,
                        95 => self.fg = DEFAULT_COLORS.bright_purple,
                        96 => self.fg = DEFAULT_COLORS.bright_cyan,
                        97 => self.fg = DEFAULT_COLORS.bright_white,
                        // Bright Background
                        100 => self.bg = DEFAULT_COLORS.bright_black,
                        101 => self.bg = DEFAULT_COLORS.bright_red,
                        102 => self.bg = DEFAULT_COLORS.bright_green,
                        103 => self.bg = DEFAULT_COLORS.bright_yellow,
                        104 => self.bg = DEFAULT_COLORS.bright_blue,
                        105 => self.bg = DEFAULT_COLORS.bright_purple,
                        106 => self.bg = DEFAULT_COLORS.bright_cyan,
                        107 => self.bg = DEFAULT_COLORS.bright_white,

                        code => {
                            println!(
                                "Unsupported SGR: Code: {} Intermediates: {:?} Params: {:?} Action: {}",
                                code, intermediates, params, action
                            );
                        }
                    }
                }
            }
            'D' => {
                let count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                self.grid.cursor.col = self.grid.cursor.col.saturating_sub(*count as usize);
            }
            // Erase in Line (EL)
            'K' => {
                // If no parameter is provided, it defaults to 0
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);

                match mode {
                    0 => {
                        self.clear_to_end();
                    }
                    1 => {
                        self.clear_to_start();
                    }
                    2 => {
                        self.clear_line();
                    }
                    _ => {} // Ignore unsupported modes
                }
            }

            // Cursor Horizontal Absolute (CHA)
            'G' | '`' => {
                let target_col = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);

                // VT coordinates are 1-indexed. Rust arrays are 0-indexed.
                let zero_indexed_col = (*target_col as usize).saturating_sub(1);
                self.grid.cursor.col = zero_indexed_col.min(self.grid.width.saturating_sub(1));
            }

            _ => {
                println!(
                    "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                    intermediates, params, action
                );
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn terminated(&self) -> bool {
        false
    }
}
