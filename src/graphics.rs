use std::rc::Rc;

use raylib::{
    color::Color,
    math::{Rectangle, Vector2},
    prelude::{RaylibDraw, RaylibDrawHandle},
    texture::Texture2D,
};

use crate::{
    camera::{Camera, CameraCanvasCoords},
    config::Config,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageId(usize);
pub struct Contents {
    pub overlay: Vec<Box<dyn Drawable>>,
    pub lines: Vec<Line>,
    pub images: Vec<Image>,
    pub erasers: Vec<Eraser>,
    pub z: usize,
    next_image_id: ImageId,
}

impl Contents {
    pub fn new() -> Self {
        Self {
            overlay: vec![],
            lines: vec![],
            images: vec![],
            erasers: vec![],
            z: 0,
            next_image_id: ImageId(0),
        }
    }

    pub fn next_image_id(&mut self) -> ImageId {
        self.next_image_id.0 += 1;
        ImageId(self.next_image_id.0 - 1)
    }

    pub fn image(&mut self, id: ImageId) -> Option<&mut Image> {
        self.images.iter_mut().find(|i| i.id == id)
    }

    pub fn remove_image(&mut self, id: ImageId) -> Option<Image> {
        let Some(index) = self.images.iter().position(|i| i.id == id) else {
            return None;
        };

        Some(self.images.remove(index))
    }

    pub fn move_image_up(&mut self, id: ImageId) {
        let max_z = self.z;

        if let Some(img) = self.image(id) {
            img.z = (img.z + 1).min(max_z);
        }
    }

    pub fn move_image_down(&mut self, id: ImageId) {
        if let Some(img) = self.image(id) {
            img.z = img.z.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub color: Color,
    pub thickness: f32,
}

pub trait Drawable {
    fn z(&self) -> usize;
    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera);
}

#[derive(Debug, Clone, Copy)]
pub struct FilledCircle {
    pub pos: Vector2,
    pub brush: Brush,
}

impl Drawable for FilledCircle {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, _: &Camera) {
        let r = (self.brush.thickness / 2.0).max(1.0);

        d.draw_circle(self.pos.x as i32, self.pos.y as i32, r, self.brush.color);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FilledRect {
    pub rect: Rectangle,
    pub color: Color,
}

impl Drawable for FilledRect {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let Vector2 { mut x, mut y } =
            Vector2::new(self.rect.x, self.rect.y).to_camera_coords(camera);
        let mut w = self.rect.width.to_camera_coords(camera);
        let mut h = self.rect.height.to_camera_coords(camera);

        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }

        d.draw_rectangle(x as i32, y as i32, w as i32, h as i32, self.color);
    }
}

#[derive(Debug)]
pub struct StraightLine {
    pub start: Vector2,
    pub end: Vector2,
    pub brush: Brush,
}

impl Drawable for StraightLine {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        d.draw_line_ex(
            self.start.to_camera_coords(camera),
            self.end.to_camera_coords(camera),
            self.brush.thickness.to_camera_coords(camera),
            self.brush.color,
        );
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub points: Vec<Vector2>,
    pub finished: bool,
    pub brush: Brush,
    z: usize,
}

impl Line {
    pub fn new(start: Vector2, brush: Brush, z: usize) -> Self {
        Self {
            points: vec![start],
            finished: false,
            brush,
            z,
        }
    }

    fn draw_longer(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        assert!(self.points.len() >= 4);

        let pts = &self
            .points
            .iter()
            .map(|p| p.to_camera_coords(camera))
            .collect::<Box<_>>();
        d.draw_spline_catmull_rom(
            pts,
            self.brush.thickness.to_camera_coords(camera),
            self.brush.color,
        );
    }

    fn draw_shorter(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let r = self.brush.thickness / 2.0;
        let r = r.to_camera_coords(camera);
        for seg in self.points.windows(2) {
            let p0 = seg[0].to_camera_coords(camera);
            let p1 = seg[1].to_camera_coords(camera);
            d.draw_circle_v(p0, r, self.brush.color);
            d.draw_circle_v(p1, r, self.brush.color);
            d.draw_line_ex(
                p0,
                p1,
                self.brush.thickness.to_camera_coords(camera),
                self.brush.color,
            );
        }
    }
}

impl Drawable for Line {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        match self.points.len() {
            ..2 => {}
            2..4 => self.draw_shorter(d, camera),
            4.. => self.draw_longer(d, camera),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub pos: Vector2,
    pub texture: Rc<Texture2D>,
    pub is_selected: bool,
    pub scale: f32,
    pub id: ImageId,
    pub z: usize,
    border_color: Color,
}

impl Image {
    pub fn new(
        pos: Vector2,
        texture: Texture2D,
        scale: f32,
        id: ImageId,
        z: usize,
        config: &Config,
    ) -> Self {
        Self {
            pos,
            texture: Rc::new(texture),
            is_selected: false,
            scale,
            id,
            z,
            border_color: config.colors[0],
        }
    }

    pub fn width(&self) -> f32 {
        self.texture.width as f32 * self.scale
    }

    pub fn height(&self) -> f32 {
        self.texture.height as f32 * self.scale
    }

    pub fn in_bounds(&self, point: Vector2, camera: &Camera) -> bool {
        let point = point.to_canvas_coords(camera);
        point.x >= self.pos.x
            && point.x <= self.pos.x + self.width()
            && point.y >= self.pos.y
            && point.y <= self.pos.y + self.height()
    }
}

impl Drawable for Image {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let draw_pos = self.pos.to_camera_coords(camera);
        let draw_size = Vector2::new(
            self.texture.width as f32 * self.scale.to_camera_coords(camera),
            self.texture.height as f32 * self.scale.to_camera_coords(camera),
        );

        d.draw_texture_ex(
            &*self.texture,
            draw_pos,
            0.0,
            self.scale.to_camera_coords(camera),
            Color::WHITE,
        );

        if self.is_selected {
            d.draw_rectangle_lines(
                draw_pos.x as i32,
                draw_pos.y as i32,
                draw_size.x as i32,
                draw_size.y as i32,
                self.border_color,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Eraser {
    rect: Rectangle,
    color: Color,
    z: usize,
}

impl Eraser {
    pub fn new(rect: Rectangle, color: Color, z: usize) -> Self {
        Self { rect, color, z }
    }
}

impl Drawable for Eraser {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        FilledRect {
            rect: self.rect,
            color: self.color,
        }
        .draw(d, camera);
    }
}
