use tiny_skia::{Color, Pixmap};

pub struct Canvas {
    pub pixmap: Pixmap,
    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixmap: Pixmap::new(width, height).unwrap(),
            width,
            height,
        }
    }

    pub fn in_bounds(&self, pos: (i32, i32)) -> bool {
        pos.0 >= 0 && pos.0 < self.width as i32 && pos.1 >= 0 && pos.1 < self.height as i32
    }

    pub fn get_buffer(&self) -> Vec<u32> {
        self.pixmap
            .data()
            .chunks(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(Color::BLACK);
    }

    pub fn set_size(&mut self, size: (u32, u32)) -> bool {
        if self.width == size.0 && self.height == size.1 {
            return false;
        }

        self.width = size.0 as u32;
        self.height = size.1 as u32;
        self.pixmap = Pixmap::new(self.width, self.height).unwrap();

        true
    }
}
