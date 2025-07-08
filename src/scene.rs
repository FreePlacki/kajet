use std::ops::{Add, Mul};

use arboard::ImageData;
use minifb::Window;
use tiny_skia::{IntSize, Pixmap, Point};

use crate::{
    camera::Camera,
    canvas::Canvas,
    command::{AddEraser, CommandInvoker, DrawLine, PasteImage, RemoveImage, ResizeImage},
    config::Config,
    graphics::{Brush, Drawable, Eraser, FilledCircle, FilledRect, Image, Line, StraightLine},
};

pub struct Contents {
    pub lines: Vec<Line>,
    pub images: Vec<Image>,
    pub erasers: Vec<Eraser>,
    pub z: usize,
    next_image_id: usize,
}

impl Contents {
    pub fn new() -> Self {
        Self {
            lines: vec![],
            images: vec![],
            erasers: vec![],
            z: 0,
            next_image_id: 0,
        }
    }

    pub fn next_image_id(&mut self) -> usize {
        self.next_image_id += 1;
        self.next_image_id - 1
    }
}

pub struct Scene {
    canvas: Canvas,
    camera: Camera,
    config: Config,
    color_idx: usize,
    brush: Brush,
    contents: Contents,
    redraw: bool,
    mouse: Option<Point>,
    prev_mouse: Option<Point>,
    active_eraser: Option<FilledRect>,
    resizing_image: Option<(usize, f32)>,
    command_invoker: CommandInvoker,
}

impl Scene {
    pub fn new(width: u32, height: u32, config: Config) -> Self {
        let brush = Brush {
            color: config.colors[0],
            thickness: config.thickness,
        };
        let command_invoker = CommandInvoker::new(config.undo_buffer_size);
        Self {
            canvas: Canvas::new(width, height, config.background),
            camera: Camera::default(),
            config,
            color_idx: 0,
            brush,
            contents: Contents::new(),
            redraw: false,
            mouse: None,
            prev_mouse: None,
            active_eraser: None,
            resizing_image: None,
            command_invoker,
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
            for img in self.contents.images.iter_mut().filter(|i| i.is_selected) {
                img.pos += Point::from_xy(
                    (mouse.x - prev_mouse.x) / self.camera.zoom,
                    (mouse.y - prev_mouse.y) / self.camera.zoom,
                );
                self.redraw = true;
            }
        }
    }

    pub fn on_pen_down(&mut self, shift_down: bool) {
        if self.mouse.is_none() {
            return;
        }

        // TODO: this makes it so that when moving the image fast it looses focus
        if self.contents.images.iter().any(|i| {
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

        if self.contents.images.iter().any(|i| i.is_selected) {
            self.redraw = true;
            self.contents
                .images
                .iter_mut()
                .for_each(|i| i.is_selected = false);
        } else if let Some(line) = self.contents.lines.last_mut() {
            if line.finished {
                self.contents
                    .lines
                    .push(Line::new(pos, self.brush, self.contents.z));
            } else if !shift_down
                && line.points.last().unwrap().distance(pos) >= 5.0 / self.camera.zoom
            {
                line.points.push(pos);
            }
        } else {
            self.contents
                .lines
                .push(Line::new(pos, self.brush, self.contents.z));
        }
    }

    pub fn on_pen_up(&mut self) {
        if let Some(line) = self.contents.lines.last_mut() {
            if !line.finished {
                line.finished = true;
                // to draw the rest when holding shift
                if let Some(mouse) = self.mouse {
                    line.points.push(
                        self.camera
                            .to_camera_coords((mouse.x as u32, mouse.y as u32)),
                    );
                    self.redraw = true;
                }
                let cmd = DrawLine::new(self.contents.lines.last().unwrap().clone());
                self.command_invoker.push(cmd);
            }
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

            if let Some(line) = self.contents.lines.last() {
                if !line.finished {
                    StraightLine {
                        start: *line.points.last().unwrap(),
                        end: self
                            .camera
                            .to_camera_coords((mouse.x as u32, mouse.y as u32)),
                        brush: self.brush,
                    }
                    .draw(&mut self.canvas, &self.camera);
                }
            }
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
                        m.x - image_data.width as f32 / 2.0,
                        m.y - image_data.height as f32 / 2.0,
                    )
                } else {
                    self.camera.pos
                }
            };
            let pos = self.camera.to_camera_coords((pos.x as u32, pos.y as u32));
            self.contents.z += 1;
            let image = Image::new(
                pos,
                img,
                1.0 / self.camera.zoom,
                self.contents.next_image_id(),
                self.contents.z,
                &self.config,
            );
            image.draw(&mut self.canvas, &self.camera);
            self.contents.images.push(image.clone());
            self.command_invoker.push(PasteImage::new(image));
        }
    }

    pub fn try_select_image(&mut self) {
        if let Some(mouse) = self.mouse {
            // in reverse + break so that only topmost one gets selected
            for img in self.contents.images.iter_mut().rev() {
                if img.in_bounds(mouse, &self.camera) {
                    if img.is_selected {
                        if self.resizing_image.is_none() || self.resizing_image.unwrap().0 != img.id
                        {
                            self.resizing_image = Some((img.id, img.scale));
                        }

                        if let Some(pm) = self.prev_mouse {
                            let d = mouse - pm;
                            let sx = d.x / img.width();
                            let sy = d.y / img.height();
                            img.scale *= 1.0 + if sx.abs() > sy.abs() { sx } else { sy };
                        }
                    } else {
                        img.is_selected = true;
                    }
                    self.redraw = true;
                    break;
                }
            }
        }
    }

    pub fn end_resizing_image(&mut self) {
        if let Some((id, scale)) = self.resizing_image {
            let mut img = None;
            for i in &self.contents.images {
                if i.id == id {
                    img = Some(i);
                    break;
                }
            }
            if let Some(img) = img {
                self.command_invoker
                    .push(ResizeImage::new(img.clone(), scale, img.scale));
            }
            self.resizing_image = None;
        }
    }

    pub fn try_remove_images(&mut self) {
        self.contents.images.retain(|i| {
            if i.is_selected {
                self.command_invoker.push(RemoveImage::new(i.clone()));
                false
            } else {
                true
            }
        });
        self.redraw = true;
    }

    pub fn on_erase(&mut self) {
        if let (Some(m), Some(pm)) = (self.mouse, self.prev_mouse) {
            if let Some(eraser) = &mut self.active_eraser {
                eraser.width += m.x - pm.x;
                eraser.height += m.y - pm.y;
            } else {
                self.active_eraser = Some(FilledRect {
                    pos: m,
                    width: 0.0,
                    height: 0.0,
                    color: self.config.background,
                })
            }
        }
    }

    pub fn on_erase_end(&mut self) {
        if let Some(eraser) = &self.active_eraser {
            let pos = self
                .camera
                .to_camera_coords((eraser.pos.x as u32, eraser.pos.y as u32));
            let rect = FilledRect {
                pos,
                width: eraser.width / self.camera.zoom,
                height: eraser.height / self.camera.zoom,
                color: eraser.color,
            }
            .to_skia();

            self.contents
                .erasers
                .push(Eraser::new(rect, eraser.color, self.contents.z));
            self.contents.z += 1;
            self.active_eraser = None;
            self.redraw = true;
            self.command_invoker.push(AddEraser::new(
                self.contents.erasers.last().unwrap().clone(),
            ));
        }
    }

    pub fn undo(&mut self) {
        self.command_invoker.undo(&mut self.contents);
        self.redraw = true;
    }

    pub fn redo(&mut self) {
        self.command_invoker.redo(&mut self.contents);
        self.redraw = true;
    }

    pub fn draw(&mut self, window: &mut Window) {
        self.redraw |= self.camera.update(self.mouse);

        if self.redraw {
            self.canvas.clear();

            let mut combined = Vec::<&dyn Drawable>::new();
            combined.extend(self.contents.images.iter().map(|i| i as &dyn Drawable));
            combined.extend(self.contents.lines.iter().map(|i| i as &dyn Drawable));
            combined.extend(self.contents.erasers.iter().map(|i| i as &dyn Drawable));
            combined.sort_by_key(|i| i.z());
            combined
                .iter()
                .for_each(|i| i.draw(&mut self.canvas, &self.camera));
        } else if let Some(line) = self.contents.lines.last() {
            if line.points.len() >= 2 && !line.finished {
                let mut l = Line::new(line.points[line.points.len() - 2], line.brush, line.z());
                l.points.push(line.points[line.points.len() - 1]);
                l.draw(&mut self.canvas, &self.camera);
            }
            if let Some(eraser) = &self.active_eraser {
                eraser.draw(&mut self.canvas, &self.camera);
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
