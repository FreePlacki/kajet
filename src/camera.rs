use crate::graphics::Point;

#[derive(Debug)]
pub struct Camera {
    pub pos: Point,
    pub zoom: f32,
}

impl Camera {
    pub fn to_canvas_coords(&self, point: Point) -> (i32, i32) {
        let x = (point.x * self.zoom + self.pos.x) as i32;
        let y = (point.y * self.zoom + self.pos.y) as i32;

        (x, y)
    }

    pub fn to_camera_coords(&self, point: (u32, u32)) -> Point {
        let x = self.pos.x + point.0 as f32 / self.zoom;
        let y = self.pos.y + point.1 as f32 / self.zoom;

        Point::new(x, y)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: Point::new(0.0, 0.0),
            zoom: 1.0,
        }
    }
}
