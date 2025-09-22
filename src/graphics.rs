use std::rc::Rc;

use raylib::{
    color::Color,
    math::Vector2,
    prelude::{RaylibDraw, RaylibDrawHandle},
    texture::Texture2D,
};
use widok::{
    Bounds, Camera, CanvasBox, CanvasPoint, CanvasRect, CanvasSpace, CanvasVector, InView, Length,
    Rect, ScreenPoint, ScreenSize, ScreenSpace, ToScreen,
};

use crate::config::Config;

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

pub trait Drawable: InView {
    fn z(&self) -> usize;
    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera);
}

#[derive(Debug, Clone, Copy)]
pub struct FilledCircle {
    pub pos: ScreenPoint,
    pub brush: Brush<ScreenSpace>,
}

impl InView for FilledCircle {
    fn is_in_view(&self, _camera: &Camera) -> bool {
        // always visible since it's in ScreenSpace
        true
    }
}

trait ToVector2 {
    fn to_vec2(self) -> raylib::ffi::Vector2;
}

impl ToVector2 for ScreenPoint {
    fn to_vec2(self) -> raylib::ffi::Vector2 {
        raylib::ffi::Vector2 {
            x: self.x,
            y: self.y,
        }
    }
}

impl ToVector2 for ScreenSize {
    fn to_vec2(self) -> raylib::ffi::Vector2 {
        raylib::ffi::Vector2 {
            x: self.width,
            y: self.height,
        }
    }
}

impl Drawable for FilledCircle {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, _: &Camera) {
        let r = self.brush.thickness / 2.0;
        let r = r.0.max(1.0);

        d.draw_circle_v(self.pos.to_vec2(), r, self.brush.color);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FilledRect {
    pub rect: CanvasRect,
    pub color: Color,
}

impl FilledRect {
    pub fn new(rect: Rect<CanvasSpace>, color: Color) -> Self {
        Self { rect, color }
    }

    fn normalized_rect(&self) -> CanvasRect {
        let mut origin = self.rect.origin;
        let mut size = self.rect.size;

        if size.width < 0.0 {
            origin.x += size.width;
            size.width = -size.width;
        }
        if size.height < 0.0 {
            origin.y += size.height;
            size.height = -size.height;
        }

        CanvasRect::new(origin, size)
    }
}

impl Bounds for FilledRect {
    fn bounds(&self) -> CanvasBox {
        self.rect.to_box2d()
    }
}

impl Drawable for FilledRect {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let rect = self.normalized_rect().to_screen(camera);
        d.draw_rectangle_v(rect.min().to_vec2(), rect.size.to_vec2(), self.color);
    }
}

#[derive(Debug)]
pub struct StraightLine {
    pub start: ScreenPoint,
    pub end: ScreenPoint,
    pub brush: Brush<ScreenSpace>,
}

impl InView for StraightLine {
    fn is_in_view(&self, _camera: &Camera) -> bool {
        // always visible since it's in ScreenSpace
        true
    }
}

impl Drawable for StraightLine {
    fn z(&self) -> usize {
        0
    }

    fn draw(&self, d: &mut RaylibDrawHandle, _camera: &Camera) {
        d.draw_line_ex(
            self.start.to_vec2(),
            self.end.to_vec2(),
            self.brush.thickness.0,
            self.brush.color,
        );
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub points: Vec<CanvasPoint>,
    pub finished: bool,
    pub brush: Brush<CanvasSpace>,
    z: usize,
}

impl Line {
    pub fn new(start: CanvasPoint, brush: Brush<CanvasSpace>, z: usize) -> Self {
        Self {
            points: vec![start],
            finished: false,
            brush,
            z,
        }
    }

    fn draw_longer(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        assert!(self.points.len() >= 4);

        let first = (self.points[0] * 2.0 - self.points[1]).to_point();
        let pts = std::iter::once(&first)
            .chain(self.points.iter())
            .map(|p| Vector2::from(p.to_screen(camera).to_vec2()))
            .collect::<Box<_>>();
        d.draw_spline_catmull_rom(
            &pts,
            self.brush.thickness.to_screen(camera).0,
            self.brush.color,
        );
    }

    fn draw_shorter(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let r = self.brush.thickness.to_screen(camera) / 2.0;
        for seg in self.points.windows(2) {
            let p0 = seg[0].to_screen(camera);
            let p1 = seg[1].to_screen(camera);
            d.draw_circle_v(p0.to_vec2(), r.0, self.brush.color);
            d.draw_circle_v(p1.to_vec2(), r.0, self.brush.color);
            d.draw_line_ex(
                p0.to_vec2(),
                p1.to_vec2(),
                self.brush.thickness.to_screen(camera).0,
                self.brush.color,
            );
        }
    }

    fn draw_single(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        assert!(self.points.len() == 1);

        let p = self.points[0].to_screen(camera);
        let r = self.brush.thickness.to_screen(camera) / 2.0;

        d.draw_circle_v(p.to_vec2(), r.0, self.brush.color);
    }
}

impl Bounds for Line {
    fn bounds(&self) -> CanvasBox {
        debug_assert!(!self.points.is_empty());

        let r = self.brush.thickness.0;

        let min_x = self
            .points
            .iter()
            .map(|a| a.x)
            .min_by(f32::total_cmp)
            .unwrap()
            - r;
        let min_y = self
            .points
            .iter()
            .map(|a| a.y)
            .min_by(f32::total_cmp)
            .unwrap()
            - r;
        let max_x = self
            .points
            .iter()
            .map(|a| a.x)
            .max_by(f32::total_cmp)
            .unwrap()
            + r;
        let max_y = self
            .points
            .iter()
            .map(|a| a.y)
            .max_by(f32::total_cmp)
            .unwrap()
            + r;

        CanvasBox::new(
            CanvasPoint::new(min_x, min_y),
            CanvasPoint::new(max_x, max_y),
        )
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
}

#[derive(Debug, Clone)]
pub struct Image {
    pub pos: CanvasPoint,
    pub texture: Rc<Texture2D>,
    pub is_selected: bool,
    pub scale: Length<CanvasSpace>,
    pub id: ImageId,
    pub z: usize,
    border_color: Color,
}

impl Image {
    pub fn new(
        pos: CanvasPoint,
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

    pub fn in_bounds(&self, point: CanvasPoint) -> bool {
        self.bounds().contains(point)
    }
}

impl Bounds for Image {
    fn bounds(&self) -> CanvasBox {
        CanvasBox::new(
            self.pos,
            self.pos + CanvasVector::new(self.width().0, self.height().0),
        )
    }
}

impl Drawable for Image {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        let rect = self.bounds().to_screen(camera).to_rect();

        d.draw_texture_ex(
            &*self.texture,
            rect.min().to_vec2(),
            0.0,
            self.scale.to_screen(camera).0,
            Color::WHITE,
        );

        if self.is_selected {
            d.draw_rectangle_lines(
                rect.min().x as i32,
                rect.min().y as i32,
                rect.size.width as i32,
                rect.size.height as i32,
                self.border_color,
            );
        }
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

impl Bounds for Eraser {
    fn bounds(&self) -> CanvasBox {
        self.rect.bounds()
    }
}

impl Drawable for Eraser {
    fn z(&self) -> usize {
        self.z
    }

    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera) {
        self.rect.draw(d, camera);
    }
}
