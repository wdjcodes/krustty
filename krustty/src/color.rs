use palette::Srgb;
pub type Rgb = Srgb;

pub struct ColorProfile {
    pub fg: Srgb<u8>,
    pub bg: Srgb<u8>,
    pub black: Srgb<u8>,
    pub red: Srgb<u8>,
    pub green: Srgb<u8>,
    pub yellow: Srgb<u8>,
    pub blue: Srgb<u8>,
    pub purple: Srgb<u8>,
    pub cyan: Srgb<u8>,
    pub white: Srgb<u8>,
    pub bright_black: Srgb<u8>,
    pub bright_red: Srgb<u8>,
    pub bright_green: Srgb<u8>,
    pub bright_yellow: Srgb<u8>,
    pub bright_blue: Srgb<u8>,
    pub bright_purple: Srgb<u8>,
    pub bright_cyan: Srgb<u8>,
    pub bright_white: Srgb<u8>,
}

pub const DEFAULT_COLORS: &ColorProfile = &ColorProfile {
    // #cccccc
    fg: Srgb::<u8>::new(0xcc, 0xcc, 0xcc),
    // #1f1f1f
    bg: Srgb::<u8>::new(0x1f, 0x1f, 0x1f),
    // #080808
    black: Srgb::new(0x08, 0x08, 0x08),
    // #d20000
    red: Srgb::new(0xd2, 0x00, 0x00),
    // #6a9955
    green: Srgb::new(0x6a, 0x99, 0x55),
    // #f0e68c
    yellow: Srgb::new(0xf0, 0xe6, 0x8c),
    // #0d73cc
    blue: Srgb::new(0x0d, 0x73, 0xcc),
    // #772fb0
    purple: Srgb::new(0x77, 0x2f, 0xb0),
    //  #279370
    cyan: Srgb::new(0x27, 0x93, 0x70),
    // #cccccc
    white: Srgb::new(0xcc, 0xcc, 0xcc),
    // #1f1f1f
    bright_black: Srgb::new(0x1f, 0x1f, 0x1f),
    // #ff2727
    bright_red: Srgb::new(0xff, 0x27, 0x27),
    // #84bf6a
    bright_green: Srgb::new(0x84, 0xbf, 0x6a),
    // #ffff55
    bright_yellow: Srgb::new(0xff, 0xff, 0x55),
    // #1a8fff
    bright_blue: Srgb::new(0x1a, 0x8f, 0xff),
    // #695abc
    bright_purple: Srgb::new(0x69, 0x5a, 0xbc),
    // #31b98d
    bright_cyan: Srgb::new(0x31, 0xb9, 0x8d),
    // #9d9d9d
    bright_white: Srgb::new(0x9d, 0x9d, 0x9d),
};
