use std::ops::Add;

use crate::{camera::Camera, canvas::Canvas};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Add<f32> for Point {
    type Output = Self;
    fn add(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Draw(Point),
    MouseUp,
}

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub color: u32,
    pub thickness: f32,
}

pub trait Drawable {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush);
}

pub struct FilledRect {
    pub pos: Point,
}

impl Drawable for FilledRect {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush) {
        let (x_start, y_start) = camera.to_canvas_coords(self.pos);
        let (x_end, y_end) = camera.to_canvas_coords(self.pos + brush.thickness);

        for y in y_start..y_end {
            for x in x_start..x_end {
                let p = (x, y);
                if !canvas.in_bounds(p) {
                    continue;
                }
                canvas[(p.0 as u32, p.1 as u32)] = brush.color;
            }
        }
    }
}

pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Drawable for Line {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush) {
        let z = 1.0 / camera.zoom;
        let (mut x0, mut y0) = (self.start.x, self.start.y);
        let (x1, y1) = (self.end.x, self.end.y);

        let dx = (x1 - x0).abs() * z;
        let sx = if x0 < x1 { 1.0 } else { -1.0 };
        let dy = -(y1 - y0).abs() * z;
        let sy = if y0 < y1 { 1.0 } else { -1.0 };
        let mut err = dx + dy;

        for _ in 0..1000 {
            FilledRect {
                pos: Point { x: x0, y: y0 },
            }
            .draw(canvas, camera, brush);

            let tol = 1.0;
            if (x0 - x1).abs() <= tol && (y0 - y1).abs() <= tol {
                break;
            }
            let e2 = 2.0 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
