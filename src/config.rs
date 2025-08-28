use std::{collections::HashMap, fs};

use configparser::ini::Ini;
use raylib::{color::Color, ffi::KeyboardKey};

use crate::input::{Action, Keybind};

const DEFAULT_CONFIG: &'static str = include_str!("../kajet.conf");

#[derive(Debug)]
pub struct Config {
    pub thickness: f32,
    pub fps: u32,
    pub scroll_sensitivity: f32,
    pub undo_buffer_size: usize,
    pub background: Color,
    pub colors: Box<[Color]>,
    pub keybinds: Box<[Keybind]>,
}

impl Default for Config {
    fn default() -> Self {
        let file = DEFAULT_CONFIG.to_string();
        let mut conf = Ini::new();
        let map = conf
            .read(file)
            .expect("Default config should get parsed correctly.");

        macro_rules! parse {
            ($func:ident) => {
                Self::$func(&map).expect("Default config values should be correct.")
            };
        }

        Self {
            thickness: parse!(parse_thickness),
            fps: parse!(parse_fps),
            scroll_sensitivity: parse!(parse_scroll_sensitivity),
            undo_buffer_size: parse!(parse_undo_buffer_size),
            background: parse!(parse_background),
            colors: parse!(parse_colors),
            keybinds: parse!(parse_keybinds),
        }
    }
}
type ConfigMap = HashMap<String, HashMap<String, Option<String>>>;

impl Config {
    pub fn from_file(path: Option<String>) -> Result<Self, String> {
        let default_file = DEFAULT_CONFIG.to_string();
        let file = if let Some(path) = path {
            match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[ERROR] Couldn't read the config file {path}");
                    default_file
                }
            }
        } else if let Some(mut path) = dirs::config_dir() {
            path.push("kajet");
            path.set_extension("conf");
            match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => {
                    let path_str = path.to_str().unwrap();
                    match fs::write(&path, &default_file) {
                        Ok(()) => eprintln!("[INFO] Created a config file in {path_str}."),
                        Err(e) => {
                            eprintln!("[ERROR] Couldn't create a config file in {path_str} ({e}).")
                        }
                    };
                    default_file
                }
            }
        } else {
            default_file
        };

        let mut conf = Ini::new();
        let map = conf.read(file)?;

        Ok(Self::from_string(&map))
    }

    fn from_string(map: &ConfigMap) -> Self {
        let default_config = Config::default();

        macro_rules! parse {
            ($name:ident, $func:ident) => {
                Self::$func(&map).unwrap_or_else(|e| {
                    eprintln!("{e} (using default value)");
                    default_config.$name
                })
            };
        }

        Self {
            thickness: parse!(thickness, parse_thickness),
            fps: parse!(fps, parse_fps),
            scroll_sensitivity: parse!(scroll_sensitivity, parse_scroll_sensitivity),
            undo_buffer_size: parse!(undo_buffer_size, parse_undo_buffer_size),
            background: parse!(background, parse_background),
            colors: parse!(colors, parse_colors),
            keybinds: parse!(keybinds, parse_keybinds),
        }
    }

    fn get_value(map: &ConfigMap, section: &str, key: &str) -> Result<String, String> {
        map.get(section)
            .ok_or(format!("Expected [{section}] section"))?
            .get(key)
            .ok_or(format!("Expected '{key}' key"))?
            .clone()
            .ok_or(format!("Expected '{key}' value"))
    }

    fn parse_color(color: &str) -> Result<Color, String> {
        let s = color.trim_start_matches("0x");
        Color::from_hex(s).map_err(|e| e.to_string())
    }

    fn parse_thickness(map: &ConfigMap) -> Result<f32, String> {
        let thickness = Self::get_value(&map, "brush", "thickness")?;
        let thickness = match thickness.parse::<f32>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        if thickness <= 0.0 {
            Err(format!("Brush thickness should be > 0.0, got {thickness}"))
        } else {
            Ok(thickness)
        }
    }

    fn parse_fps(map: &ConfigMap) -> Result<u32, String> {
        let fps = Self::get_value(&map, "other", "fps")?;
        let fps = match fps.parse::<u32>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        if fps < 1 {
            Err(format!("FPS should be >= 1, got {fps}"))
        } else {
            Ok(fps)
        }
    }

    fn parse_scroll_sensitivity(map: &ConfigMap) -> Result<f32, String> {
        let scroll_sensitivity = Self::get_value(&map, "other", "scroll_sensitivity")?;
        let scroll_sensitivity = match scroll_sensitivity.parse::<f32>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        if scroll_sensitivity <= 0.0 {
            Err(format!(
                "Scroll sensitivity should be > 0.0, got {scroll_sensitivity}"
            ))
        } else {
            Ok(scroll_sensitivity)
        }
    }

    fn parse_undo_buffer_size(map: &ConfigMap) -> Result<usize, String> {
        let undo_buffer_size = Self::get_value(&map, "other", "undo_buffer_size")?;
        let undo_buffer_size = match undo_buffer_size.parse::<usize>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        Ok(undo_buffer_size)
    }

    fn parse_colors(map: &ConfigMap) -> Result<Box<[Color]>, String> {
        let mut colors = Vec::<Color>::new();

        for i in 0..=9 {
            let name = format!("color{i}");
            if !map["colors"].contains_key(&name) {
                continue;
            }
            let color = Self::get_value(&map, "colors", &name)?;
            colors.push(Self::parse_color(&color)?);
        }

        if colors.is_empty() {
            Err("At least one color has to be defined.".to_string())
        } else {
            Ok(colors.into_boxed_slice())
        }
    }

    fn parse_background(map: &ConfigMap) -> Result<Color, String> {
        let background = Self::get_value(&map, "colors", "background")?;
        let background = Self::parse_color(&background)?;
        Ok(background)
    }

    fn parse_keybinds(map: &ConfigMap) -> Result<Box<[Keybind]>, String> {
        let mut keybinds = vec![];

        if let Some(binds) = map.get("keybinds") {
            for (action, keys) in binds {
                let action = Self::parse_action(action);
                if let (Some(action), Some(keys)) = (action, keys) {
                    let mut key_combo = vec![];
                    for keys in keys.split('|') {
                        let keys: Option<Vec<_>> = keys.split('+').map(Self::parse_key).collect();
                        let keys = match keys {
                            Some(k) => k,
                            None => continue,
                        }
                        .into_boxed_slice();
                        key_combo.push(keys);
                    }
                    if key_combo.is_empty() {
                        continue;
                    }

                    keybinds.push(Keybind::new(key_combo.into_boxed_slice(), action));
                }
            }
        }

        Ok(keybinds.into_boxed_slice())
    }

    fn parse_action(s: &str) -> Option<Action> {
        match s.trim().to_lowercase().as_str() {
            "undo" => Some(Action::Undo),
            "redo" => Some(Action::Redo),
            "prev" | "previous" | "prev_color" | "previous_color" => Some(Action::PrevColor),
            "next" | "next_color" => Some(Action::NextColor),
            "paste" | "paste_image" | "clipboard" | "paste_clipboard" => Some(Action::Paste),
            "remove" | "remove_image" | "delete" | "delete_image" => Some(Action::Remove),
            "up" | "up_layer" | "move_up_layer" => Some(Action::UpLayer),
            "down" | "down_layer" | "move_down_layer" => Some(Action::DownLayer),
            a => {
                eprintln!("[CONFIG] Unknown action '{a}'");
                None
            }
        }
    }

    fn parse_key(s: &str) -> Option<KeyboardKey> {
        match s.trim().to_lowercase().as_str() {
            "shift" => Some(KeyboardKey::KEY_LEFT_SHIFT),
            "ctrl" | "control" => Some(KeyboardKey::KEY_LEFT_CONTROL),
            "alt" => Some(KeyboardKey::KEY_LEFT_ALT),
            "caps" | "capslock" => Some(KeyboardKey::KEY_CAPS_LOCK),
            "left" | "leftarrow" => Some(KeyboardKey::KEY_LEFT),
            "right" | "rightarrow" => Some(KeyboardKey::KEY_RIGHT),
            "up" | "uparrow" => Some(KeyboardKey::KEY_UP),
            "down" | "downarrow" => Some(KeyboardKey::KEY_DOWN),
            "tab" => Some(KeyboardKey::KEY_TAB),
            "del" | "delete" => Some(KeyboardKey::KEY_DELETE),
            "back" | "backspace" => Some(KeyboardKey::KEY_BACKSPACE),
            "a" => Some(KeyboardKey::KEY_A),
            "b" => Some(KeyboardKey::KEY_B),
            "c" => Some(KeyboardKey::KEY_C),
            "d" => Some(KeyboardKey::KEY_D),
            "e" => Some(KeyboardKey::KEY_E),
            "f" => Some(KeyboardKey::KEY_F),
            "g" => Some(KeyboardKey::KEY_G),
            "h" => Some(KeyboardKey::KEY_H),
            "i" => Some(KeyboardKey::KEY_I),
            "j" => Some(KeyboardKey::KEY_J),
            "k" => Some(KeyboardKey::KEY_K),
            "l" => Some(KeyboardKey::KEY_L),
            "m" => Some(KeyboardKey::KEY_M),
            "n" => Some(KeyboardKey::KEY_N),
            "o" => Some(KeyboardKey::KEY_O),
            "p" => Some(KeyboardKey::KEY_P),
            "q" => Some(KeyboardKey::KEY_Q),
            "r" => Some(KeyboardKey::KEY_R),
            "s" => Some(KeyboardKey::KEY_S),
            "t" => Some(KeyboardKey::KEY_T),
            "u" => Some(KeyboardKey::KEY_U),
            "v" => Some(KeyboardKey::KEY_V),
            "w" => Some(KeyboardKey::KEY_W),
            "x" => Some(KeyboardKey::KEY_X),
            "y" => Some(KeyboardKey::KEY_Y),
            "z" => Some(KeyboardKey::KEY_Z),
            k => {
                eprintln!("[CONFIG] Unknown key '{k}'");
                None
            }
        }
    }
}
