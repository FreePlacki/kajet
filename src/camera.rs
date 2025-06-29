use crate::{FPS, graphics::Point};

pub struct Camera {
    pub pos: Point,
    pub zoom: f32,

    zoom_interp: Interpolator,
    pos_x_interp: Interpolator,
    pos_y_interp: Interpolator,
    mouse: Option<(f32, f32)>,
}

impl Camera {
    pub fn to_canvas_coords(&self, point: Point) -> (i32, i32) {
        let x = ((point.x - self.pos.x) * self.zoom) as i32;
        let y = ((point.y - self.pos.y) * self.zoom) as i32;

        (x, y)
    }

    pub fn to_camera_coords(&self, point: (u32, u32)) -> Point {
        let x = point.0 as f32 / self.zoom + self.pos.x;
        let y = point.1 as f32 / self.zoom + self.pos.y;

        Point::new(x, y)
    }

    pub fn update_zoom(&mut self, target: f32, mouse: Option<(f32, f32)>) {
        self.zoom_interp = Interpolator::new(self.zoom, target);
        self.mouse = mouse;
    }

    pub fn update_pos(&mut self, target: Point) {
        self.pos_x_interp = Interpolator::new(self.pos.x, target.x);
        self.pos_y_interp = Interpolator::new(self.pos.y, target.y);
    }

    pub fn update(&mut self) {
        self.pos.x += self.pos_x_interp.advance();
        self.pos.y += self.pos_y_interp.advance();

        let prev_zoom = self.zoom;
        self.zoom += self.zoom_interp.advance();

        if let Some((mouse_x, mouse_y)) = self.mouse {
            self.pos.x -= mouse_x * (1.0 / self.zoom - 1.0 / prev_zoom);
            self.pos.y -= mouse_y * (1.0 / self.zoom - 1.0 / prev_zoom);
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point::new(0.0, 0.0),
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

    pub fn advance(&mut self) -> f32 {
        if self.is_increasing && self.current >= self.target
            || !self.is_increasing && self.current <= self.target
        {
            self.starting = self.target;
            return 0.0;
        }

        let delta = (self.target - self.starting) * 2e-3 * FPS as f32;
        self.current += delta;
        delta
    }
}
