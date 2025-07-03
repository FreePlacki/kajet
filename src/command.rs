use std::collections::VecDeque;

use crate::{
    graphics::{Image, Line},
    scene::Contents,
};

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

pub trait Command {
    fn execute(&mut self, contents: &mut Contents);
    fn undo(&mut self, contents: &mut Contents);
}

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
