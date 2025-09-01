use std::rc::Rc;

use raylib::{
    color::Color,
    math::Vector2,
    prelude::{RaylibDraw, RaylibDrawHandle},
    texture::Texture2D,
};

use crate::{
    camera::Camera,
    config::Config,
    units::{CanvasSpace, Length, Point, Rect, ScreenSpace, Transformable},
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
        let index = self.images.iter().position(|i| i.id == id)?;

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
pub struct Brush<Space> {
    pub color: Color,
    pub thickness: Length<Space>,
}

pub trait Drawable {
    fn z(&self) -> usize;
    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera);
    fn is_visible(&self, camera: &Camera) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct FilledCircle {
    pub pos: Point<ScreenSpace>,
    pub brush: Brush<ScreenSpace>,
}

impl Drawable for FilledCircle {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, _: &Camera) {
        let r = f32::from(self.brush.thickness / 2.0).max(1.0);

        d.draw_circle_v(self.pos, r, self.brush.color);
    }

    fn is_visible(&self, _camera: &Camera) -> bool {
        // always visible since it's in ScreenSpace
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FilledRect {
    pub rect: Rect<CanvasSpace>,
    pub color: Color,
}

impl FilledRect {
    pub fn new(rect: Rect<CanvasSpace>, color: Color) -> Self {
        Self { rect, color }
    }
}

impl Drawable for FilledRect {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let rect = self.rect.normalized().transform(camera);
        d.draw_rectangle_v(rect.pos(), rect.size(), self.color);
    }

    fn is_visible(&self, camera: &Camera) -> bool {
        camera.get_rect().check_collision(self.rect)
    }
}

#[derive(Debug)]
pub struct StraightLine {
    pub start: Point<ScreenSpace>,
    pub end: Point<ScreenSpace>,
    pub brush: Brush<ScreenSpace>,
}

impl Drawable for StraightLine {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, _camera: &Camera) {
        d.draw_line_ex(
            self.start,
            self.end,
            self.brush.thickness.into(),
            self.brush.color,
        );
    }

    fn is_visible(&self, _camera: &Camera) -> bool {
        // always visible since it's in ScreenSpace
        true
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub points: Vec<Point<CanvasSpace>>,
    pub finished: bool,
    pub brush: Brush<CanvasSpace>,
    z: usize,
}

impl Line {
    pub fn new(start: Point<CanvasSpace>, brush: Brush<CanvasSpace>, z: usize) -> Self {
        Self {
            points: vec![start],
            finished: false,
            brush,
            z,
        }
    }

    fn draw_longer(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        assert!(self.points.len() >= 4);

        let pts = std::iter::once(&(self.points[0] * 2.0 - self.points[1]))
            .chain(self.points.iter())
            .map(|p| Vector2::from(p.transform(camera)))
            .collect::<Box<_>>();
        d.draw_spline_catmull_rom(
            &pts,
            self.brush.thickness.transform(camera).into(),
            self.brush.color,
        );
    }

    fn draw_shorter(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let r = f32::from(self.brush.thickness.transform(camera)) / 2.0;
        for seg in self.points.windows(2) {
            let p0 = seg[0].transform(camera);
            let p1 = seg[1].transform(camera);
            d.draw_circle_v(p0, r, self.brush.color);
            d.draw_circle_v(p1, r, self.brush.color);
            d.draw_line_ex(
                p0,
                p1,
                self.brush.thickness.transform(camera).into(),
                self.brush.color,
            );
        }
    }

    fn draw_single(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        assert!(self.points.len() == 1);

        let p = self.points[0].transform(camera);
        let r = self.brush.thickness.transform(camera) / 2.0;

        d.draw_circle_v(p, r.into(), self.brush.color);
    }
}

impl Drawable for Line {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        match self.points.len() {
            0 => {}
            1 => self.draw_single(d, camera),
            2..4 => self.draw_shorter(d, camera),
            4.. => self.draw_longer(d, camera),
        }
    }

    fn is_visible(&self, camera: &Camera) -> bool {
        if self.points.len() == 1 {
            return camera.get_rect().check_collision_point(self.points[0]);
        }

        for seg in self.points.windows(2) {
            if camera.get_rect().check_collision_line(seg[0], seg[1]) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub pos: Point<CanvasSpace>,
    pub texture: Rc<Texture2D>,
    pub is_selected: bool,
    pub scale: Length<CanvasSpace>,
    pub id: ImageId,
    pub z: usize,
    border_color: Color,
}

impl Image {
    pub fn new(
        pos: Point<CanvasSpace>,
        texture: Texture2D,
        scale: Length<CanvasSpace>,
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

    pub fn width(&self) -> Length<CanvasSpace> {
        self.scale * self.texture.width as f32
    }

    pub fn height(&self) -> Length<CanvasSpace> {
        self.scale * self.texture.height as f32
    }

    fn get_rect(&self) -> Rect<CanvasSpace> {
        Rect::new(self.pos, self.width(), self.height())
    }

    pub fn in_bounds(&self, point: Point<CanvasSpace>) -> bool {
        self.get_rect().check_collision_point(point)
    }
}

impl Drawable for Image {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let draw_pos: Vector2 = self.pos.transform(camera).into();
        let draw_size = Vector2::new(
            self.width().transform(camera).into(),
            self.height().transform(camera).into(),
        );

        d.draw_texture_ex(
            &*self.texture,
            draw_pos,
            0.0,
            self.scale.transform(camera).into(),
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

    fn is_visible(&self, camera: &Camera) -> bool {
        camera.get_rect().check_collision(self.get_rect())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Eraser {
    rect: FilledRect,
    z: usize,
}

impl Eraser {
    pub fn new(rect: Rect<CanvasSpace>, color: Color, z: usize) -> Self {
        Self {
            rect: FilledRect::new(rect, color),
            z,
        }
    }
}

impl Drawable for Eraser {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        self.rect.draw(d, camera);
    }

    fn is_visible(&self, camera: &Camera) -> bool {
        self.rect.is_visible(camera)
    }
}
