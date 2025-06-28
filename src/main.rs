use std::{
    ops::{Add, Mul},
    time::Instant,
};

use canvas::Canvas;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

use crate::{
    camera::Camera,
    graphics::{Brush, Drawable, Event, Line},
};

mod camera;
mod canvas;
mod graphics;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;
const FPS: usize = 60;

fn main() {
    let mut window = Window::new("Kajet", WIDTH, HEIGHT, WindowOptions::default()).unwrap();
    window.set_cursor_style(CursorStyle::Crosshair);
    window.set_target_fps(FPS);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut camera = Camera::default();
    let mut brush = Brush {
        color: 0x00ffffff,
        thickness: 6.0,
    };

    let mut events = Vec::<Event>::new();

    let mut i = 0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let pos = camera.to_camera_coords((x as u32, y as u32));

            if window.get_mouse_down(MouseButton::Left) {
                events.push(Event::Draw(pos, brush));
            } else if matches!(events.last(), Some(Event::Draw(_, _))) {
                events.push(Event::MouseUp);
            }
        }

        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            let inc = 20.0;
            match key {
                Key::Left => camera.pos.x -= inc,
                Key::Right => camera.pos.x += inc,
                Key::Up => camera.pos.y -= inc,
                Key::Down => camera.pos.y += inc,
                _ => (),
            }
        }

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            dbg!(camera.pos);
            if window.is_key_down(Key::LeftCtrl) {
                brush.thickness = brush.thickness.add(scroll_y.signum()).clamp(1.0, 30.0);
            } else {
                let prev_zoom = camera.zoom;
                camera.zoom = camera.zoom.mul(1.0 + scroll_y / 10.0).clamp(0.1, 10.0);
                if let Some((mouse_x, mouse_y)) = window.get_mouse_pos(MouseMode::Discard) {
                    camera.pos.x -= mouse_x * (1.0 / camera.zoom - 1.0 / prev_zoom);
                    camera.pos.y -= mouse_y * (1.0 / camera.zoom - 1.0 / prev_zoom);
                }
            }
        }

        let now = Instant::now();
        // TODO: only clear when zoom or pos changed
        canvas.clear();
        for i in 1..events.len() {
            if let (Event::Draw(p0, b), Event::Draw(p1, _)) = (events[i - 1], events[i]) {
                Line { start: p0, end: p1 }.draw(&mut canvas, &camera, b);
            }
        }
        i += 1;
        if i == FPS {
            i = 0;
            println!("Drawing time: {} ms", now.elapsed().as_millis());
            println!("Events count: {}", events.len());
        }

        window
            .update_with_buffer(
                canvas.get_buffer(),
                canvas.width as usize,
                canvas.height as usize,
            )
            .unwrap();
    }
}
