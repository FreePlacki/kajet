use canvas::Canvas;
use minifb::{CursorStyle, Key, MouseButton, MouseMode, Window, WindowOptions};

use crate::graphics::{Brush, Drawable, Line, Point};

mod canvas;
mod graphics;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    let mut canvas = Canvas::new(WIDTH, HEIGHT);

    let mut window = Window::new("Kajet", WIDTH, HEIGHT, WindowOptions::default())
        .expect("Unable to create the window");
    window.set_cursor_style(CursorStyle::Crosshair);

    window.set_target_fps(60);

    let brush = Brush {
        color: 0x00ffffff,
        thickness: 6,
    };

    let mut points = Vec::<Point>::new();
    let mut is_drawing = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let pos = Point::new(x as u32, y as u32);

            if window.get_mouse_down(MouseButton::Left) {
                if is_drawing {
                    if let Some(&prev) = points.iter().next_back() {
                        let line = Line {
                            start: prev,
                            end: pos,
                        };
                        line.draw(&mut canvas, brush);
                    }
                }
                points.push(pos);
                is_drawing = true;
            } else {
                is_drawing = false;
            }
        }

        if let Some((scroll_x, scroll_y)) = window.get_scroll_wheel() {
            println!("Scrolling {} - {}", scroll_x, scroll_y);
        }

        window
            .update_with_buffer(&canvas.get_buffer(), WIDTH, HEIGHT)
            .unwrap();
    }
}
