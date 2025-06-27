use std::ops::Add;

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

fn main() {
    let mut window = Window::new("Kajet", WIDTH, HEIGHT, WindowOptions::default()).unwrap();
    window.set_cursor_style(CursorStyle::Crosshair);
    window.set_target_fps(60);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut camera = Camera::default();
    let mut brush = Brush {
        color: 0x00ffffff,
        thickness: 6.0,
    };

    let mut events = Vec::<Event>::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let pos = camera.to_camera_coords((x as u32, y as u32));

            if window.get_mouse_down(MouseButton::Left) {
                events.push(Event::Draw(pos));
            } else {
                if !matches!(events.last(), Some(Event::MouseUp)) {
                    events.push(Event::MouseUp);
                }
            }
        }

        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            let inc = 20.0;
            match key {
                Key::Left => camera.pos.x += inc,
                Key::Right => camera.pos.x -= inc,
                Key::Up => camera.pos.y += inc,
                Key::Down => camera.pos.y -= inc,
                _ => (),
            }
        }


        if let Some((scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if window.is_key_down(Key::LeftCtrl) {
                brush.thickness = brush.thickness.add(scroll_y.signum()).clamp(1.0, 30.0);
            } else {
                // TODO: zoom in the direction of the mouse cursor
                camera.zoom = camera.zoom.add(0.1 * scroll_y.signum()).clamp(0.1, 5.0);
            }
        }

        canvas.clear();
        for i in 1..events.len() {
            if let (Event::Draw(p0), Event::Draw(p1)) = (events[i - 1], events[i]) {
                Line { start: p0, end: p1 }.draw(&mut canvas, &camera, brush);
            }
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
