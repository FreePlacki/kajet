#![windows_subsystem = "windows"]

use std::{
    env,
    ops::{Add, Mul},
    process,
};

use arboard::Clipboard;
use canvas::Canvas;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use tiny_skia::{IntSize, Pixmap, Point};

use crate::{
    camera::Camera,
    config::Config,
    graphics::{Brush, Drawable, FilledCircle, Image, Line},
};

mod camera;
mod canvas;
mod config;
mod graphics;

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

    let mut canvas = Canvas::new(WIDTH, HEIGHT, config.background);
    let mut camera = Camera::default();
    let mut color_idx = 0;
    let mut brush = Brush {
        color: config.colors[color_idx],
        thickness: config.thickness,
    };

    let mut lines = Vec::<Line>::new();
    let mut images = Vec::<Image>::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let resized = {
            let sz = window.get_size();
            canvas.set_size((sz.0 as u32, sz.1 as u32))
        };
        canvas.clear_overlay();

        let mouse = window.get_mouse_pos(MouseMode::Discard);
        if let Some((x, y)) = mouse {
            let pos = camera.to_camera_coords((x as u32, y as u32));

            if window.get_mouse_down(MouseButton::Left) {
                if let Some(line) = lines.last_mut() {
                    if line.finished {
                        lines.push(Line::new(pos, brush));
                    } else if line.points.last().unwrap().distance(pos) >= 5.0 / camera.zoom {
                        line.points.push(pos);
                    }
                } else {
                    lines.push(Line::new(pos, brush));
                }
            } else {
                if let Some(line) = lines.last_mut() {
                    line.finished = true;
                }
                if window.get_mouse_down(MouseButton::Right) {
                    camera.update_pos(Point::from_xy(x, y));
                    window.set_cursor_style(CursorStyle::ClosedHand);
                    window.set_cursor_visibility(true);
                } else {
                    FilledCircle {
                        pos: Point::from_xy(x, y),
                        brush: Brush {
                            color: brush.color,
                            thickness: brush.thickness * camera.zoom,
                        },
                    }
                    .draw(&mut canvas, &camera);
                    camera.end_panning();
                    window.set_cursor_visibility(false);
                }
            }
        }
        camera.update_mouse(mouse.map(|m| Point::from_xy(m.0, m.1)));

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if window.is_key_down(Key::LeftCtrl) {
                brush.thickness = brush.thickness.add(scroll_y.signum()).clamp(1.0, 30.0);
            } else {
                let new_zoom = camera
                    .zoom
                    .mul(1.0 + scroll_y.signum() * 0.25 * config.scroll_sensitivity)
                    .clamp(0.1, 10.0);
                let mouse = window
                    .get_mouse_pos(MouseMode::Discard)
                    .map(|m| Point::from_xy(m.0, m.1));
                camera.update_zoom(new_zoom, mouse);
            }
        }

        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Left => {
                    color_idx = (color_idx - 1) % config.colors.len();
                    brush.color = config.colors[color_idx];
                }
                Key::Right => {
                    color_idx = (color_idx + 1) % config.colors.len();
                    brush.color = config.colors[color_idx];
                }
                _ => (),
            }
        }

        if window.is_key_down(Key::LeftCtrl) && window.is_key_pressed(Key::V, KeyRepeat::No) {
            if let Some(ref mut clipboard) = clipboard {
                if let Ok(image_data) = clipboard.get_image() {
                    let img = Pixmap::from_vec(
                        image_data.bytes.to_vec(),
                        IntSize::from_wh(image_data.width as u32, image_data.height as u32)
                            .unwrap(),
                    );

                    if let Some(img) = img {
                        let pos = {
                            if let Some(m) = mouse {
                                Point::from_xy(
                                    m.0 - image_data.width as f32 / 2.0 * camera.zoom,
                                    m.1 - image_data.height as f32 / 2.0 * camera.zoom,
                                )
                            } else {
                                camera.pos
                            }
                        };
                        let pos = camera.to_camera_coords((pos.x as u32, pos.y as u32));
                        let image = Image { pos, pixmap: img };
                        image.draw(&mut canvas, &camera);
                        images.push(image);
                    }
                }
            }
        }

        let updated = camera.update();
        if updated || resized {
            canvas.clear();
            for img in &images {
                img.draw(&mut canvas, &camera);
            }
            for line in &lines {
                line.draw(&mut canvas, &camera);
            }
        } else if let Some(line) = lines.last() {
            if line.points.len() >= 2 {
                let mut l = Line::new(line.points[line.points.len() - 2], line.brush);
                l.points.push(line.points[line.points.len() - 1]);
                l.draw(&mut canvas, &camera);
            }
        }

        window
            .update_with_buffer(
                &canvas.get_buffer(),
                canvas.width as usize,
                canvas.height as usize,
            )
            .unwrap();
    }
}
