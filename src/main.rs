use std::{
    ops::{Add, Mul},
    time::Instant,
};

use canvas::Canvas;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

use crate::{
    camera::Camera,
    graphics::{Brush, Drawable, Event, Line, Point},
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

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let pos = camera.to_camera_coords((x as u32, y as u32));

            if window.get_mouse_down(MouseButton::Left) {
                let mut push_new = true;
                let mut push_double = true; // to render single dots we need two points
                if let Some(Event::Draw(prev_pos, _)) = events.last() {
                    if prev_pos.dist(pos) < 10.0 / camera.zoom {
                        push_new = false;
                    }
                    push_double = false;
                }

                if push_new {
                    events.push(Event::Draw(pos, brush));
                }
                if push_double {
                    events.push(Event::Draw(pos, brush));
                }
            } else if matches!(events.last(), Some(Event::Draw(_, _))) {
                events.push(Event::MouseUp);
            }
        }

        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            let inc = 40.0 / camera.zoom;
            let delta = match key {
                Key::Left => Some(Point::new(-inc, 0.0)),
                Key::Right => Some(Point::new(inc, 0.0)),
                Key::Up => Some(Point::new(0.0, -inc)),
                Key::Down => Some(Point::new(0.0, inc)),
                _ => None,
            };
            if let Some(delta) = delta {
                camera.update_pos(camera.pos + delta);
            }
        }

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if window.is_key_down(Key::LeftCtrl) {
                brush.thickness = brush.thickness.add(scroll_y.signum()).clamp(1.0, 30.0);
            } else {
                let new_zoom = camera.zoom.mul(1.0 + scroll_y / 10.0).clamp(0.1, 10.0);
                camera.update_zoom(new_zoom, window.get_mouse_pos(MouseMode::Discard));
            }
        }
        let updated = camera.update();

        if updated {
            let now = Instant::now();
            canvas.clear();
            for i in 1..events.len() {
                if let (Event::Draw(p0, b), Event::Draw(p1, _)) = (events[i - 1], events[i]) {
                    Line { start: p0, end: p1 }.draw(&mut canvas, &camera, b);
                }
            }
            println!("Drawing time: {} ms", now.elapsed().as_millis());
            println!("Events count: {}", events.len());
        } else if events.len() >= 2 {
            if let (Event::Draw(p0, b), Event::Draw(p1, _)) =
                (events[events.len() - 2], events[events.len() - 1])
            {
                Line { start: p0, end: p1 }.draw(&mut canvas, &camera, b);
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
