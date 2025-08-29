use arboard::ImageData;
use raylib::ffi;
use raylib::{
    RaylibHandle, RaylibThread,
    ffi::{KeyboardKey, MouseButton, MouseCursor},
    math::Vector2,
    prelude::RaylibDraw,
    texture,
};
use std::ffi::c_void;

use crate::graphics::{Brush, FilledCircle, StraightLine};
use crate::units::{CanvasSpace, Length, Point, Rect, ScreenSpace, Transformable, Vector};
use crate::{
    command::{self, AddEraser, DrawLine},
    graphics::{Drawable, Eraser, FilledRect, Image, ImageId, Line},
    input::Action,
    scene::SceneData,
};

pub trait StateHandler {
    fn on_enter(&mut self, _data: &mut SceneData, _rl: &mut RaylibHandle) {}
    fn on_exit(&mut self, _data: &mut SceneData, _rl: &mut RaylibHandle) {}

    fn step(
        &mut self,
        data: &mut SceneData,
        thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition;

    fn draw(&self, data: &mut SceneData, thread: &RaylibThread, rl: &mut RaylibHandle) {
        data.camera.update(rl);

        // DO NOT USE RaylibHandle::draw as it results in some input being dropped!
        let mut d = rl.begin_drawing(thread);
        d.clear_background(data.config.background);

        let mut combined = Vec::<&dyn Drawable>::new();
        combined.extend(data.contents.images.iter().map(|i| i as &dyn Drawable));
        combined.extend(data.contents.lines.iter().map(|i| i as &dyn Drawable));
        combined.extend(data.contents.erasers.iter().map(|i| i as &dyn Drawable));
        combined = combined
            .into_iter()
            .filter(|i| i.is_visible(&data.camera))
            .collect::<Vec<_>>();
        combined.sort_by_key(|i| i.z());
        combined.extend(data.contents.overlay.iter().map(|i| &**i as &dyn Drawable));
        combined.iter().for_each(|i| i.draw(&mut d, &data.camera));

        if data.config.show_fps {
            d.draw_fps(50, 50);
        }
    }
}

pub struct Idle;
struct Drawing;
struct DrawingStraight;
struct MovingCanvas;
struct ModifyingImage(ImageId);
struct MovingImage {
    id: ImageId,
    start_pos: Point<CanvasSpace>,
}
struct ResizingImage {
    id: ImageId,
    start_scale: Length<CanvasSpace>,
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
    fn image_from_arboard(data: &arboard::ImageData) -> texture::Image {
        let mut rl_image = ffi::Image {
            data: data.bytes.as_ptr() as *mut c_void,
            width: data.width as i32,
            height: data.height as i32,
            mipmaps: 1,
            format: ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
        };

        // SAFETY: We must clone the pixel buffer so that `rl_image` owns it
        // Raylib expects to be able to free this memory later
        let owned = data.bytes.clone();
        rl_image.data = owned.as_ptr() as *mut c_void;
        std::mem::forget(owned);

        unsafe { texture::Image::from_raw(rl_image) }
    }

    fn try_paste_image(
        &self,
        data: &mut SceneData,
        thread: &RaylibThread,
        rl: &mut RaylibHandle,
        image_data: ImageData,
    ) {
        let rl_image = Self::image_from_arboard(&image_data);

        let Ok(texture) = rl.load_texture_from_image(thread, &rl_image) else {
            return;
        };

        let mouse = Point::<ScreenSpace>::new(rl.get_mouse_position());
        let delta = Vector::<ScreenSpace>::new(Vector2::new(
            image_data.width as f32 / 2.0,
            image_data.height as f32 / 2.0,
        ));
        let pos = mouse - delta;

        // TODO: consider adding this to contents instead of exposing this api
        data.contents.z += 1;
        let image = Image::new(
            pos.transform(&data.camera),
            texture,
            Length::new(1.0 / data.camera.zoom),
            data.contents.next_image_id(),
            data.contents.z,
            &data.config,
        );
        data.contents.images.push(image.clone());
        data.command_invoker.push(command::PasteImage::new(image));
    }
}

impl StateHandler for Idle {
    fn on_enter(&mut self, _data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.hide_cursor();
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        data.contents.overlay.push(Box::new(FilledCircle {
            pos: Point::new(rl.get_mouse_position()),
            brush: Brush {
                color: data.brush.color,
                thickness: data.brush.thickness.transform(&data.camera),
            },
        }));

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                && let Some(id) = data.image_under_cursor(Point::new(rl.get_mouse_position()))
            {
                return Transition::Switch(Box::new(ModifyingImage(id)));
            }
            return Transition::Switch(Box::new(Drawing));
        }

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                return Transition::Switch(Box::new(Erasing::new()));
            }

            return Transition::Switch(Box::new(MovingCanvas));
        }

        let scroll = rl.get_mouse_wheel_move_v();
        if scroll.y != 0.0 {
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                data.update_thickness(scroll.y);
            } else {
                data.update_zoom(scroll.y);
            }
        }

        match data.input_handler.interpret(rl) {
            Action::Undo => data.command_invoker.undo(&mut data.contents),
            Action::Redo => data.command_invoker.redo(&mut data.contents),
            Action::NextColor => data.update_color(true),
            Action::PrevColor => data.update_color(false),
            Action::Paste => {
                if let Some(ref mut clipboard) = data.clipboard
                    && let Ok(image_data) = clipboard.get_image()
                {
                    self.try_paste_image(data, thread, rl, image_data);
                }
            }
            _ => {}
        }

        Transition::Stay
    }
}

impl StateHandler for Drawing {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.hide_cursor();
        let pos = Point::<ScreenSpace>::new(rl.get_mouse_position()).transform(&data.camera);
        data.contents
            .lines
            .push(Line::new(pos, data.brush, data.contents.z));
    }

    fn on_exit(&mut self, data: &mut SceneData, _rl: &mut RaylibHandle) {
        if let Some(last) = data.contents.lines.last_mut() {
            last.finished = true;
            let cmd = DrawLine::new(data.contents.lines.last().unwrap().clone());
            data.command_invoker.push(cmd);
        }
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            return Transition::Switch(Box::new(Idle));
        }

        data.contents.overlay.push(Box::new(FilledCircle {
            pos: Point::<ScreenSpace>::new(rl.get_mouse_position()),
            brush: Brush::<ScreenSpace> {
                color: data.brush.color,
                thickness: data.brush.thickness.transform(&data.camera),
            },
        }));

        if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            return Transition::Switch(Box::new(DrawingStraight));
        }

        let pos = Point::<ScreenSpace>::new(rl.get_mouse_position()).transform(&data.camera);
        let line = data
            .contents
            .lines
            .last_mut()
            .expect("A line should be present because we insert a new one on_enter");
        if line.finished {
            data.contents
                .lines
                .push(Line::new(pos, data.brush, data.contents.z));
        } else if f32::from(
            line.points
                .last()
                .unwrap()
                .distance_to(&pos)
                .transform(&data.camera),
        ) >= 5.0
        {
            line.points.push(pos);
        }

        Transition::Stay
    }
}

impl StateHandler for DrawingStraight {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        let pos = Point::<ScreenSpace>::new(rl.get_mouse_position()).transform(&data.camera);

        data.contents
            .lines
            .push(Line::new(pos, data.brush, data.contents.z));
    }

    fn on_exit(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        let pos = Point::<ScreenSpace>::new(rl.get_mouse_position()).transform(&data.camera);

        let line =
            data.contents.lines.last_mut().expect(
                "There should be a line already when exiting the straight line drawing state.",
            );
        line.points.push(pos);
        line.finished = true;

        let cmd = DrawLine::new(data.contents.lines.last().unwrap().clone());
        data.command_invoker.push(cmd);
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                return Transition::Switch(Box::new(Drawing));
            }
            return Transition::Switch(Box::new(Idle));
        }
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            return Transition::Switch(Box::new(Idle));
        }

        if let Some(line) = data.contents.lines.last() {
            let pos = Point::<ScreenSpace>::new(rl.get_mouse_position());

            data.contents.overlay.push(Box::new(StraightLine {
                start: line.points.last().unwrap().transform(&data.camera),
                end: pos,
                brush: Brush::<ScreenSpace> {
                    color: data.brush.color,
                    thickness: data.brush.thickness.transform(&data.camera),
                },
            }));
        }

        Transition::Stay
    }
}

impl StateHandler for MovingCanvas {
    fn on_enter(&mut self, _data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.show_cursor();
        rl.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_POINTING_HAND);
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            return Transition::Switch(Box::new(Idle));
        }

        data.camera
            .update_pos(Vector::<ScreenSpace>::new(rl.get_mouse_delta()));

        Transition::Stay
    }
}

impl StateHandler for Erasing {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.show_cursor();
        rl.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_ARROW);

        let start = Point::<ScreenSpace>::new(rl.get_mouse_position()).transform(&data.camera);
        self.eraser = Some(FilledRect::new(
            Rect::new(start, Length::new(0.0), Length::new(0.0)),
            data.config.background,
        ));
    }

    fn on_exit(&mut self, data: &mut SceneData, _rl: &mut RaylibHandle) {
        if let Some(eraser) = &self.eraser {
            data.contents
                .erasers
                .push(Eraser::new(eraser.rect, eraser.color, data.contents.z));
            data.contents.z += 1;
            data.command_invoker
                .push(AddEraser::new(*data.contents.erasers.last().unwrap()));
        }
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT)
        {
            return Transition::Switch(Box::new(Idle));
        }

        if let Some(eraser) = self.eraser.as_mut() {
            let d = Vector::<ScreenSpace>::new(rl.get_mouse_delta()).transform(&data.camera);
            *eraser.rect.w_mut() += d.x();
            *eraser.rect.h_mut() += d.y();

            data.contents.overlay.push(Box::new(*eraser));
        }

        Transition::Stay
    }
}

impl StateHandler for ModifyingImage {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        data.contents
            .image(self.0)
            .expect("In modifying image state an image with provided id should exist")
            .is_selected = true;

        rl.show_cursor();
        rl.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_ARROW);
    }

    fn on_exit(&mut self, data: &mut SceneData, _rl: &mut RaylibHandle) {
        let Some(img) = data.contents.image(self.0) else {
            return;
        };

        img.is_selected = false;
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        let image_under_cursor =
            data.image_under_cursor(Point::new(rl.get_mouse_position())) != Some(self.0);
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            if image_under_cursor {
                return Transition::Switch(Box::new(Idle));
            }
            return Transition::Switch(Box::new(MovingImage::new(self.0)));
        }

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            if image_under_cursor {
                return Transition::Switch(Box::new(Idle));
            }
            return Transition::Switch(Box::new(ResizingImage::new(self.0)));
        }

        let scroll = rl.get_mouse_wheel_move_v();
        if scroll.y != 0.0 && !rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
            data.update_zoom(scroll.y);
        }

        match data.input_handler.interpret(rl) {
            Action::Remove => {
                let mut img = data.contents.remove_image(self.0).unwrap();
                img.is_selected = false;
                data.command_invoker.push(command::RemoveImage::new(img));

                return Transition::Switch(Box::new(Idle));
            }
            Action::UpLayer => {
                data.contents.move_image_up(self.0);
            }
            Action::DownLayer => {
                data.contents.move_image_down(self.0);
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
            start_scale: Length::new(1.0),
        }
    }
}

impl StateHandler for ResizingImage {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.show_cursor();
        rl.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_RESIZE_NWSE);
        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when exiting resizing state");
        self.start_scale = img.scale;
    }

    fn on_exit(&mut self, data: &mut SceneData, _window: &mut RaylibHandle) {
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

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            return Transition::Switch(Box::new(ModifyingImage(self.id)));
        }

        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when in resizing state");
        let d = Point::<ScreenSpace>::new(rl.get_mouse_delta());
        unsafe {
            let sx: f32 = d.x().transform(&data.camera).v() / img.width().v();
            let sy: f32 = d.y().transform(&data.camera).v() / img.height().v();
            img.scale *= 1.0 + if sx.abs() > sy.abs() { sx } else { sy };
        }

        Transition::Stay
    }
}

impl MovingImage {
    pub fn new(id: ImageId) -> Self {
        Self {
            id,
            start_pos: Point::new(Vector2::default()),
        }
    }
}

impl StateHandler for MovingImage {
    fn on_enter(&mut self, data: &mut SceneData, _rl: &mut RaylibHandle) {
        self.start_pos = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when entering moving state")
            .pos;
    }

    fn on_exit(&mut self, data: &mut SceneData, _rl: &mut RaylibHandle) {
        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when exiting moving state");

        if f32::from(self.start_pos.distance_to(&img.pos).transform(&data.camera)) > 10.0 {
            data.command_invoker
                .push(command::MoveImage::new(self.id, self.start_pos, img.pos));
        }
    }

    fn step(
        &mut self,
        data: &mut SceneData,
        _thread: &RaylibThread,
        rl: &mut RaylibHandle,
    ) -> Transition {
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            return Transition::Switch(Box::new(ModifyingImage(self.id)));
        }

        let img = data
            .contents
            .image(self.id)
            .expect("Image id should be correct when in moving state");
        img.pos += Vector::<ScreenSpace>::new(rl.get_mouse_delta()).transform(&data.camera);

        Transition::Stay
    }
}
