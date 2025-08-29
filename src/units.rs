use core::f32;
use std::{marker::PhantomData, ops};

use raylib::math::{Rectangle, Vector2};
use raylib::{collision, ffi};

use crate::camera::Camera;

#[derive(Debug, Clone, Copy)]
pub struct ScreenSpace;
#[derive(Debug, Clone, Copy)]
pub struct CanvasSpace;

#[derive(Debug, Clone, Copy)]
pub struct Length<Space> {
    v: f32,
    _marker: PhantomData<Space>,
}

impl<Space> Length<Space> {
    pub fn new(v: f32) -> Self {
        Self {
            v,
            _marker: PhantomData,
        }
    }

    /// unsafe because it's recommended to use transforms and overloaded operators
    pub unsafe fn v(&self) -> f32 {
        self.v
    }

    /// unsafe because it's recommended to use transforms and overloaded operators
    pub unsafe fn v_mut(&mut self) -> &mut f32 {
        &mut self.v
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point<Space> {
    v: Vector2,
    _marker: PhantomData<Space>,
}

impl<Space> Point<Space> {
    pub fn new(v: Vector2) -> Self {
        Self {
            v,
            _marker: PhantomData,
        }
    }

    pub fn x(&self) -> Length<Space> {
        Length::new(self.v.x)
    }

    pub fn y(&self) -> Length<Space> {
        Length::new(self.v.y)
    }

    /// unsafe because it's recommended to use transforms and overloaded operators
    pub unsafe fn v(&self) -> Vector2 {
        self.v
    }

    /// unsafe because it's recommended to use transforms and overloaded operators
    pub unsafe fn v_mut(&mut self) -> &mut Vector2 {
        &mut self.v
    }

    fn with_x(&self, x: f32) -> Self {
        Self {
            v: Vector2::new(x, self.v.y),
            _marker: PhantomData,
        }
    }

    fn with_y(&self, y: f32) -> Self {
        Self {
            v: Vector2::new(self.v.x, y),
            _marker: PhantomData,
        }
    }

    pub fn distance_to(&self, other: &Self) -> Length<Space> {
        Length::new(self.v.distance_to(other.v))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vector<Space> {
    v: Vector2,
    _marker: PhantomData<Space>,
}

impl<Space> Vector<Space> {
    pub fn new(v: Vector2) -> Self {
        Self {
            v,
            _marker: PhantomData,
        }
    }

    pub fn x(&self) -> Length<Space> {
        Length::new(self.v.x)
    }

    pub fn y(&self) -> Length<Space> {
        Length::new(self.v.y)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect<Space> {
    pos: Point<Space>,
    w: Length<Space>,
    h: Length<Space>,
}

impl<Space: Copy> Rect<Space> {
    pub fn new(pos: Point<Space>, w: Length<Space>, h: Length<Space>) -> Self {
        Self { pos, w, h }
    }

    /// If the rectangle has a negative width or height then this returns a
    /// normalized version with positive dimensions by changing the position.
    pub fn normalized(self) -> Self {
        let Vector2 { mut x, mut y } = self.pos.v;
        let mut w = self.w.v;
        let mut h = self.h.v;

        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }

        Self::new(
            Point::new(Vector2::new(x, y)),
            Length::new(w),
            Length::new(h),
        )
    }

    pub fn w(self) -> Length<Space> {
        self.w
    }

    pub fn w_mut(&mut self) -> &mut Length<Space> {
        &mut self.w
    }

    pub fn h(self) -> Length<Space> {
        self.h
    }

    pub fn h_mut(&mut self) -> &mut Length<Space> {
        &mut self.h
    }

    fn to_raylib(self) -> Rectangle {
        Rectangle {
            x: self.pos.v.x,
            y: self.pos.v.y,
            width: self.w.v,
            height: self.h.v,
        }
    }

    pub fn check_collision(&self, other: Self) -> bool {
        self.to_raylib().check_collision_recs(&other.to_raylib())
    }

    pub fn check_collision_circle(&self, pos: Point<Space>, r: Length<Space>) -> bool {
        self.to_raylib().check_collision_circle_rec(pos.v, r.v)
    }

    pub fn check_collision_point(&self, point: Point<Space>) -> bool {
        self.to_raylib().check_collision_point_rec(point.v)
    }

    pub fn check_collision_line(&self, start: Point<Space>, end: Point<Space>) -> bool {
        let edges = [
            (self.pos, self.pos.with_x(self.pos.v.x + self.w.v)), // top
            (self.pos, self.pos.with_y(self.pos.v.y + self.h.v)), // left
            (
                self.pos.with_y(self.pos.v.y + self.h.v),
                self.pos.with_x(self.pos.v.x + self.w.v),
            ), // bottom
            (
                self.pos.with_x(self.pos.v.x + self.w.v),
                self.pos.with_y(self.pos.v.y + self.h.v),
            ), // right
        ];

        if self.check_collision_point(start) || self.check_collision_point(end) {
            return true;
        }

        edges
            .into_iter()
            .any(|(es, ee)| collision::check_collision_lines(es.v, ee.v, start.v, end.v).is_some())
    }
}

impl Rect<ScreenSpace> {
    pub fn pos(&self) -> ffi::Vector2 {
        self.pos.v.into()
    }

    pub fn size(&self) -> ffi::Vector2 {
        ffi::Vector2 {
            x: self.w.v,
            y: self.h.v,
        }
    }
}

pub trait Transformable<FromSpace, ToSpace> {
    type Output;

    fn transform(self, camera: &Camera) -> Self::Output;
}

impl Transformable<CanvasSpace, ScreenSpace> for Length<CanvasSpace> {
    type Output = Length<ScreenSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Length::new(self.v * camera.zoom)
    }
}

impl Transformable<ScreenSpace, CanvasSpace> for Length<ScreenSpace> {
    type Output = Length<CanvasSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Length::new(self.v / camera.zoom)
    }
}

impl Transformable<CanvasSpace, ScreenSpace> for Point<CanvasSpace> {
    type Output = Point<ScreenSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Point::new((self.v - camera.pos.v) * camera.zoom)
    }
}

impl Transformable<ScreenSpace, CanvasSpace> for Point<ScreenSpace> {
    type Output = Point<CanvasSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Point::new(self.v / camera.zoom + camera.pos.v)
    }
}

impl Transformable<CanvasSpace, ScreenSpace> for Vector<CanvasSpace> {
    type Output = Vector<ScreenSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Vector::new(self.v * camera.zoom)
    }
}

impl Transformable<ScreenSpace, CanvasSpace> for Vector<ScreenSpace> {
    type Output = Vector<CanvasSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Vector::new(self.v / camera.zoom)
    }
}

impl Transformable<CanvasSpace, ScreenSpace> for Rect<CanvasSpace> {
    type Output = Rect<ScreenSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Rect::new(
            self.pos.transform(camera),
            self.w.transform(camera),
            self.h.transform(camera),
        )
    }
}

impl Transformable<ScreenSpace, CanvasSpace> for Rect<ScreenSpace> {
    type Output = Rect<CanvasSpace>;

    fn transform(self, camera: &Camera) -> Self::Output {
        Rect::new(
            self.pos.transform(camera),
            self.w.transform(camera),
            self.h.transform(camera),
        )
    }
}

// IMPORTANT: only conversion from ScreenSpace is implemented so that it's the
// one used when interacting with raylib

impl From<Length<ScreenSpace>> for f32 {
    fn from(value: Length<ScreenSpace>) -> Self {
        value.v
    }
}

impl From<Point<ScreenSpace>> for ffi::Vector2 {
    fn from(value: Point<ScreenSpace>) -> Self {
        value.v.into()
    }
}

impl From<Point<ScreenSpace>> for Vector2 {
    fn from(value: Point<ScreenSpace>) -> Self {
        value.v
    }
}

impl From<Rect<ScreenSpace>> for ffi::Rectangle {
    fn from(value: Rect<ScreenSpace>) -> Self {
        Rectangle {
            x: value.pos.v.x,
            y: value.pos.v.y,
            width: value.w.v,
            height: value.h.v,
        }.into()
    }
}

impl From<Rect<ScreenSpace>> for Rectangle {
    fn from(value: Rect<ScreenSpace>) -> Self {
        Rectangle {
            x: value.pos.v.x,
            y: value.pos.v.y,
            width: value.w.v,
            height: value.h.v,
        }
    }
}

impl<Space> ops::AddAssign<Length<Space>> for Length<Space> {
    fn add_assign(&mut self, rhs: Length<Space>) {
        self.v += rhs.v;
    }
}

impl<Space> ops::Mul<f32> for Length<Space> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Length::new(self.v * rhs)
    }
}

impl<Space> ops::MulAssign<f32> for Length<Space> {
    fn mul_assign(&mut self, rhs: f32) {
        self.v *= rhs;
    }
}

impl<Space> ops::Div<f32> for Length<Space> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Length::new(self.v / rhs)
    }
}

impl<Space> ops::AddAssign<Vector<Space>> for Point<Space> {
    fn add_assign(&mut self, rhs: Vector<Space>) {
        self.v += rhs.v
    }
}

impl<Space> ops::Sub<Vector<Space>> for Point<Space> {
    type Output = Self;

    fn sub(self, rhs: Vector<Space>) -> Self::Output {
        Point::new(self.v - rhs.v)
    }
}

impl<Space> ops::SubAssign<Vector<Space>> for Point<Space> {
    fn sub_assign(&mut self, rhs: Vector<Space>) {
        self.v -= rhs.v
    }
}

impl<Space> ops::Mul<f32> for Point<Space> {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Point::new(self.v * rhs)
    }
}

impl<Space> ops::Div<f32> for Point<Space> {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Point::new(self.v / rhs)
    }
}

impl<Space> ops::Sub<Point<Space>> for Point<Space> {
    type Output = Self;

    fn sub(self, rhs: Point<Space>) -> Self::Output {
        Point::new(self.v - rhs.v)
    }
}
