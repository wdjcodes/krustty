use std::{collections::HashMap, rc::Rc};

use tracing::{debug, error, info, warn};

use crate::ui::{GpuHandle, texture::Texture};

pub struct CachedGlyph {
    /// The (x, y) position in the atlas (in pixels)
    pub x: u32,
    pub y: u32,
}

pub struct FontCache {
    fonts: Vec<Font>,
    fc: fontconfig::Fontconfig,
}

impl FontCache {
    /// Initialize the FontCache loading the systems default monospace font
    /// # Panics
    /// If no font is found to initialize the cache with the function will panic
    fn new() -> Self {
        let fc = fontconfig::Fontconfig::new().expect("Failed to initalize fontconfig");
        let font_desc = fc
            .find("monospace", None)
            .expect("Could not find a monospace font, krustty is not currently shipped with one");

        info!(
            "Selected font: {} index {:?} path {:?}",
            font_desc.name, font_desc.index, font_desc.path
        );

        let font_bytes = std::fs::read(&font_desc.path).unwrap_or_else(|_| {
            panic!(
                "Failed to read font file: {}",
                font_desc.path.to_string_lossy()
            )
        });

        let f = fontdue::Font::from_bytes(font_bytes.as_ref(), Default::default()).unwrap();
        let font = Font::new(f);
        let fonts = vec![font];

        Self { fonts, fc }
    }

    /// Returns the index for the font to be used to render the provided glyph. If one has not
    /// already been loaded searches the system for a monospace font that provides
    /// it. If none is found this returns the default font (`self.fonts[0]`) with
    /// the expectation that the default symbol will be rendered from that font.
    fn select_font(&mut self, c: char) -> &Font {
        if let Some(pos) = self.fonts.iter().position(|f| f.font.has_glyph(c)) {
            &self.fonts[pos]
        } else {
            if let Some(f) = self.load_font(c) {
                self.fonts.push(f);
                self.fonts.last().unwrap()
            } else {
                &self.fonts[0]
            }
        }
    }

    /// Tries to load a font that contains the specified glyph.
    /// Returns `None` if one was not found on the system
    fn load_font(&mut self, c: char) -> Option<Font> {
        let mut pat = fontconfig::Pattern::new(&self.fc);
        pat.add_string(fontconfig::FC_FAMILY, c"monospace");
        let mut cs = fontconfig::CharSet::create();
        cs.add_char(c);
        pat.add_charset(cs);
        let font_desc = if let Some(fd) = self.fc.find_pattern(&mut pat) {
            fd
        } else {
            warn!("Failed to find a font for: u+{:04x}", c as u32);
            return None;
        };

        info!(
            "Selected font: {} index {:?} path {:?}",
            font_desc.name, font_desc.index, font_desc.path
        );

        let font_bytes = if let Ok(font_bytes) = std::fs::read(&font_desc.path) {
            font_bytes
        } else {
            error!(
                "Failed to read font file: {}",
                font_desc.path.to_string_lossy()
            );
            return None;
        };

        let f = fontdue::Font::from_bytes(font_bytes.as_ref(), Default::default()).unwrap();
        Some(Font::new(f))
    }
}
pub struct Font {
    pub font: fontdue::Font,
    pub baseline: f32,
}

impl Font {
    fn new(font: fontdue::Font) -> Self {
        let baseline = font
            .horizontal_line_metrics(FONT_PX)
            .expect("Failed to get font metrics")
            .ascent;
        Self { font, baseline }
    }
}

/// provides mechanisms to resolve a single character to a rendered
/// glyph to make text rendering fast and easy on the gpu
pub struct GlyphCache {
    /// Maps a character + settings to a location in the atlas
    cache: HashMap<char, CachedGlyph>,
    atlas: AtlasData,
    texture: Option<Rc<Texture>>,
    fonts: FontCache,
}

const FONT_PX: f32 = 14.666667;

impl GlyphCache {
    pub fn new(atlas_size: u32, cell_width: u32, cell_height: u32) -> Self {
        let fonts = FontCache::new();
        let atlas = AtlasData::new(atlas_size, cell_width, cell_height);
        Self {
            cache: HashMap::new(),
            atlas,
            texture: None,
            fonts,
        }
    }

    // Loads all of the common printable english ascii characters into the cache
    pub fn load_ascii(&mut self) {
        for c in '!'..='~' {
            let font = self.fonts.select_font(c);
            let glyph = self.atlas.load_glyph(c, font, FONT_PX);
            self.cache.insert(c, glyph);
        }
    }

    pub fn get(&mut self, c: char) -> &CachedGlyph {
        let font = self.fonts.select_font(c);
        self.cache
            .entry(c)
            .or_insert_with(|| self.atlas.load_glyph(c, font, FONT_PX))
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

    pub fn get_atlas_or_init(&mut self, device: &wgpu::Device) -> Rc<Texture> {
        self.texture
            .get_or_insert_with(|| Rc::new(Texture::new(device, "atlas_texture", 1024, 1024)))
            .clone()
    }

    pub fn update_atlas_texture(&mut self, gpu: &GpuHandle) {
        // Do not update texture if not dirty
        if !self.is_dirty() {
            return;
        }

        let texture = self.get_atlas_or_init(&gpu.device);
        texture.write_texture(&gpu.queue, &self.atlas.data);
        self.clean();
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
    pub fn load_glyph(&mut self, c: char, font: &Font, px: f32) -> CachedGlyph {
        let (metrics, bitmap) = font.font.rasterize(c, px);
        // Calculate grid position
        let cols = self.atlas_size / self.cell_width;
        let row = self.next_index / cols;
        let col = self.next_index % cols;

        // Calculate the offset needed to place glyph on baseline
        let y_off = font.baseline as i32 - (metrics.ymin + metrics.height as i32);

        debug!(
            "Char: {} U+{:04x} Baseline: {} y_off: {} height: {} y_min: {} x_min: {} width: {}",
            c,
            c as u32,
            font.baseline,
            y_off,
            metrics.height,
            metrics.ymin,
            metrics.xmin,
            metrics.width
        );

        let atlas_x = col * self.cell_width;
        let atlas_y = row * self.cell_height;

        // Copy fontdue bitmap into our large atlas buffer
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let dest_x = (atlas_x + x as u32).saturating_add_signed(metrics.xmin);
                let dest_y = (atlas_y + y as u32).saturating_add_signed(y_off);

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
