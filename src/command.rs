use std::collections::VecDeque;
use std::fmt::Debug;

use raylib::math::Vector2;

use crate::graphics::{Contents, Eraser, Image, ImageId, Line};

#[derive(Debug)]
pub struct CommandInvoker {
    undos: VecDeque<Box<dyn Command>>,
    redos: VecDeque<Box<dyn Command>>,
    buffer_size: usize,
}

impl CommandInvoker {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            undos: VecDeque::<Box<dyn Command>>::new(),
            redos: VecDeque::<Box<dyn Command>>::new(),
            buffer_size,
        }
    }

    pub fn push<T: Command + 'static>(&mut self, command: T) {
        while self.undos.len() >= self.buffer_size {
            self.undos.pop_front();
        }

        self.undos.push_back(Box::new(command));
    }

    pub fn undo(&mut self, contents: &mut Contents) {
        if let Some(mut command) = self.undos.pop_back() {
            command.undo(contents);
            while self.redos.len() >= self.buffer_size {
                self.redos.pop_front();
            }
            self.redos.push_back(command);
        }
    }

    pub fn redo(&mut self, contents: &mut Contents) {
        if let Some(mut command) = self.redos.pop_back() {
            command.execute(contents);
            while self.undos.len() >= self.buffer_size {
                self.undos.pop_front();
            }
            self.undos.push_back(command);
        }
    }
}

pub trait Command: Debug {
    fn execute(&mut self, contents: &mut Contents);
    fn undo(&mut self, contents: &mut Contents);
}

#[derive(Debug)]
pub struct DrawLine {
    line: Line,
}

impl DrawLine {
    pub fn new(line: Line) -> Self {
        Self { line }
    }
}

impl Command for DrawLine {
    fn execute(&mut self, contents: &mut Contents) {
        contents.lines.push(self.line.clone());
    }

    fn undo(&mut self, contents: &mut Contents) {
        contents.lines.pop();
    }
}

#[derive(Debug)]
pub struct PasteImage {
    image: Image,
}

impl PasteImage {
    pub fn new(image: Image) -> Self {
        Self { image }
    }
}

impl Command for PasteImage {
    fn execute(&mut self, contents: &mut Contents) {
        contents.images.push(self.image.clone());
    }

    fn undo(&mut self, contents: &mut Contents) {
        contents.images.pop();
    }
}

#[derive(Debug)]
pub struct RemoveImage {
    image: Image,
}

impl RemoveImage {
    pub fn new(image: Image) -> Self {
        Self { image }
    }
}

impl Command for RemoveImage {
    fn execute(&mut self, contents: &mut Contents) {
        contents.images.retain(|i| i.id != self.image.id);
    }

    fn undo(&mut self, contents: &mut Contents) {
        contents.images.push(self.image.clone());
    }
}

#[derive(Debug)]
pub struct ResizeImage {
    id: ImageId,
    start_scale: f32,
    end_scale: f32,
}

impl ResizeImage {
    pub fn new(id: ImageId, start_scale: f32, end_scale: f32) -> Self {
        Self {
            id,
            start_scale,
            end_scale,
        }
    }
}

impl Command for ResizeImage {
    fn execute(&mut self, contents: &mut Contents) {
        if let Some(img) = contents.image(self.id) {
            img.scale = self.end_scale;
        }
    }

    fn undo(&mut self, contents: &mut Contents) {
        if let Some(img) = contents.image(self.id) {
            img.scale = self.start_scale;
        }
    }
}

#[derive(Debug)]
pub struct MoveImage {
    id: ImageId,
    start_pos: Vector2,
    end_pos: Vector2,
}

impl MoveImage {
    pub fn new(id: ImageId, start_pos: Vector2, end_pos: Vector2) -> Self {
        Self {
            id,
            start_pos,
            end_pos,
        }
    }
}

impl Command for MoveImage {
    fn execute(&mut self, contents: &mut Contents) {
        if let Some(img) = contents.image(self.id) {
            img.pos = self.end_pos;
        }
    }

    fn undo(&mut self, contents: &mut Contents) {
        if let Some(img) = contents.image(self.id) {
            img.pos = self.start_pos;
        }
    }
}

#[derive(Debug)]
pub struct AddEraser {
    eraser: Eraser,
}

impl AddEraser {
    pub fn new(eraser: Eraser) -> Self {
        Self { eraser }
    }
}

impl Command for AddEraser {
    fn execute(&mut self, contents: &mut Contents) {
        contents.erasers.push(self.eraser);
    }

    fn undo(&mut self, contents: &mut Contents) {
        contents.erasers.pop();
    }
}
