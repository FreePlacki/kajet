use std::{collections::HashMap, fs};

use configparser::ini::Ini;

use crate::graphics::Color;

#[derive(Debug)]
pub struct Config {
    pub thickness: f32,
    pub scroll_sensitivity: f32,
    pub background: Color,
    pub colors: Vec<Color>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            thickness: 5.0,
            scroll_sensitivity: 1.0,
            background: Color(0),
            colors: vec![
                Color(0xFFFFFFFF),
                Color(0xFFFF0000),
                Color(0xFF00FF00),
                Color(0xFF0000FF),
            ],
        }
    }
}

impl Config {
    pub fn from_file(path: Option<String>) -> Result<Self, String> {
        let mut conf = Ini::new();

        let default_file = include_str!("../kajet.conf").to_string();
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
                        Err(_) => {
                            eprintln!("[ERROR] Couldn't create a config file in {path_str}.")
                        }
                    };
                    default_file
                }
            }
        } else {
            default_file
        };

        let map = conf.read(file)?;

        let thickness = Self::get_value(&map, "brush", "thickness")?;
        let thickness = match thickness.parse::<f32>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        if thickness <= 0.0 {
            return Err(format!("Brush thickness should be > 0.0, got {thickness}"));
        }

        let scroll_sensitivity = Self::get_value(&map, "controls", "scroll_sensitivity")?;
        let scroll_sensitivity = match scroll_sensitivity.parse::<f32>() {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }?;
        if scroll_sensitivity <= 0.0 {
            return Err(format!("Scroll sensitivity should be > 0.0, got {thickness}"));
        }

        let background = Self::get_value(&map, "colors", "background")?;
        let background = Self::parse_color(&background)?;

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
            return Err("At least one color has to be defined.".to_string());
        }

        Ok(Self {
            thickness,
            scroll_sensitivity,
            background,
            colors,
        })
    }

    fn get_value(
        map: &HashMap<String, HashMap<String, Option<String>>>,
        section: &str,
        key: &str,
    ) -> Result<String, String> {
        map.get(section)
            .ok_or(format!("Expected [{section}] section"))?
            .get(key)
            .ok_or(format!("Expected '{key}' key"))?
            .clone()
            .ok_or(format!("Expected '{key}' value"))
    }

    fn parse_color(color: &str) -> Result<Color, String> {
        let s = color.trim_start_matches("0x");
        let col = u32::from_str_radix(s, 16);
        match col {
            Ok(c) => Ok(Color(c | 0xFF000000)),
            Err(e) => Err(e.to_string()),
        }
    }
}
