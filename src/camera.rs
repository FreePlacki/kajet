use tiny_skia::Point;

use crate::FPS;

pub struct Camera {
    pub pos: Point,
    pub zoom: f32,
    zoom_interp: Interpolator,
    pos_x_interp: Interpolator,
    pos_y_interp: Interpolator,
}

impl Camera {
    pub fn to_camera_coords(&self, point: (u32, u32)) -> Point {
        let x = point.0 as f32 / self.zoom + self.pos.x;
        let y = point.1 as f32 / self.zoom + self.pos.y;

        Point::from_xy(x, y)
    }

    pub fn update_zoom(&mut self, target: f32) {
        self.zoom_interp = Interpolator::new(self.zoom, target, 0.1);
    }

    pub fn update_pos(&mut self, mouse: Point, prev_mouse: Option<Point>) {
        if let Some(prev_mouse) = prev_mouse {
            let mut diff = prev_mouse - mouse;
            diff.x /= self.zoom;
            diff.y /= self.zoom;
            let new_pos = self.pos + diff;
            self.pos_x_interp = Interpolator::new(self.pos.x, new_pos.x, 0.0);
            self.pos_y_interp = Interpolator::new(self.pos.y, new_pos.y, 0.0);
        }
    }

    pub fn update(&mut self, mouse: Option<Point>) -> bool {
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

            if let Some(Point { x, y }) = mouse {
                self.pos.x -= x * (1.0 / self.zoom - 1.0 / prev_zoom);
                self.pos.y -= y * (1.0 / self.zoom - 1.0 / prev_zoom);
                self.pos_x_interp = Interpolator::new(self.pos.x, self.pos.x, 0.0);
                self.pos_y_interp = Interpolator::new(self.pos.y, self.pos.y, 0.0);
            }
            updated = true;
        }

        if !updated {
            return false;
        }

        true
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point::from_xy(0.0, 0.0),
            zoom: 1.0,
            zoom_interp: Interpolator::new(1.0, 1.0, 0.0),
            pos_x_interp: Interpolator::new(0.0, 0.0, 0.0),
            pos_y_interp: Interpolator::new(0.0, 0.0, 0.0),
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

    pub fn advance(&mut self) -> Option<f32> {
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
            (self.target - self.starting) / (self.duration_sec * FPS as f32)
        };
        self.current += delta;
        Some(delta)
    }
}
