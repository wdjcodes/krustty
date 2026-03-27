pub type Rgb = [u8; 3];

pub struct ColorProfile {
    pub black: Rgb,
    pub red: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub blue: Rgb,
    pub purple: Rgb,
    pub cyan: Rgb,
    pub white: Rgb,
    pub bright_black: Rgb,
    pub bright_red: Rgb,
    pub bright_green: Rgb,
    pub bright_yellow: Rgb,
    pub bright_blue: Rgb,
    pub bright_purple: Rgb,
    pub bright_cyan: Rgb,
    pub bright_white: Rgb,
}

pub const DEFAULT_COLORS: &ColorProfile = &ColorProfile {
    black: [b'\x08'; 3],
    red: [b'\xd2', b'\x00', b'\x00'],
    green: [b'\x6a', b'\x99', b'\x55'],
    yellow: [b'\xf0', b'\xe6', b'\x8c'],
    blue: [b'\x0d', b'\x73', b'\xcc'],
    purple: [b'\x77', b'\x2f', b'\xb0'],
    cyan: [b'\x27', b'\x93', b'\x70'],
    white: [b'\xcc'; 3],
    bright_black: [b'\x1f'; 3],
    bright_red: [b'\xff', b'\x27', b'\x27'],
    bright_green: [b'\x84', b'\xbf', b'\x6a'],
    bright_yellow: [b'\xff', b'\xff', b'\x55'],
    bright_blue: [b'\x1a', b'\x8f', b'\xff'],
    bright_purple: [b'\x69', b'\x5a', b'\xbc'],
    bright_cyan: [b'\x31', b'\xb9', b'\x8d'],
    bright_white: [b'\x9d'; 3],
};
