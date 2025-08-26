use std::{collections::HashSet, rc::Rc};

use minifb::{Key, KeyRepeat, Window};

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
    keys: Box<[Box<[Key]>]>,
    action: Action,
}

impl Keybind {
    pub fn new(keys: Box<[Box<[Key]>]>, action: Action) -> Self {
        Self { keys, action }
    }
    pub fn action(&self, down: &HashSet<Key>, pressed: Key) -> Option<Action> {
        for combo in &self.keys {
            if let Some((&last, rest)) = combo.split_last() {
                if pressed == last && rest.iter().all(|k| down.contains(k)) {
                    return Some(self.action);
                }
            }
        }

        return None;
    }
}

pub struct InputHandler {
    config: Rc<Config>,
}

impl InputHandler {
    pub fn new(config: Rc<Config>) -> Self {
        Self { config }
    }

    pub fn interpret(&self, window: &Window) -> Action {
        if let Some(&pressed) = window.get_keys_pressed(KeyRepeat::Yes).first() {
            let down = window.get_keys().into_iter().collect();
            for keybind in &self.config.keybinds {
                if let Some(action) = keybind.action(&down, pressed) {
                    return action;
                }
            }
            Action::None
        } else {
            Action::None
        }
    }
}
