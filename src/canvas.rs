use tiny_skia::Pixmap;

use crate::graphics::Color;

pub struct Canvas {
    pub pixmap: Pixmap,
    pub width: u32,
    pub height: u32,
    pub overlay: Pixmap,
    background: Color,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            pixmap: Pixmap::new(width, height).unwrap(),
            width,
            height,
            overlay: Pixmap::new(width, height).unwrap(),
            background,
        }
    }

    pub fn get_buffer(&self) -> Vec<u32> {
        self.pixmap
            .data()
            .chunks(4)
            .map(Color::from_rgba)
            .zip(self.overlay.data().chunks(4).map(|c| {
                if c[3] == 0 {
                    None
                } else {
                    Some(Color::from_rgba(c).0)
                }
            }))
            .map(|(base, overlay_opt)| overlay_opt.unwrap_or(base.0))
            .collect()
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(self.background.to_skia());
    }

    pub fn clear_overlay(&mut self) {
        self.overlay.fill(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
    }

    pub fn set_size(&mut self, size: (u32, u32)) -> bool {
        if self.width == size.0 && self.height == size.1 {
            return false;
        }

        self.width = size.0;
        self.height = size.1;
        let pixmap = Pixmap::new(self.width, self.height);
        if let Some(pixmap) = pixmap {
            self.pixmap = pixmap;
            self.overlay = Pixmap::new(self.width, self.height).unwrap();
            true
        } else {
            false
        }
    }
}
