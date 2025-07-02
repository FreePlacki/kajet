use std::ops::{Add, Mul};

use arboard::ImageData;
use minifb::Window;
use tiny_skia::{IntSize, Pixmap, Point};

use crate::{
    camera::Camera,
    canvas::Canvas,
    config::Config,
    graphics::{Brush, Drawable, FilledCircle, Image, Line},
};

pub struct Scene {
    canvas: Canvas,
    camera: Camera,
    config: Config,
    color_idx: usize,
    brush: Brush,
    lines: Vec<Line>,
    images: Vec<Image>,
    redraw: bool,
    mouse: Option<Point>,
    prev_mouse: Option<Point>,
}

impl Scene {
    pub fn new(width: u32, height: u32, config: Config) -> Self {
        let brush = Brush {
            color: config.colors[0],
            thickness: config.thickness,
        };
        Self {
            canvas: Canvas::new(width, height, config.background),
            camera: Camera::default(),
            config,
            color_idx: 0,
            brush,
            lines: vec![],
            images: vec![],
            redraw: false,
            mouse: None,
            prev_mouse: None,
        }
    }

    pub fn next_frame(&mut self, window_size: (usize, usize), mouse_pos: Option<(f32, f32)>) {
        self.redraw = self
            .canvas
            .set_size((window_size.0 as u32, window_size.1 as u32));
        self.prev_mouse = self.mouse;
        self.mouse = mouse_pos.map(|(x, y)| Point::from_xy(x, y));

        self.canvas.clear_overlay();
    }

    pub fn update_thickness(&mut self, scroll_y: f32) {
        self.brush.thickness = self.brush.thickness.add(scroll_y.signum()).clamp(1.0, 30.0);
    }

    pub fn update_zoom(&mut self, scroll_y: f32) {
        let new_zoom = self
            .camera
            .zoom
            .mul(1.0 + scroll_y.signum() * 0.25 * self.config.scroll_sensitivity)
            .clamp(0.1, 10.0);
        self.camera.update_zoom(new_zoom);
    }

    pub fn update_color(&mut self, forward: bool) {
        self.color_idx = if forward {
            self.color_idx + 1
        } else {
            self.color_idx - 1
        } % self.config.colors.len();

        self.brush.color = self.config.colors[self.color_idx];
    }

    pub fn move_images(&mut self) {
        if let (Some(mouse), Some(prev_mouse)) = (self.mouse, self.prev_mouse) {
            for img in self.images.iter_mut().filter(|i| i.is_selected) {
                img.pos += Point::from_xy(
                    (mouse.x - prev_mouse.x) / self.camera.zoom,
                    (mouse.y - prev_mouse.y) / self.camera.zoom,
                );
                self.redraw = true;
            }
        }
    }

    pub fn on_pen_down(&mut self) {
        if self.mouse.is_none() {
            return;
        }

        // TODO: this makes it so that when moving the image fast it looses focus
        if self.images.iter().any(|i| {
            i.is_selected
                && match self.mouse {
                    Some(m) => i.in_bounds(m, &self.camera),
                    None => false,
                }
        }) {
            self.move_images();
            return;
        }

        let pos = {
            let m = self.mouse.unwrap();
            self.camera.to_camera_coords((m.x as u32, m.y as u32))
        };

        if self.images.iter().any(|i| i.is_selected) {
            self.redraw = true;
            self.images.iter_mut().for_each(|i| i.is_selected = false);
        } else if let Some(line) = self.lines.last_mut() {
            if line.finished {
                self.lines.push(Line::new(pos, self.brush));
            } else if line.points.last().unwrap().distance(pos) >= 5.0 / self.camera.zoom {
                line.points.push(pos);
            }
        } else {
            self.lines.push(Line::new(pos, self.brush));
        }
    }

    pub fn on_pen_up(&mut self) {
        if let Some(line) = self.lines.last_mut() {
            line.finished = true;
        }
    }

    pub fn on_move(&mut self) {
        if let Some(mouse) = self.mouse {
            self.camera.update_pos(mouse, self.prev_mouse);
        }
    }

    pub fn update_cursor(&mut self) {
        if let Some(mouse) = self.mouse {
            FilledCircle {
                pos: mouse,
                brush: Brush {
                    color: self.brush.color,
                    thickness: self.brush.thickness * self.camera.zoom,
                },
            }
            .draw(&mut self.canvas, &self.camera);
        }
        self.camera.end_panning();
    }

    pub fn try_paste_image(&mut self, image_data: ImageData) {
        let img = Pixmap::from_vec(
            image_data.bytes.to_vec(),
            IntSize::from_wh(image_data.width as u32, image_data.height as u32).unwrap(),
        );

        if let Some(img) = img {
            let pos = {
                if let Some(m) = self.mouse {
                    Point::from_xy(
                        m.x - image_data.width as f32 / 2.0 * self.camera.zoom,
                        m.y - image_data.height as f32 / 2.0 * self.camera.zoom,
                    )
                } else {
                    self.camera.pos
                }
            };
            let pos = self.camera.to_camera_coords((pos.x as u32, pos.y as u32));
            let image = Image::new(pos, img, &self.config);
            image.draw(&mut self.canvas, &self.camera);
            self.images.push(image);
        }
    }

    pub fn try_select_image(&mut self) {
        if let Some(mouse) = self.mouse {
            // in reverse + break so that only topmost one gets selected
            for img in self.images.iter_mut().rev() {
                if img.in_bounds(mouse, &self.camera) {
                    img.is_selected = true;
                    self.redraw = true;
                    break;
                }
            }
        }
    }

    pub fn try_delete_images(&mut self) {
        self.images.retain(|i| !i.is_selected);
        self.redraw = true;
    }

    pub fn draw(&mut self, window: &mut Window) {
        self.redraw |= self.camera.update(self.mouse);

        if self.redraw {
            self.canvas.clear();
            for img in &self.images {
                img.draw(&mut self.canvas, &self.camera);
            }
            for line in &self.lines {
                line.draw(&mut self.canvas, &self.camera);
            }
        } else if let Some(line) = self.lines.last() {
            if line.points.len() >= 2 {
                let mut l = Line::new(line.points[line.points.len() - 2], line.brush);
                l.points.push(line.points[line.points.len() - 1]);
                l.draw(&mut self.canvas, &self.camera);
            }
        }

        window
            .update_with_buffer(
                &self.canvas.get_buffer(),
                self.canvas.width as usize,
                self.canvas.height as usize,
            )
            .unwrap();
    }
}
