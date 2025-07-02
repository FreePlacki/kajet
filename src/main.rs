#![windows_subsystem = "windows"]

use std::{env, process};

use arboard::Clipboard;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

use crate::{config::Config, scene::Scene};

mod camera;
mod canvas;
mod config;
mod graphics;
mod scene;

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
        // is someone tries --help or -h
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

    let mut clipboard = match Clipboard::new() {
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

    let mut scene = Scene::new(WIDTH, HEIGHT, config);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        scene.next_frame(window.get_size(), window.get_mouse_pos(MouseMode::Discard));

        if window.get_mouse_down(MouseButton::Left) && !window.is_key_down(Key::LeftCtrl) {
            scene.on_pen_down();
        } else {
            scene.on_pen_up();
            if window.get_mouse_down(MouseButton::Right) {
                scene.on_move();
                window.set_cursor_style(CursorStyle::ClosedHand);
                window.set_cursor_visibility(true);
            } else {
                scene.update_cursor();
                window.set_cursor_visibility(false);
            }
        }

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if window.is_key_down(Key::LeftCtrl) {
                scene.update_thickness(scroll_y);
            } else {
                scene.update_zoom(scroll_y);
            }
        }

        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Left => scene.update_color(false),
                Key::Right => scene.update_color(true),
                Key::Delete | Key::D => scene.try_delete_images(),
                _ => (),
            }
        }

        if window.is_key_down(Key::LeftCtrl) {
            if window.is_key_pressed(Key::V, KeyRepeat::No) {
                if let Some(ref mut clipboard) = clipboard {
                    if let Ok(image_data) = clipboard.get_image() {
                        scene.try_paste_image(image_data);
                    }
                }
            }

            if window.get_mouse_down(MouseButton::Left) {
                scene.try_select_image();
            }
        }

        scene.draw(&mut window);
    }
}
