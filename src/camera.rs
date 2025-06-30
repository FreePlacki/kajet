use tiny_skia::Point;

use crate::{FPS, canvas::Canvas};

pub struct Camera {
    pub pos: Point,
    pub zoom: f32,

    zoom_interp: Interpolator,
    pos_x_interp: Interpolator,
    pos_y_interp: Interpolator,
    mouse: Option<(f32, f32)>,
}

impl Camera {
    pub fn get_bounds(&self, canvas: &Canvas) -> (Point, Point) {
        let w = canvas.width as f32 / self.zoom;
        let h = canvas.height as f32 / self.zoom;

        (self.pos, self.pos + Point::from_xy(w, h))
    }

    pub fn to_canvas_coords(&self, point: Point) -> (i32, i32) {
        let x = ((point.x - self.pos.x) * self.zoom) as i32;
        let y = ((point.y - self.pos.y) * self.zoom) as i32;

        (x, y)
    }

    pub fn to_camera_coords(&self, point: (u32, u32)) -> Point {
        let x = point.0 as f32 / self.zoom + self.pos.x;
        let y = point.1 as f32 / self.zoom + self.pos.y;

        Point::from_xy(x, y)
    }

    pub fn update_zoom(&mut self, target: f32, mouse: Option<(f32, f32)>) {
        self.zoom_interp = Interpolator::new(self.zoom, target);
        self.mouse = mouse;
    }

    pub fn update_pos(&mut self, target: Point) {
        self.pos_x_interp = Interpolator::new(self.pos.x, target.x);
        self.pos_y_interp = Interpolator::new(self.pos.y, target.y);
    }

    pub fn update(&mut self) -> bool {
        let mut updated = false;

        if let Some(dx) = self.pos_x_interp.advance() {
            self.pos.x += dx;
            updated = true;
        }
        if let Some(dy) = self.pos_y_interp.advance() {
            self.pos.y += dy;
            updated = true;
        }

        let prev_zoom = self.zoom;
        if let Some(dz) = self.zoom_interp.advance() {
            self.zoom += dz;
            updated = true;
        }

        if !updated {
            return false;
        }

        if let Some((mouse_x, mouse_y)) = self.mouse {
            self.pos.x -= mouse_x * (1.0 / self.zoom - 1.0 / prev_zoom);
            self.pos.y -= mouse_y * (1.0 / self.zoom - 1.0 / prev_zoom);
        }

        true
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point::from_xy(0.0, 0.0),
            zoom: 1.0,
            zoom_interp: Interpolator::new(1.0, 1.0),
            pos_x_interp: Interpolator::new(0.0, 0.0),
            pos_y_interp: Interpolator::new(0.0, 0.0),
            mouse: None,
        }
    }
}

struct Interpolator {
    starting: f32,
    current: f32,
    target: f32,
    is_increasing: bool,
}

impl Interpolator {
    pub fn new(starting: f32, target: f32) -> Self {
        Self {
            starting,
            current: starting,
            target,
            is_increasing: starting < target,
        }
    }

    pub fn advance(&mut self) -> Option<f32> {
        if self.is_increasing && self.current >= self.target
            || !self.is_increasing && self.current <= self.target
        {
            self.starting = self.target;
            return None;
        }

        let delta = (self.target - self.starting) * 2e-3 * FPS as f32;
        self.current += delta;
        Some(delta)
    }
}
