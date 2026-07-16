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
    #[expect(
        unused,
        reason = "Will be used when more support for true color is added"
    )]
    Rgb(Rgb),
    Default(Component),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Component {
    Fg,
    Bg,
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
            Color::Default(comp) => match comp {
                Component::Fg => self.default_fg,
                Component::Bg => self.default_bg,
            },
        }
    }

    #[inline]
    pub fn get_cursor_color(&self) -> Rgb {
        self.cursor_color
    }

    const fn empty() -> Self {
        Self {
            colors: [srgb!("#000000"); 256],
            default_fg: srgb!("#000000"),
            default_bg: srgb!("#000000"),
            cursor_color: srgb!("#000000"),
        }
    }

    const fn set_named_color(mut self, name: NamedColor, value: Rgb) -> Self {
        self.colors[name as usize] = value;
        self
    }

    #[expect(
        unused,
        reason = "will be used when more support for 256 color is added"
    )]
    const fn set_indexed_color(mut self, idx: u8, value: Rgb) -> Self {
        self.colors[idx as usize] = value;
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

pub const DEFAULT_PALETTE: ColorPalette = ColorPalette::empty()
    // #232627
    .set_named_color(Black, srgb!("#232627"))
    // #ed1515
    .set_named_color(Red, srgb!("#ed1515"))
    // #11d116
    .set_named_color(Green, srgb!("#11d116"))
    // #f67400
    .set_named_color(Yellow, srgb!("#f67400"))
    // #1d99f3
    .set_named_color(Blue, srgb!("#1d99f3"))
    // #9b59b6
    .set_named_color(Magenta, srgb!("#9b59b6"))
    // #1abc9c
    .set_named_color(Cyan, srgb!("#1abc9c"))
    // #fcfcfc
    .set_named_color(White, srgb!("#fcfcfc"))
    // #7f8c8d
    .set_named_color(BrightBlack, srgb!("#7f8c8d"))
    // #c0392b
    .set_named_color(BrightRed, srgb!("#c0392b"))
    // #1cdc9a
    .set_named_color(BrightGreen, srgb!("#1cdc9a"))
    // #fdbc4b
    .set_named_color(BrightYellow, srgb!("#fdbc4b"))
    // #3daee9
    .set_named_color(BrightBlue, srgb!("#3daee9"))
    // #8e44ad
    .set_named_color(BrightMagenta, srgb!("#8e44ad"))
    // #16a085
    .set_named_color(BrightCyan, srgb!("#16a085"))
    // #ffffff
    .set_named_color(BrightWhite, srgb!("#ffffff"))
    // #fcfcfc
    .set_default_fg(srgb!("#fcfcfc"))
    // #232627
    .set_default_bg(srgb!("#232627"))
    // #fcfcfc
    .set_cursor(srgb!("#fcfcfc"));
