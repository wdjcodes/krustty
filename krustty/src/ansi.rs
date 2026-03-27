use std::sync::{Arc, Mutex};

use vte::Perform;

use crate::{color::DEFAULT_COLORS, terminal::Terminal};

pub struct AnsiParser {
    term: Arc<Mutex<Terminal>>,
}

impl AnsiParser {
    pub fn new(term: Arc<Mutex<Terminal>>) -> Self {
        Self { term: term.clone() }
    }
}

impl Perform for AnsiParser {
    fn print(&mut self, c: char) {
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal while printing ANSI");
        term.print(c);
    }

    fn execute(&mut self, byte: u8) {
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal while executing ANSI");
        match byte {
            b'\n' => {
                term.grid.carriage_return();
                term.grid.line_feed();
            }
            b'\x0B' | b'\x0C' => {
                term.grid.line_feed();
            }
            b'\r' => {
                term.grid.carriage_return();
            }
            b'\x08' => {
                // Backspace (BS)
                term.grid.left();
            }
            b'\t' => {
                term.grid.advance_cursor(4);
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
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal during CSI dispatch");
        match action {
            'm' => {
                for param in params {
                    let code = param.first().unwrap_or(&255);
                    match code {
                        0 => {
                            term.fg = DEFAULT_COLORS.white;
                            term.bg = DEFAULT_COLORS.black;
                        }
                        // Foreground
                        30 => term.fg = DEFAULT_COLORS.black,
                        31 => term.fg = DEFAULT_COLORS.red,
                        32 => term.fg = DEFAULT_COLORS.green,
                        33 => term.fg = DEFAULT_COLORS.yellow,
                        34 => term.fg = DEFAULT_COLORS.blue,
                        35 => term.fg = DEFAULT_COLORS.purple,
                        36 => term.fg = DEFAULT_COLORS.cyan,
                        37 => term.fg = DEFAULT_COLORS.white,
                        39 => term.fg = DEFAULT_COLORS.white,
                        // Background
                        40 => term.bg = DEFAULT_COLORS.black,
                        41 => term.bg = DEFAULT_COLORS.red,
                        42 => term.bg = DEFAULT_COLORS.green,
                        43 => term.bg = DEFAULT_COLORS.yellow,
                        44 => term.bg = DEFAULT_COLORS.blue,
                        45 => term.bg = DEFAULT_COLORS.purple,
                        46 => term.bg = DEFAULT_COLORS.cyan,
                        47 => term.bg = DEFAULT_COLORS.white,
                        49 => term.bg = DEFAULT_COLORS.black,
                        // Bright Foreground
                        90 => term.fg = DEFAULT_COLORS.bright_black,
                        91 => term.fg = DEFAULT_COLORS.bright_red,
                        92 => term.fg = DEFAULT_COLORS.bright_green,
                        93 => term.fg = DEFAULT_COLORS.bright_yellow,
                        94 => term.fg = DEFAULT_COLORS.bright_blue,
                        95 => term.fg = DEFAULT_COLORS.bright_purple,
                        96 => term.fg = DEFAULT_COLORS.bright_cyan,
                        97 => term.fg = DEFAULT_COLORS.bright_white,
                        // Bright Background
                        100 => term.bg = DEFAULT_COLORS.bright_black,
                        101 => term.bg = DEFAULT_COLORS.bright_red,
                        102 => term.bg = DEFAULT_COLORS.bright_green,
                        103 => term.bg = DEFAULT_COLORS.bright_yellow,
                        104 => term.bg = DEFAULT_COLORS.bright_blue,
                        105 => term.bg = DEFAULT_COLORS.bright_purple,
                        106 => term.bg = DEFAULT_COLORS.bright_cyan,
                        107 => term.bg = DEFAULT_COLORS.bright_white,

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
                term.grid.cursor.col = term.grid.cursor.col.saturating_sub(*count as usize);
            }
            // Erase in Line (EL)
            'K' => {
                // If no parameter is provided, it defaults to 0
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);

                match mode {
                    0 => {
                        term.clear_to_end();
                    }
                    1 => {
                        term.clear_to_start();
                    }
                    2 => {
                        term.clear_line();
                    }
                    _ => {} // Ignore unsupported modes
                }
            }

            // Cursor Horizontal Absolute (CHA)
            'G' | '`' => {
                let target_col = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);

                // VT coordinates are 1-indexed. Rust arrays are 0-indexed.
                let zero_indexed_col = (*target_col as usize).saturating_sub(1);
                term.grid.cursor.col = zero_indexed_col.min(term.grid.width.saturating_sub(1));
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
