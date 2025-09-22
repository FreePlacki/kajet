#![windows_subsystem = "windows"]

use crate::{config::Config, scene::Scene};
use arboard::Clipboard;
use std::{env, process};

mod command;
mod config;
mod graphics;
mod input;
mod scene;
mod state;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

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

    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title("Kajet")
        .resizable()
        .msaa_4x()
        .log_level(raylib::ffi::TraceLogLevel::LOG_WARNING)
        .build();

    rl.set_target_fps(config.fps);
    rl.hide_cursor();

    let mut scene = Scene::new(config, clipboard, &mut rl);

    while !rl.window_should_close() {
        scene.process_frame(&thread, &mut rl);
    }
}
