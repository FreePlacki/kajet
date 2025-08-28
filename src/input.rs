use std::rc::Rc;

use raylib::{RaylibHandle, ffi::KeyboardKey};

use crate::config::Config;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Undo,
    Redo,
    NextColor,
    PrevColor,
    Paste,
    Remove,
    UpLayer,
    DownLayer,
    None,
}

#[derive(Debug)]
pub struct Keybind {
    keys: Box<[Box<[KeyboardKey]>]>,
    action: Action,
}

impl Keybind {
    pub fn new(keys: Box<[Box<[KeyboardKey]>]>, action: Action) -> Self {
        Self { keys, action }
    }
    pub fn action(&self, rl: &RaylibHandle) -> Option<Action> {
        for combo in &self.keys {
            if let Some((&last, rest)) = combo.split_last()
                && rl.is_key_pressed(last)
                && rest.iter().all(|&k| rl.is_key_down(k))
            {
                return Some(self.action);
            }
        }

        None
    }
}

pub struct InputHandler {
    config: Rc<Config>,
}

impl InputHandler {
    pub fn new(config: Rc<Config>) -> Self {
        Self { config }
    }

    pub fn interpret(&self, rl: &RaylibHandle) -> Action {
        for keybind in &self.config.keybinds {
            if let Some(action) = keybind.action(rl) {
                return action;
            }
        }
        Action::None
    }
}
