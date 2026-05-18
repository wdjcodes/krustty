use log::info;
use vte::Perform;
use winit::event_loop::EventLoopProxy;

use crate::{color::DEFAULT_COLORS, grid::Grid, ui::Event};

pub struct Terminal {
    pub grid: Grid,

    pub event_loop: EventLoopProxy<Event>,
    response_buffer: Vec<u8>,
}

impl Terminal {
    pub fn new(event_loop: EventLoopProxy<Event>, width: usize, height: usize) -> Self {
        let grid = Grid::new(width, height, 1000);
        Self {
            grid,
            event_loop,
            response_buffer: vec![],
        }
    }

    pub fn take_response(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.response_buffer)
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        self.grid.write_at_cursor(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
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
                self.grid.cursor_left(1);
            }
            b'\t' => {
                self.grid.advance_cursor(4);
            }
            //others Still need to be implemented
            byte => info!("Unsupported control character: 0x{:2x}", byte),
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
        let grid = &mut self.grid;
        match action {
            'c' => {
                let code = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                match code {
                    0 => {
                        self.response_buffer.extend_from_slice(b"\x1b[?62;22c");
                        let _ = self.event_loop.send_event(Event::SendPtyResponse);
                    }
                    code => info!(
                        "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                        intermediates, code, action
                    ),
                }
            }
            'm' => {
                for param in params {
                    let code = param.first().unwrap_or(&255);
                    match code {
                        0 => {
                            grid.set_fg(DEFAULT_COLORS.fg.into_format());
                            grid.set_bg(DEFAULT_COLORS.bg.into_format());
                        }
                        7 => grid.set_inverse(true),
                        27 => grid.set_inverse(false),
                        // Foreground
                        30 => grid.set_fg(DEFAULT_COLORS.black.into_format()),
                        31 => grid.set_fg(DEFAULT_COLORS.red.into_format()),
                        32 => grid.set_fg(DEFAULT_COLORS.green.into_format()),
                        33 => grid.set_fg(DEFAULT_COLORS.yellow.into_format()),
                        34 => grid.set_fg(DEFAULT_COLORS.blue.into_format()),
                        35 => grid.set_fg(DEFAULT_COLORS.purple.into_format()),
                        36 => grid.set_fg(DEFAULT_COLORS.cyan.into_format()),
                        37 => grid.set_fg(DEFAULT_COLORS.white.into_format()),
                        39 => grid.set_fg(DEFAULT_COLORS.white.into_format()),
                        // Background
                        40 => grid.set_bg(DEFAULT_COLORS.black.into_format()),
                        41 => grid.set_bg(DEFAULT_COLORS.red.into_format()),
                        42 => grid.set_bg(DEFAULT_COLORS.green.into_format()),
                        43 => grid.set_bg(DEFAULT_COLORS.yellow.into_format()),
                        44 => grid.set_bg(DEFAULT_COLORS.blue.into_format()),
                        45 => grid.set_bg(DEFAULT_COLORS.purple.into_format()),
                        46 => grid.set_bg(DEFAULT_COLORS.cyan.into_format()),
                        47 => grid.set_bg(DEFAULT_COLORS.white.into_format()),
                        49 => grid.set_bg(DEFAULT_COLORS.black.into_format()),
                        // Bright Foreground
                        90 => grid.set_fg(DEFAULT_COLORS.bright_black.into_format()),
                        91 => grid.set_fg(DEFAULT_COLORS.bright_red.into_format()),
                        92 => grid.set_fg(DEFAULT_COLORS.bright_green.into_format()),
                        93 => grid.set_fg(DEFAULT_COLORS.bright_yellow.into_format()),
                        94 => grid.set_fg(DEFAULT_COLORS.bright_blue.into_format()),
                        95 => grid.set_fg(DEFAULT_COLORS.bright_purple.into_format()),
                        96 => grid.set_fg(DEFAULT_COLORS.bright_cyan.into_format()),
                        97 => grid.set_fg(DEFAULT_COLORS.bright_white.into_format()),
                        // Bright Background
                        100 => grid.set_bg(DEFAULT_COLORS.bright_black.into_format()),
                        101 => grid.set_bg(DEFAULT_COLORS.bright_red.into_format()),
                        102 => grid.set_bg(DEFAULT_COLORS.bright_green.into_format()),
                        103 => grid.set_bg(DEFAULT_COLORS.bright_yellow.into_format()),
                        104 => grid.set_bg(DEFAULT_COLORS.bright_blue.into_format()),
                        105 => grid.set_bg(DEFAULT_COLORS.bright_purple.into_format()),
                        106 => grid.set_bg(DEFAULT_COLORS.bright_cyan.into_format()),
                        107 => grid.set_bg(DEFAULT_COLORS.bright_white.into_format()),

                        code => {
                            info!(
                                "Unsupported SGR: Code: {} Intermediates: {:?} Params: {:?} Action: {}",
                                code, intermediates, params, action
                            );
                        }
                    }
                }
            }
            'A' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);
                count = if *count == 0 { &1 } else { count };
                grid.cursor_up(*count as usize);
            }
            'B' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                grid.cursor_down(*count as usize);
            }
            'C' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                grid.cursor_right(*count as usize)
            }
            'D' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                grid.cursor_left(*count as usize);
            }
            // Cursor Horizontal Absolute (CHA)
            'G' | '`' => {
                let target_col = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);

                // VT coordinates are 1-indexed. Rust arrays are 0-indexed.
                let zero_indexed_col = (*target_col as usize).saturating_sub(1);
                self.grid.cursor.col = zero_indexed_col.min(self.grid.width.saturating_sub(1));
            }
            // Position cursor [row;column]
            'H' | 'f' => {
                let (row, col) = match params.iter().collect::<Vec<&[u16]>>().as_slice() {
                    [row, col, ..] => {
                        if let Some(row) = row.first()
                            && let Some(col) = col.first()
                        {
                            (row, col)
                        } else {
                            log::debug!("Missing row or column");
                            return;
                        }
                    }
                    _ => (&1, &1),
                };
                grid.set_cursor(*row as usize, *col as usize);
            }
            'J' => {
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                match mode {
                    0 => grid.clear_screen_to_end(),
                    2 => grid.clear_screen(),
                    _ => {
                        info!(
                            "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                            intermediates, params, action
                        );
                    }
                }
            }
            // Erase in Line (EL)
            'K' => {
                // If no parameter is provided, it defaults to 0
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);

                match mode {
                    0 => {
                        grid.clear_line_to_end();
                    }
                    1 => {
                        grid.clear_to_start();
                    }
                    2 => {
                        grid.clear_current_line();
                    }
                    _ => {
                        info!(
                            "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                            intermediates, params, action
                        );
                    }
                }
            }
            _ => {
                info!(
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
