use arboard::ImageData;
use minifb::{CursorStyle, Key, MouseButton, Window};
use tiny_skia::{IntSize, Pixmap, Point};

use crate::{
    command::{self, AddEraser, DrawLine},
    graphics::{
        Brush, Drawable, Eraser, FilledCircle, FilledRect, Image, ImageId, Line, StraightLine,
    },
    input::Action,
    scene::SceneData,
};

pub trait StateHandler {
    fn on_enter(&mut self, _data: &mut SceneData, _window: &mut Window) {}
    fn on_exit(&mut self, _data: &mut SceneData, _window: &mut Window) {}

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition;

    fn draw(&self, data: &mut SceneData, window: &mut Window) {
        // TODO: consider different draw implementation for each state

        data.redraw |= data.camera.update(data.mouse);

        if data.redraw {
            data.canvas.clear();

            let mut combined = Vec::<&dyn Drawable>::new();
            combined.extend(data.contents.images.iter().map(|i| i as &dyn Drawable));
            combined.extend(data.contents.lines.iter().map(|i| i as &dyn Drawable));
            combined.extend(data.contents.erasers.iter().map(|i| i as &dyn Drawable));
            combined.sort_by_key(|i| i.z());
            combined
                .iter()
                .for_each(|i| i.draw(&mut data.canvas, &data.camera));
        } else if let Some(line) = data.contents.lines.last() {
            if line.points.len() >= 2 && !line.finished {
                let mut l = Line::new(line.points[line.points.len() - 2], line.brush, line.z());
                l.points.push(line.points[line.points.len() - 1]);
                l.draw(&mut data.canvas, &data.camera);
            }
        }

        window
            .update_with_buffer(
                &data.canvas.get_buffer(),
                data.canvas.width as usize,
                data.canvas.height as usize,
            )
            .unwrap();
    }
}

pub struct Idle;
struct Drawing;
struct MovingCanvas;
struct ModifyingImage(ImageId);
struct MovingImage {
    id: ImageId,
    start_pos: Point,
}
struct ResizingImage {
    id: ImageId,
    start_scale: f32,
}

struct Erasing {
    eraser: Option<FilledRect>,
}

impl Erasing {
    pub fn new() -> Self {
        Self { eraser: None }
    }
}

pub enum Transition {
    Stay,
    Switch(Box<dyn StateHandler>),
}

impl Idle {
    fn try_paste_image(&self, data: &mut SceneData, image_data: ImageData) {
        let img = Pixmap::from_vec(
            image_data.bytes.to_vec(),
            IntSize::from_wh(image_data.width as u32, image_data.height as u32).unwrap(),
        );

        if let Some(img) = img {
            let pos = {
                if let Some(m) = data.mouse {
                    Point::from_xy(
                        m.x - image_data.width as f32 / 2.0,
                        m.y - image_data.height as f32 / 2.0,
                    )
                } else {
                    data.camera.pos
                }
            };
            let pos = data.camera.to_camera_coords((pos.x as u32, pos.y as u32));
            data.contents.z += 1;
            let image = Image::new(
                pos,
                img,
                1.0 / data.camera.zoom,
                data.contents.next_image_id(),
                data.contents.z,
                &data.config,
            );
            image.draw(&mut data.canvas, &data.camera);
            data.contents.images.push(image.clone());
            data.command_invoker.push(command::PasteImage::new(image));
        }
    }
}

impl StateHandler for Idle {
    fn on_enter(&mut self, _data: &mut SceneData, window: &mut Window) {
        window.set_cursor_visibility(false);
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if let Some(mouse) = data.mouse {
            let cursor = FilledCircle {
                pos: mouse,
                brush: Brush {
                    color: data.brush.color,
                    thickness: data.brush.thickness * data.camera.zoom,
                },
            };
            cursor.draw(&mut data.canvas, &data.camera);
        }

        if window.get_mouse_down(MouseButton::Left) {
            if window.is_key_down(Key::LeftCtrl) {
                if let Some(id) = data.image_under_cursor() {
                    return Transition::Switch(Box::new(ModifyingImage(id)));
                }
            }
            return Transition::Switch(Box::new(Drawing));
        }

        if window.get_mouse_down(MouseButton::Right) {
            if window.is_key_down(Key::LeftCtrl) {
                return Transition::Switch(Box::new(Erasing::new()));
            }

            return Transition::Switch(Box::new(MovingCanvas));
        }

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if window.is_key_down(Key::LeftCtrl) {
                data.update_thickness(scroll_y);
            } else {
                data.update_zoom(scroll_y);
            }
        }

        match data.input_handler.interpret(window) {
            Action::Undo => data.undo(),
            Action::Redo => data.redo(),
            Action::NextColor => data.update_color(true),
            Action::PrevColor => data.update_color(false),
            Action::Paste => {
                if let Some(ref mut clipboard) = data.clipboard {
                    if let Ok(image_data) = clipboard.get_image() {
                        self.try_paste_image(data, image_data);
                    }
                }
            }
            _ => {}
        }

        Transition::Stay
    }
}

impl StateHandler for Drawing {
    fn on_enter(&mut self, _data: &mut SceneData, window: &mut Window) {
        window.set_cursor_visibility(false);
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut Window) {
        if let Some(last) = data.contents.lines.last_mut() {
            last.finished = true;
            // to draw the rest when holding shift
            if let Some(mouse) = data.mouse {
                last.points.push(
                    data.camera
                        .to_camera_coords((mouse.x as u32, mouse.y as u32)),
                );
                data.redraw = true;
            }
            let cmd = DrawLine::new(data.contents.lines.last().unwrap().clone());
            data.command_invoker.push(cmd);
        }
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if !window.get_mouse_down(MouseButton::Left) {
            return Transition::Switch(Box::new(Idle));
        }

        let Some(mouse) = data.mouse else {
            return Transition::Switch(Box::new(Idle));
        };

        let pos = data
            .camera
            .to_camera_coords((mouse.x as u32, mouse.y as u32));

        FilledCircle {
            pos: mouse,
            brush: Brush {
                color: data.brush.color,
                thickness: data.brush.thickness * data.camera.zoom,
            },
        }
        .draw(&mut data.canvas, &data.camera);

        if let Some(line) = data.contents.lines.last_mut() {
            if line.finished {
                data.contents
                    .lines
                    .push(Line::new(pos, data.brush, data.contents.z));
            } else if !window.is_key_down(Key::LeftShift)
                && line.points.last().unwrap().distance(pos) >= 5.0 / data.camera.zoom
            {
                line.points.push(pos);
            } else {
                StraightLine {
                    start: *line.points.last().unwrap(),
                    end: pos,
                    brush: data.brush,
                }
                .draw(&mut data.canvas, &data.camera);
            }
        } else {
            data.contents
                .lines
                .push(Line::new(pos, data.brush, data.contents.z));
        }

        Transition::Stay
    }
}

impl StateHandler for MovingCanvas {
    fn on_enter(&mut self, _data: &mut SceneData, window: &mut Window) {
        window.set_cursor_visibility(true);
        window.set_cursor_style(minifb::CursorStyle::ClosedHand);
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if !window.get_mouse_down(MouseButton::Right) {
            return Transition::Switch(Box::new(Idle));
        }

        if let Some(mouse) = data.mouse {
            data.camera.update_pos(mouse, data.prev_mouse);
        }

        Transition::Stay
    }
}

impl StateHandler for Erasing {
    fn on_enter(&mut self, data: &mut SceneData, window: &mut Window) {
        window.set_cursor_visibility(true);
        window.set_cursor_style(CursorStyle::Arrow);

        self.eraser = Some(FilledRect {
            pos: data
                .mouse
                .expect("Mouse should be available when entering the Erasing state"),
            width: 0.0,
            height: 0.0,
            color: data.config.background,
        });
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut Window) {
        if let Some(eraser) = &self.eraser {
            let pos = data
                .camera
                .to_camera_coords((eraser.pos.x as u32, eraser.pos.y as u32));
            let rect = FilledRect {
                pos,
                width: eraser.width / data.camera.zoom,
                height: eraser.height / data.camera.zoom,
                color: eraser.color,
            }
            .to_skia();

            data.contents
                .erasers
                .push(Eraser::new(rect, eraser.color, data.contents.z));
            data.contents.z += 1;
            data.redraw = true;
            data.command_invoker.push(AddEraser::new(
                data.contents.erasers.last().unwrap().clone(),
            ));
        }
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if !window.is_key_down(Key::LeftCtrl) || !window.get_mouse_down(MouseButton::Right) {
            return Transition::Switch(Box::new(Idle));
        }

        if let Some(eraser) = self.eraser.as_mut() {
            if let (Some(m), Some(pm)) = (data.mouse, data.prev_mouse) {
                eraser.width += m.x - pm.x;
                eraser.height += m.y - pm.y;
            }

            eraser.draw(&mut data.canvas, &data.camera);
        }

        Transition::Stay
    }
}

impl StateHandler for ModifyingImage {
    fn on_enter(&mut self, data: &mut SceneData, window: &mut Window) {
        data.contents
            .image(self.0)
            .expect("In modifying image state an image with provided id should exist")
            .is_selected = true;
        data.redraw = true;

        window.set_cursor_visibility(true);
        window.set_cursor_style(CursorStyle::Arrow);
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut Window) {
        let Some(img) = data.contents.image(self.0) else {
            return;
        };

        img.is_selected = false;
        data.redraw = true;
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if window.get_mouse_down(MouseButton::Left) {
            if data.image_under_cursor() != Some(self.0) {
                return Transition::Switch(Box::new(Idle));
            }
            return Transition::Switch(Box::new(MovingImage::new(self.0)));
        }

        if window.get_mouse_down(MouseButton::Right) {
            if data.image_under_cursor() != Some(self.0) {
                return Transition::Switch(Box::new(Idle));
            }
            return Transition::Switch(Box::new(ResizingImage::new(self.0)));
        }

        if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
            if !window.is_key_down(Key::LeftCtrl) {
                data.update_zoom(scroll_y);
            }
        }

        match data.input_handler.interpret(window) {
            Action::Remove => {
                let mut img = data.contents.remove_image(self.0).unwrap();
                img.is_selected = false;
                data.command_invoker.push(command::RemoveImage::new(img));
                data.redraw = true;

                return Transition::Switch(Box::new(Idle));
            }
            Action::UpLayer => {
                data.contents.move_image_up(self.0);
                data.redraw = true;
            }
            Action::DownLayer => {
                data.contents.move_image_down(self.0);
                data.redraw = true;
            }
            Action::None => {}
            _ => {
                // strange, but it's a way for the Idle state to handle the
                // input on the next frame
                return Transition::Switch(Box::new(Idle));
            }
        };

        Transition::Stay
    }
}

impl ResizingImage {
    pub fn new(id: ImageId) -> Self {
        Self {
            id,
            start_scale: 1.0,
        }
    }
}

impl StateHandler for ResizingImage {
    fn on_enter(&mut self, data: &mut SceneData, window: &mut Window) {
        window.set_cursor_visibility(true);
        window.set_cursor_style(CursorStyle::ResizeAll);
        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when exiting resizing state");
        self.start_scale = img.scale;
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut Window) {
        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when exiting resizing state");

        data.command_invoker.push(command::ResizeImage::new(
            self.id,
            self.start_scale,
            img.scale,
        ));
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if !window.get_mouse_down(MouseButton::Right) {
            return Transition::Switch(Box::new(ModifyingImage(self.id)));
        }

        if let (Some(m), Some(pm)) = (data.mouse, data.prev_mouse) {
            let img = data
                .contents
                .image(self.id)
                .expect("Image id should be correct when in resizing state");
            let d = m - pm;
            let sx = d.x / img.width() / data.camera.zoom;
            let sy = d.y / img.height() / data.camera.zoom;
            img.scale *= 1.0 + if sx.abs() > sy.abs() { sx } else { sy };
        }

        data.redraw = true;

        Transition::Stay
    }
}

impl MovingImage {
    pub fn new(id: ImageId) -> Self {
        Self {
            id,
            start_pos: Point::default(),
        }
    }
}

impl StateHandler for MovingImage {
    fn on_enter(&mut self, data: &mut SceneData, _window: &mut Window) {
        self.start_pos = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when entering moving state")
            .pos;
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut Window) {
        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when exiting moving state");

        if self.start_pos.distance(img.pos) * data.camera.zoom > 10.0 {
            data.command_invoker
                .push(command::MoveImage::new(self.id, self.start_pos, img.pos));
        }
    }

    fn step(&mut self, data: &mut SceneData, window: &Window) -> Transition {
        if !window.get_mouse_down(MouseButton::Left) {
            return Transition::Switch(Box::new(ModifyingImage(self.id)));
        }

        if let (Some(mouse), Some(prev_mouse)) = (data.mouse, data.prev_mouse) {
            let img = data
                .contents
                .image(self.id)
                .expect("Image id should be correct when in moving state");
            img.pos += Point::from_xy(
                (mouse.x - prev_mouse.x) / data.camera.zoom,
                (mouse.y - prev_mouse.y) / data.camera.zoom,
            );
        }

        data.redraw = true;
        Transition::Stay
    }
}
