#![windows_subsystem = "windows"]

use std::{env, process};

use arboard::Clipboard;
use minifb::{Key, Window, WindowOptions};

use crate::{config::Config, scene::Scene};

mod camera;
mod canvas;
mod command;
mod config;
mod graphics;
mod input;
mod scene;
mod state;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FPS: u32 = 120;

fn usage(prog_name: &str) {
    eprintln!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    eprintln!("{}", env!("CARGO_PKG_DESCRIPTION"));
    eprintln!();
    eprintln!("Usage: {prog_name} [config path]");
    process::exit(1);
}

fn main() {
    let mut args = env::args();
    let prog_name = args.next().unwrap();

    let config_path = args.next().inspect(|p| {
        // if someone tries --help or -h
        if p.starts_with("-") {
            usage(&prog_name);
        }
    });

    let config = match Config::from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Couldn't parse config: {e}");
            Config::default()
        }
    };

    let clipboard = match Clipboard::new() {
        Ok(c) => Some(c),
        Err(_) => {
            eprintln!("[ERROR] Couldn't initialize clipboard");
            None
        }
    };

    let mut window = Window::new(
        "Kajet",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions {
            resize: true,
            ..Default::default()
        },
    )
    .unwrap();
    window.set_cursor_visibility(false);
    window.set_target_fps(FPS as usize);

    let mut scene = Scene::new(config, clipboard, &mut window);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        scene.process_frame(&mut window);
    }
}
