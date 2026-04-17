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
    /// Maps a character + settings to a location in the atlas
    cache: HashMap<char, CachedGlyph>,
    font: fontdue::Font,
    atlas: AtlasData,
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
        let atlas = AtlasData::new(atlas_size, cell_width, cell_height);
        Self {
            cache: HashMap::new(),
            // Initialize a clear atlas (Alpha = 0)
            font,
            atlas,
        }
    }

    // Loads all of the common printable english ascii characters into the cache
    pub fn load_ascii(&mut self) {
        for c in '!'..='~' {
            let glyph = self.atlas.load_glyph(&self.font, c, 16.0);
            self.cache.insert(c, glyph);
        }
    }

    pub fn get(&mut self, c: char) -> &CachedGlyph {
        let atlas = &mut self.atlas;
        let font = &self.font;
        self.cache
            .entry(c)
            .or_insert_with(|| atlas.load_glyph(font, c, 16.0))
    }

    pub fn atlas_data(&self) -> &[u8] {
        &self.atlas.data
    }

    pub fn atlas_size(&self) -> u32 {
        self.atlas.atlas_size
    }

    pub fn is_dirty(&self) -> bool {
        self.atlas.dirty
    }

    pub fn clean(&mut self) {
        self.atlas.dirty = false;
    }
}

/// Contains the actual pixel data for the glyph atlas
struct AtlasData {
    pub data: Vec<u8>,
    next_index: u32,
    pub atlas_size: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub dirty: bool,
}

impl AtlasData {
    fn new(size: u32, cell_width: u32, cell_height: u32) -> Self {
        Self {
            data: vec![0_u8; (size * size) as usize],
            next_index: 0,
            atlas_size: size,
            cell_width,
            cell_height,
            dirty: false,
        }
    }
    /// Rasterizes a char and packs it into the atlas.
    /// Returns the pixel coordinates of the new glyph.
    pub fn load_glyph(&mut self, font: &fontdue::Font, c: char, px: f32) -> CachedGlyph {
        let (metrics, bitmap) = font.rasterize(c, px);
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
                    self.data[dest_idx] = bitmap[src_idx];
                }
            }
        }
        self.dirty = true;
        let glyph = CachedGlyph {
            x: atlas_x,
            y: atlas_y,
        };
        self.next_index += 1;
        glyph
    }
}
