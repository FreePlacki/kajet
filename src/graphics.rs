use crate::canvas::Canvas;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Point {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    pub color: u32,
    pub thickness: u32,
}

pub trait Drawable {
    fn draw(&self, canvas: &mut Canvas, brush: Brush);
}

pub struct FilledRect {
    pub pos: Point,
}

impl Drawable for FilledRect {
    fn draw(&self, canvas: &mut Canvas, brush: Brush) {
        for y in self.pos.y..(self.pos.y + brush.thickness) {
            for x in self.pos.x..(self.pos.x + brush.thickness) {
                let p = Point::new(x, y);
                if !canvas.in_bounds(p) {
                    break;
                }
                canvas[p] = brush.color;
            }
        }
    }
}

pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Drawable for Line {
    fn draw(&self, canvas: &mut Canvas, brush: Brush) {
        let mut x0 = self.start.x as i32;
        let mut y0 = self.start.y as i32;
        let x1 = self.end.x as i32;
        let y1 = self.end.y as i32;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let half = brush.thickness as i32 / 2;

        loop {
            FilledRect {
                pos: Point {
                    x: x0 as u32,
                    y: y0 as u32,
                },
            }
            .draw(canvas, brush);

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
