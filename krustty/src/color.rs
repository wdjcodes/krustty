use palette::Srgb;
pub type Rgb = Srgb;

pub struct ColorProfile {
    pub fg: Srgb,
    pub bg: Srgb,
    pub black: Srgb,
    pub red: Srgb,
    pub green: Srgb,
    pub yellow: Srgb,
    pub blue: Srgb,
    pub purple: Srgb,
    pub cyan: Srgb,
    pub white: Srgb,
    pub bright_black: Srgb,
    pub bright_red: Srgb,
    pub bright_green: Srgb,
    pub bright_yellow: Srgb,
    pub bright_blue: Srgb,
    pub bright_purple: Srgb,
    pub bright_cyan: Srgb,
    pub bright_white: Srgb,
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

pub const DEFAULT_COLORS: &ColorProfile = &ColorProfile {
    // #fcfcfc
    fg: srgb!("#fcfcfc"),
    // #232627
    bg: srgb!("#232627"),
    // ##232627
    black: srgb!("#232627"),
    // #ed1515
    red: srgb!("#ed1515"),
    // #11d116
    green: srgb!("#11d116"),
    // #f67400
    yellow: srgb!("#f67400"),
    // #1d99f3
    blue: srgb!("#1d99f3"),
    // #9b59b6
    purple: srgb!("#9b59b6"),
    // #1abc9c
    cyan: srgb!("#1abc9c"),
    // #fcfcfc
    white: srgb!("#fcfcfc"),
    // #7f8c8d
    bright_black: srgb!("#7f8c8d"),
    // #c0392b
    bright_red: srgb!("#c0392b"),
    // #1cdc9a
    bright_green: srgb!("#1cdc9a"),
    // #fdbc4b
    bright_yellow: srgb!("#fdbc4b"),
    // #3daee9
    bright_blue: srgb!("#3daee9"),
    // #8e44ad
    bright_purple: srgb!("#8e44ad"),
    // #16a085
    bright_cyan: srgb!("#16a085"),
    // #ffffff
    bright_white: srgb!("#ffffff"),
};
