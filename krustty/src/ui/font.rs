use std::collections::HashMap;

use rust_fontconfig::FcPattern;

pub struct CachedGlyph {
    /// The (x, y) position in the atlas (in pixels)
    pub x: u32,
    pub y: u32,
}

/// provides mechanisms to resolve a single character to a rendered
/// glyph to make text rendering fast and easy on the gpu
pub struct GlyphCache {
    pub atlas_size: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    /// Maps a character + settings to a location in the atlas
    pub cache: HashMap<char, CachedGlyph>,
    /// The raw pixel data (Grayscale/Alpha) to be sent to the GPU
    pub pixel_data: Vec<u8>,
    next_index: u32,
    font: fontdue::Font,
}

impl GlyphCache {
    pub fn new(atlas_size: u32, cell_width: u32, cell_height: u32) -> Self {
        let fc = rust_fontconfig::FcFontCache::build();
        let font_match = fc
            .query(
                &FcPattern {
                    monospace: rust_fontconfig::PatternMatch::True,
                    ..Default::default()
                },
                &mut Vec::new(),
            )
            .expect("Could not find a monospace font, krustty is not currently shipped with one");

        let font_bytes = fc
            .get_font_bytes(&font_match.id)
            .expect("Error loading font");

        let font = fontdue::Font::from_bytes(font_bytes, Default::default()).unwrap();
        Self {
            atlas_size,
            cell_width,
            cell_height,
            cache: HashMap::new(),
            // Initialize a clear atlas (Alpha = 0)
            pixel_data: vec![0_u8; (atlas_size * atlas_size) as usize],
            next_index: 0,
            font,
        }
    }

    /// Rasterizes a char and packs it into the atlas.
    /// Returns the pixel coordinates of the new glyph.
    pub fn load_glyph(&mut self, c: char, px: f32) {
        let (metrics, bitmap) = self.font.rasterize(c, px);
        // Calculate grid position
        let cols = self.atlas_size / self.cell_width;
        let row = self.next_index / cols;
        let col = self.next_index % cols;

        let atlas_x = col * self.cell_width;
        let atlas_y = row * self.cell_height;

        // Copy fontdue bitmap into our large atlas buffer
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let dest_x = atlas_x + x as u32;
                let dest_y = atlas_y + y as u32;

                // Ensure we don't write out of bounds
                if dest_x < self.atlas_size && dest_y < self.atlas_size {
                    let dest_idx = (dest_y * self.atlas_size + dest_x) as usize;
                    let src_idx = y * metrics.width + x;
                    self.pixel_data[dest_idx] = bitmap[src_idx];
                }
            }
        }

        let glyph = CachedGlyph {
            x: atlas_x,
            y: atlas_y,
        };
        self.cache.insert(c, glyph);
        self.next_index += 1;
    }

    // Loads all of the common printable english ascii characters into the cache
    pub fn load_ascii(&mut self) {
        for c in '!'..='~' {
            self.load_glyph(c, 16.0);
        }
    }
}
