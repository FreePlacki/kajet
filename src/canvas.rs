use std::ops::{Index, IndexMut};

use crate::graphics::Point;

pub struct Canvas {
    buffer: Vec<u32>,
    width: u32,
    height: u32,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0; width * height],
            width: width as u32,
            height: height as u32,
        }
    }

    pub fn in_bounds(&self, pos: Point) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    pub fn get_buffer(&self) -> &[u32] {
        &self.buffer
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

impl Index<Point> for Canvas {
    type Output = u32;
    fn index(&self, index: Point) -> &Self::Output {
        let idx = index.y * self.width + index.x;
        &self[idx as usize]
    }
}

impl IndexMut<Point> for Canvas {
    fn index_mut(&mut self, index: Point) -> &mut Self::Output {
        let idx = index.y * self.width + index.x;
        &mut self[idx as usize]
    }
}
