use std::{
    ops::{Add, Mul},
    time::Instant,
};

use canvas::Canvas;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use tiny_skia::Color;

use crate::{
    camera::Camera,
    graphics::{Brush, Drawable, Line, Point},
};

mod camera;
mod canvas;
mod graphics;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FPS: u32 = 60;

fn main() {
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
    window.set_cursor_style(CursorStyle::Crosshair);
    window.set_target_fps(FPS as usize);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut camera = Camera::default();
    let mut brush = Brush {
        color: Color::WHITE,
        thickness: 6.0,
    };

    let mut lines = Vec::<Line>::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let resized = {
            let sz = window.get_size();
            canvas.set_size((sz.0 as u32, sz.1 as u32))
        };

        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let pos = camera.to_camera_coords((x as u32, y as u32));

            if window.get_mouse_down(MouseButton::Left) {
                if let Some(line) = lines.last_mut() {
                    if line.finished {
                        lines.push(Line::new(pos, brush));
                    } else if line.points.last().unwrap().dist(pos) >= 5.0 {
                        line.points.push(pos);
                    }
                } else {
                    lines.push(Line::new(pos, brush));
                }
            } else if let Some(line) = lines.last_mut() {
                line.finished = true;
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
        let now = Instant::now();
        canvas.clear();
        for line in &lines {
            line.draw(&mut canvas, &camera);
        }
        dbg!(now.elapsed());
        if let Some(l) = lines.first() {
            dbg!(l.points.len());
        }

        // if updated || resized {
        //     let now = Instant::now();
        //     canvas.clear();
        //     for i in 1..events.len() {
        //         if let (Event::Draw(p0, b), Event::Draw(p1, _)) = (events[i - 1], events[i]) {
        //             Line { start: p0, end: p1 }.draw(&mut canvas, &camera, b);
        //         }
        //     }
        //     println!("Drawing time: {} ms", now.elapsed().as_millis());
        //     println!("Events count: {}", events.len());
        // } else if events.len() >= 2 {
        //     if let (Event::Draw(p0, b), Event::Draw(p1, _)) =
        //         (events[events.len() - 2], events[events.len() - 1])
        //     {
        //         Line { start: p0, end: p1 }.draw(&mut canvas, &camera, b);
        //     }
        // }

        window
            .update_with_buffer(
                &canvas.get_buffer(),
                canvas.width as usize,
                canvas.height as usize,
            )
            .unwrap();
    }
}
