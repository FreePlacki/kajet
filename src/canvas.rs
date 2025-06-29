use std::ops::{Index, IndexMut};

pub struct Canvas {
    buffer: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0; width * height],
            width: width as u32,
            height: height as u32,
        }
    }

    pub fn in_bounds(&self, pos: (i32, i32)) -> bool {
        pos.0 >= 0 && pos.0 < self.width as i32 && pos.1 >= 0 && pos.1 < self.height as i32
    }

    pub fn get_buffer(&self) -> &[u32] {
        &self.buffer
    }

    pub fn clear(&mut self) {
        self.buffer.iter_mut().for_each(|i| *i = 0);
    }

    pub fn set_size(&mut self, size: (usize, usize)) -> bool {
        if self.width == size.0 as u32 && self.height == size.1 as u32 {
            return false;
        }

        self.width = size.0 as u32;
        self.height = size.1 as u32;
        self.buffer = vec![0; (self.width * self.height) as usize];

        true
    }
}

impl Index<usize> for Canvas {
    type Output = u32;
    fn index(&self, index: usize) -> &Self::Output {
        &self.buffer[index]
    }
}

impl IndexMut<usize> for Canvas {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.buffer[index]
    }
}

impl Index<(u32, u32)> for Canvas {
    type Output = u32;
    fn index(&self, index: (u32, u32)) -> &Self::Output {
        let idx = index.1 * self.width + index.0;
        &self[idx as usize]
    }
}

impl IndexMut<(u32, u32)> for Canvas {
    fn index_mut(&mut self, index: (u32, u32)) -> &mut Self::Output {
        let idx = index.1 * self.width + index.0;
        &mut self[idx as usize]
    }
}
