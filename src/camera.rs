use raylib::{RaylibHandle, math::Vector2};

use crate::units::{CanvasSpace, Length, Point, Rect, ScreenSpace, Transformable, Vector};

pub struct Camera {
    pub zoom: f32,
    pub pos: Point<CanvasSpace>,
    width: Length<CanvasSpace>,
    height: Length<CanvasSpace>,
    zoom_interp: Interpolator,
}

impl Camera {
    pub fn update_zoom(&mut self, target: f32) {
        self.zoom_interp = Interpolator::new(self.zoom, target, 0.07);
    }

    pub fn update_pos(&mut self, mouse_delta: Vector<ScreenSpace>) {
        let new_pos = self.pos - mouse_delta.transform(self);
        self.pos = new_pos;
    }

    pub fn update(&mut self, rl: &RaylibHandle) -> bool {
        let mut updated = false;
        let dt = rl.get_frame_time();
        let mouse = Point::<ScreenSpace>::new(rl.get_mouse_position());
        self.width = Length::<ScreenSpace>::new(rl.get_render_width() as f32).transform(self);
        self.height = Length::<ScreenSpace>::new(rl.get_render_height() as f32).transform(self);

        let prev_zoom = self.zoom;
        if let Some(dz) = self.zoom_interp.advance(dt) {
            self.zoom += dz;

            unsafe {
                *self.pos.v_mut() = self.pos.v() - mouse.v() * (1.0 / self.zoom - 1.0 / prev_zoom);
            }
            updated = true;
        }

        if !updated {
            return false;
        }

        true
    }

    pub fn get_rect(&self) -> Rect<CanvasSpace> {
        Rect::new(self.pos, self.width, self.height)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point::new(Vector2::default()),
            zoom: 1.0,
            width: Length::new(0.0),
            height: Length::new(0.0),
            zoom_interp: Interpolator::new(1.0, 1.0, 0.0),
        }
    }
}

struct Interpolator {
    starting: f32,
    current: f32,
    target: f32,
    duration_sec: f32,
    is_increasing: bool,
}

impl Interpolator {
    pub fn new(starting: f32, target: f32, duration_sec: f32) -> Self {
        Self {
            starting,
            current: starting,
            target,
            duration_sec,
            is_increasing: starting < target,
        }
    }

    pub fn advance(&mut self, dt: f32) -> Option<f32> {
        let tol = 1e-3;
        if self.is_increasing && self.current + tol >= self.target
            || !self.is_increasing && self.current <= self.target + tol
        {
            self.starting = self.target;
            return None;
        }

        let delta = if self.duration_sec == 0.0 {
            self.target - self.starting
        } else {
            (self.target - self.starting) / self.duration_sec * dt
        };
        self.current += delta;
        Some(delta)
    }
}
