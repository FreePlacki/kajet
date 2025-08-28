use raylib::{
    RaylibHandle,
    math::{Rectangle, Vector2},
};

pub struct Camera {
    pub zoom: f32,
    pos: Vector2,
    width: f32,
    height: f32,
    zoom_interp: Interpolator,
    pos_x_interp: Interpolator,
    pos_y_interp: Interpolator,
}

impl Camera {
    pub fn update_zoom(&mut self, target: f32) {
        self.zoom_interp = Interpolator::new(self.zoom, target, 0.07);
    }

    pub fn update_pos(&mut self, mouse_delta: Vector2) {
        let diff = mouse_delta / self.zoom;
        let new_pos = self.pos - diff;
        self.pos_x_interp = Interpolator::new(self.pos.x, new_pos.x, 0.0);
        self.pos_y_interp = Interpolator::new(self.pos.y, new_pos.y, 0.0);
    }

    pub fn update(&mut self, rl: &RaylibHandle) -> bool {
        let mut updated = false;
        let dt = rl.get_frame_time();
        let mouse = rl.get_mouse_position();
        self.width = (rl.get_render_width() as f32).to_canvas_coords(self);
        self.height = (rl.get_render_height() as f32).to_canvas_coords(self);

        if let Some(dx) = self.pos_x_interp.advance(dt) {
            self.pos.x += dx;
            updated = true;
        }
        if let Some(dy) = self.pos_y_interp.advance(dt) {
            self.pos.y += dy;
            updated = true;
        }

        let prev_zoom = self.zoom;
        if let Some(dz) = self.zoom_interp.advance(dt) {
            self.zoom += dz;

            self.pos.x -= mouse.x * (1.0 / self.zoom - 1.0 / prev_zoom);
            self.pos.y -= mouse.y * (1.0 / self.zoom - 1.0 / prev_zoom);
            self.pos_x_interp = Interpolator::new(self.pos.x, self.pos.x, 0.0);
            self.pos_y_interp = Interpolator::new(self.pos.y, self.pos.y, 0.0);
            updated = true;
        }

        if !updated {
            return false;
        }

        true
    }

    pub fn get_rect(&self) -> Rectangle {
        Rectangle {
            x: self.pos.x,
            y: self.pos.y,
            width: self.width,
            height: self.height,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Vector2::zero(),
            zoom: 1.0,
            width: 0.0,
            height: 0.0,
            zoom_interp: Interpolator::new(1.0, 1.0, 0.0),
            pos_x_interp: Interpolator::new(0.0, 0.0, 0.0),
            pos_y_interp: Interpolator::new(0.0, 0.0, 0.0),
        }
    }
}

pub trait CameraCanvasCoords {
    fn to_camera_coords(self, camera: &Camera) -> Self;
    fn to_canvas_coords(self, camera: &Camera) -> Self;
}

impl CameraCanvasCoords for f32 {
    fn to_camera_coords(self, camera: &Camera) -> Self {
        self * camera.zoom
    }

    fn to_canvas_coords(self, camera: &Camera) -> Self {
        self / camera.zoom
    }
}

impl CameraCanvasCoords for Vector2 {
    fn to_camera_coords(self, camera: &Camera) -> Self {
        (self - camera.pos) * camera.zoom
    }

    fn to_canvas_coords(self, camera: &Camera) -> Self {
        self / camera.zoom + camera.pos
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
