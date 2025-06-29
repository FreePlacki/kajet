use std::ops::{Add, Sub};

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

    pub fn dist(&self, other: Point) -> f32 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
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

impl Sub<f32> for Point {
    type Output = Self;
    fn sub(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl Add<Point> for Point {
    type Output = Self;
    fn add(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Point> for Point {
    type Output = Self;
    fn sub(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Draw(Point, Brush),
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

        if !canvas.in_bounds((x_start, y_start)) && !canvas.in_bounds((x_end, y_end)) {
            return;
        }

        for y in y_start..y_end {
            for x in x_start..x_end {
                let p = (x, y);
                if canvas.in_bounds(p) {
                    canvas[(p.0 as u32, p.1 as u32)] = brush.color;
                }
            }
        }
    }
}

pub struct FilledCircle {
    pub pos: Point,
}

impl Drawable for FilledCircle {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush) {
        let r = brush.thickness / 2.0;
        let (x_start, y_start) = camera.to_canvas_coords(self.pos - r);
        let (x_end, y_end) = camera.to_canvas_coords(self.pos + r);

        if !canvas.in_bounds((x_start, y_start)) && !canvas.in_bounds((x_end, y_end)) {
            return;
        }

        for y in y_start..y_end {
            for x in x_start..x_end {
                let p = (x, y);
                if canvas.in_bounds(p)
                    && self.pos.dist(camera.to_camera_coords((x as u32, y as u32))) <= r
                {
                    canvas[(p.0 as u32, p.1 as u32)] = brush.color;
                }
            }
        }
    }
}

pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Line {
    fn can_be_in_bounds(&self, canvas: &Canvas, camera: &Camera, brush: Brush) -> bool {
        let min_x = self.start.x.min(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_x = self.start.x.max(self.end.x);
        let max_y = self.start.y.max(self.end.y);

        let Point {
            x: cam_max_x,
            y: cam_max_y,
        } = camera.to_camera_coords((canvas.width, canvas.height));

        !(max_x + brush.thickness < camera.pos.x
            || max_y + brush.thickness < camera.pos.y
            || min_x - brush.thickness > cam_max_x
            || min_y - brush.thickness > cam_max_y)
    }
}

impl Drawable for Line {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush) {
        if !self.can_be_in_bounds(canvas, camera, brush) {
            return;
        }

        let z = 1.0 / camera.zoom;
        let (mut x0, mut y0) = (self.start.x, self.start.y);
        let (x1, y1) = (self.end.x, self.end.y);

        let dx = (x1 - x0).abs() * z;
        let sx = if x0 < x1 { 1.0 } else { -1.0 };
        let dy = -(y1 - y0).abs() * z;
        let sy = if y0 < y1 { 1.0 } else { -1.0 };
        let mut err = dx + dy;

        // TODO: this is not the correct way to draw a line with thickness
        // consider: https://github.com/ArminJo/Arduino-BlueDisplay/blob/master/src/LocalGUI/ThickLine.hpp
        loop {
            FilledCircle {
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
