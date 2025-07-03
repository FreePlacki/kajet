use std::collections::VecDeque;

use crate::{
    graphics::{Image, Line},
    scene::Contents,
};

pub struct CommandInvoker {
    undos: VecDeque<Box<dyn Command>>,
    redos: VecDeque<Box<dyn Command>>,
}

impl CommandInvoker {
    pub fn new() -> Self {
        Self {
            undos: VecDeque::<Box<dyn Command>>::new(),
            redos: VecDeque::<Box<dyn Command>>::new(),
        }
    }

    pub fn push<T: Command + 'static>(&mut self, command: T) {
        self.undos.push_back(Box::new(command));
    }

    pub fn undo(&mut self, contents: &mut Contents) {
        if let Some(mut command) = self.undos.pop_back() {
            command.undo(contents);
            self.redos.push_back(command);
        }
    }

    pub fn redo(&mut self, contents: &mut Contents) {
        if let Some(mut command) = self.redos.pop_back() {
            command.execute(contents);
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
