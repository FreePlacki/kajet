use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Point, Stroke, Transform};

use crate::{camera::Camera, canvas::Canvas};

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
            pb.move_to(p0.x, p0.y);
            pb.line_to(p1.x, p1.y);
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
