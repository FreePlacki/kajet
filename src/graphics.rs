use std::ops::{Add, Sub};

use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

use crate::{
    camera::Camera,
    canvas::{self, Canvas},
};

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
pub struct Brush {
    pub color: Color,
    pub thickness: f32,
}

pub trait Drawable {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera);
}

// pub struct FilledCircle {
//     pub pos: Point,
// }
//
// impl Drawable for FilledCircle {
//     fn draw(&self, canvas: &mut Canvas, camera: &Camera, brush: Brush) {
//         let r = brush.thickness / 2.0;
//         let (x_start, y_start) = camera.to_canvas_coords(self.pos - r);
//         let (x_end, y_end) = camera.to_canvas_coords(self.pos + r);
//
//         if !canvas.in_bounds((x_start, y_start)) && !canvas.in_bounds((x_end, y_end)) {
//             return;
//         }
//
//         for y in y_start..y_end {
//             for x in x_start..x_end {
//                 let p = (x, y);
//                 if canvas.in_bounds(p)
//                     && self.pos.dist(camera.to_camera_coords((x as u32, y as u32))) <= r
//                 {
//                     canvas[(p.0 as u32, p.1 as u32)] = brush.color;
//                 }
//             }
//         }
//     }
// }

#[derive(Debug)]
pub struct Line {
    pub points: Vec<Point>,
    pub finished: bool,
    pub brush: Brush,
}

impl Line {
    pub fn new(start: Point, brush: Brush) -> Self {
        Self {
            points: vec![start],
            finished: false,
            brush,
        }
    }
    //
    // const INSIDE: u8 = 0;
    // const LEFT: u8 = 1 << 0;
    // const RIGHT: u8 = 1 << 1;
    // const BOTTOM: u8 = 1 << 2;
    // const TOP: u8 = 1 << 3;
    //
    // fn compute_out_code(p: Point, bounds: (Point, Point)) -> u8 {
    //     let (Point { x: xmin, y: ymin }, Point { x: xmax, y: ymax }) = bounds;
    //     let mut code = Self::INSIDE;
    //     if p.x < xmin {
    //         code |= Self::LEFT;
    //     } else if p.x > xmax {
    //         code |= Self::RIGHT;
    //     }
    //     if p.y < ymin {
    //         code |= Self::BOTTOM;
    //     } else if p.y > ymax {
    //         code |= Self::TOP;
    //     }
    //     code
    // }
    //
    // fn cohen_sutherland_clip(
    //     mut p0: Point,
    //     mut p1: Point,
    //     camera: &Camera,
    //     canvas: &Canvas,
    // ) -> Option<(Point, Point)> {
    //     let bounds = camera.get_bounds(canvas);
    //     let (Point { x: xmin, y: ymin }, Point { x: xmax, y: ymax }) = bounds;
    //
    //     let mut code0 = Self::compute_out_code(p0, bounds);
    //     let mut code1 = Self::compute_out_code(p1, bounds);
    //
    //     loop {
    //         if code0 == 0 && code1 == 0 {
    //             // both inside -> no modification
    //             return Some((p0, p1));
    //         } else if code0 & code1 != 0 {
    //             // share an outside zone -> trivial reject
    //             return None;
    //         } else {
    //             // at least one endpoint is outside; pick it
    //             let out_code = if code0 != 0 { code0 } else { code1 };
    //             // find intersection
    //             let (x, y) = if out_code & Self::TOP != 0 {
    //                 let x = p0.x + (p1.x - p0.x) * (ymax - p0.y) / (p1.y - p0.y);
    //                 (x, ymax)
    //             } else if out_code & Self::BOTTOM != 0 {
    //                 let x = p0.x + (p1.x - p0.x) * (ymin - p0.y) / (p1.y - p0.y);
    //                 (x, ymin)
    //             } else if out_code & Self::RIGHT != 0 {
    //                 let y = p0.y + (p1.y - p0.y) * (xmax - p0.x) / (p1.x - p0.x);
    //                 (xmax, y)
    //             } else {
    //                 // LEFT
    //                 let y = p0.y + (p1.y - p0.y) * (xmin - p0.x) / (p1.x - p0.x);
    //                 (xmin, y)
    //             };
    //             // replace outside point
    //             if out_code == code0 {
    //                 p0.x = x;
    //                 p0.y = y;
    //                 code0 = Self::compute_out_code(p0, bounds);
    //             } else {
    //                 p1.x = x;
    //                 p1.y = y;
    //                 code1 = Self::compute_out_code(p1, bounds);
    //             }
    //         }
    //     }
    // }
}

impl Drawable for Line {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera) {
        if self.points.len() < 2 {
            return;
        }

        let mut pb = PathBuilder::new();
        for seg in self.points.windows(2) {
            let p0 = seg[0];
            let p1 = seg[1];
            // if let Some((p0, p1)) = Self::cohen_sutherland_clip(p0, p1, camera, canvas) {
                pb.move_to(p0.x, p0.y);
                pb.line_to(p1.x, p1.y);
            // }
        }

        let path = pb.finish().unwrap();

        let mut paint = Paint::default();
        paint.set_color(self.brush.color);

        let stroke = Stroke {
            width: self.brush.thickness,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };

        canvas.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            Transform::from_translate(-camera.pos.x, -camera.pos.y)
                .post_scale(camera.zoom, camera.zoom),
            None,
        );
    }
}
