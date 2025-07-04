use tiny_skia::{
    LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Point, Stroke, Transform,
};

use crate::{camera::Camera, canvas::Canvas, config::Config};

#[derive(Debug, Clone, Copy)]
pub struct Color(pub u32); // ARGB

impl Color {
    pub fn from_rgba(color: &[u8]) -> Self {
        Self(u32::from_be_bytes([color[3], color[0], color[1], color[2]]))
    }

    pub fn to_skia(self) -> tiny_skia::Color {
        let bytes = self.0.to_be_bytes();
        tiny_skia::Color::from_rgba8(bytes[1], bytes[2], bytes[3], bytes[0])
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

pub struct FilledCircle {
    pub pos: Point,
    pub brush: Brush,
}

impl Drawable for FilledCircle {
    fn draw(&self, canvas: &mut Canvas, _: &Camera) {
        let r = (self.brush.thickness / 2.0).max(1.0);
        let start = self.pos - Point::from_xy(r, r);
        let end = self.pos + Point::from_xy(r, r);
        let (x_start, y_start) = (start.x as i32, start.y as i32);
        let (x_end, y_end) = (end.x as i32, end.y as i32);

        if !canvas.in_bounds((x_start, y_start)) && !canvas.in_bounds((x_end, y_end)) {
            return;
        }

        for y in y_start..y_end {
            for x in x_start..x_end {
                let p = (x, y);
                if canvas.in_bounds(p) && self.pos.distance(Point::from_xy(x as f32, y as f32)) <= r
                {
                    canvas.overlay[p.1 as usize * canvas.width as usize + p.0 as usize] =
                        self.brush.color.0;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
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
        paint.set_color(self.brush.color.to_skia());

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

#[derive(Clone)]
pub struct Image {
    pub pos: Point,
    pub pixmap: Pixmap,
    pub is_selected: bool,
    pub id: usize,
    border_color: Color,
}

impl Image {
    pub fn new(pos: Point, pixmap: Pixmap, id: usize, config: &Config) -> Self {
        Self {
            pos,
            pixmap,
            is_selected: false,
            id,
            border_color: config.colors[0],
        }
    }

    pub fn in_bounds(&self, point: Point, camera: &Camera) -> bool {
        let point = camera.to_camera_coords((point.x as u32, point.y as u32));
        point.x >= self.pos.x
            && point.x <= self.pos.x + self.pixmap.width() as f32
            && point.y >= self.pos.y
            && point.y <= self.pos.y + self.pixmap.height() as f32
    }
}

impl Drawable for Image {
    fn draw(&self, canvas: &mut Canvas, camera: &Camera) {
        if self.is_selected {
            let (w, h) = (self.pixmap.width() as f32, self.pixmap.height() as f32);
            Line {
                points: vec![
                    self.pos,
                    self.pos + Point::from_xy(w, 0.0),
                    self.pos + Point::from_xy(w, h),
                    self.pos + Point::from_xy(0.0, h),
                    self.pos,
                ],
                finished: true,
                brush: Brush {
                    color: self.border_color,
                    thickness: 5.0,
                },
            }
            .draw(canvas, camera);
        }

        canvas.pixmap.draw_pixmap(
            0,
            0,
            self.pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::from_translate(self.pos.x - camera.pos.x, self.pos.y - camera.pos.y)
                .post_scale(camera.zoom, camera.zoom),
            None,
        );
    }
}
