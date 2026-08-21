use palette::Srgb;

use crate::color::NamedColor::{
    Black, Blue, BrightBlack, BrightBlue, BrightCyan, BrightGreen, BrightMagenta, BrightRed,
    BrightWhite, BrightYellow, Cyan, Green, Magenta, Red, White, Yellow,
};
pub type Rgb = Srgb;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Color {
    Named(NamedColor),
    Indexed(u8),
    Rgb(Rgb),
    DefaultFg,
    DefaultBg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

impl From<NamedColor> for Color {
    fn from(value: NamedColor) -> Self {
        Self::Named(value)
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self::Indexed(value)
    }
}

pub struct ColorPalette {
    /// The 256 Color palette
    /// 0-15 are the named colors
    /// 16-231 are the 6x6x6 color cube
    /// 232-255 are grayscale
    colors: [Rgb; 256],
    default_fg: Rgb,
    default_bg: Rgb,
    cursor_color: Rgb,
}

macro_rules! srgb {
    ($hex:expr) => {{
        let c = color_hex::color_from_hex!($hex);
        // Convert from u8 [0, 255] to Srgb<f32> [0.0, 1.0]
        Srgb::new(
            c[0] as f32 / 255.0,
            c[1] as f32 / 255.0,
            c[2] as f32 / 255.0,
        )
    }};
}

impl ColorPalette {
    pub fn resolve_color(&self, color: Color) -> Rgb {
        match color {
            Color::Named(name) => self.colors[name as usize],
            Color::Indexed(idx) => self.colors[idx as usize],
            Color::Rgb(rgb) => rgb,
            Color::DefaultFg => self.default_fg,
            Color::DefaultBg => self.default_bg,
        }
    }

    #[inline]
    pub fn get_cursor_color(&self) -> Rgb {
        self.cursor_color
    }

    const fn intialize() -> Self {
        let mut colors = [srgb!("#000000"); 256];

        // Initialize 6x6x6 Color cube for 256 color space
        let mut i = 0;
        while i < 216 {
            let mut c = i;
            let mut r = c / 36;
            c = c - (r * 36);
            let mut g = c / 6;
            c = c - (g * 6);
            let mut b = c;
            r = if r > 0 { r * 40 + 55 } else { 0 };
            g = if g > 0 { g * 40 + 55 } else { 0 };
            b = if b > 0 { b * 40 + 55 } else { 0 };
            colors[i + 16] = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            i += 1;
        }

        // Initialize 24 color grayscale ramp
        i = 0;
        while i < 24 {
            let bright = (8 + i * 10) as f32 / 255.0;
            colors[i + 232] = Srgb::new(bright, bright, bright);
            i += 1;
        }

        Self {
            colors,
            default_fg: srgb!("#000000"),
            default_bg: srgb!("#000000"),
            cursor_color: srgb!("#000000"),
        }
    }

    const fn set_color(mut self, name: NamedColor, value: Rgb) -> Self {
        self.colors[name as usize] = value;
        self
    }

    const fn set_default_fg(mut self, value: Rgb) -> Self {
        self.default_fg = value;
        self
    }

    const fn set_default_bg(mut self, value: Rgb) -> Self {
        self.default_bg = value;
        self
    }

    const fn set_cursor(mut self, value: Rgb) -> Self {
        self.cursor_color = value;
        self
    }
}

pub const DEFAULT_PALETTE: ColorPalette = ColorPalette::intialize()
    // #232627
    .set_color(Black, srgb!("#232627"))
    // #ed1515
    .set_color(Red, srgb!("#ed1515"))
    // #11d116
    .set_color(Green, srgb!("#11d116"))
    // #f67400
    .set_color(Yellow, srgb!("#f67400"))
    // #1d99f3
    .set_color(Blue, srgb!("#1d99f3"))
    // #9b59b6
    .set_color(Magenta, srgb!("#9b59b6"))
    // #1abc9c
    .set_color(Cyan, srgb!("#1abc9c"))
    // #fcfcfc
    .set_color(White, srgb!("#fcfcfc"))
    // #7f8c8d
    .set_color(BrightBlack, srgb!("#7f8c8d"))
    // #c0392b
    .set_color(BrightRed, srgb!("#c0392b"))
    // #1cdc9a
    .set_color(BrightGreen, srgb!("#1cdc9a"))
    // #fdbc4b
    .set_color(BrightYellow, srgb!("#fdbc4b"))
    // #3daee9
    .set_color(BrightBlue, srgb!("#3daee9"))
    // #8e44ad
    .set_color(BrightMagenta, srgb!("#8e44ad"))
    // #16a085
    .set_color(BrightCyan, srgb!("#16a085"))
    // #ffffff
    .set_color(BrightWhite, srgb!("#ffffff"))
    // #fcfcfc
    .set_default_fg(srgb!("#fcfcfc"))
    // #232627
    .set_default_bg(srgb!("#232627"))
    // #fcfcfc
    .set_cursor(srgb!("#fcfcfc"));
