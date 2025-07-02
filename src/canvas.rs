use tiny_skia::Pixmap;

use crate::graphics::Color;

pub struct Canvas {
    pub pixmap: Pixmap,
    pub width: u32,
    pub height: u32,
    pub overlay: Vec<u32>,
    background: Color,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            pixmap: Pixmap::new(width, height).unwrap(),
            width,
            height,
            overlay: vec![background.0; (width * height) as usize],
            background,
        }
    }

    pub fn in_bounds(&self, pos: (i32, i32)) -> bool {
        pos.0 >= 0 && pos.0 < self.width as i32 && pos.1 >= 0 && pos.1 < self.height as i32
    }

    pub fn get_buffer(&self) -> Vec<u32> {
        self.pixmap
            .data()
            .chunks(4)
            .map(Color::from_rgba)
            .zip(&self.overlay)
            .map(|(buff, over)| {
                if *over == self.background.0 {
                    buff.0
                } else {
                    *over
                }
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(self.background.to_skia());
    }

    pub fn clear_overlay(&mut self) {
        self.overlay.fill(self.background.0);
    }

    pub fn set_size(&mut self, size: (u32, u32)) -> bool {
        if self.width == size.0 && self.height == size.1 {
            return false;
        }

        self.width = size.0;
        self.height = size.1;
        self.pixmap = Pixmap::new(self.width, self.height).unwrap();
        self.overlay = vec![self.background.0; (self.width * self.height) as usize];

        true
    }
}
