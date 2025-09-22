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
use widok::{
    CanvasLength, CanvasPoint, CanvasRect, CanvasSize, ScreenPoint, ScreenSize, ScreenSpace,
    ScreenVector, ToCanvas, ToScreen,
};

use crate::graphics::{Brush, FilledCircle, StraightLine};
use crate::{
    command::{self, AddEraser, DrawLine},
    graphics::{Drawable, Eraser, FilledRect, Image, ImageId, Line},
    input::Action,
    scene::SceneData,
};

fn mouse_pos(rl: &RaylibHandle) -> ScreenPoint {
    let Vector2 { x, y } = rl.get_mouse_position();
    ScreenPoint::new(x, y)
}

fn mouse_delta(rl: &RaylibHandle) -> ScreenVector {
    let Vector2 { x, y } = rl.get_mouse_delta();
    ScreenVector::new(x, y)
}

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
        let size = ScreenSize::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        data.camera.update(rl.get_frame_time(), mouse_pos(rl), size);

        // DO NOT USE RaylibHandle::draw as it results in some input being dropped!
        let mut d = rl.begin_drawing(thread);
        d.clear_background(data.config.background);

        let mut combined = Vec::<&dyn Drawable>::new();
        combined.extend(data.contents.images.iter().map(|i| i as &dyn Drawable));
        combined.extend(data.contents.lines.iter().map(|i| i as &dyn Drawable));
        combined.extend(data.contents.erasers.iter().map(|i| i as &dyn Drawable));
        combined = combined
            .into_iter()
            .filter(|i| i.is_in_view(&data.camera))
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
    start_pos: CanvasPoint,
}
struct ResizingImage {
    id: ImageId,
    start_scale: CanvasLength,
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

        let mouse = mouse_pos(rl);
        let delta = ScreenVector::new(
            image_data.width as f32 / 2.0,
            image_data.height as f32 / 2.0,
        );
        let pos = mouse - delta;

        // TODO: consider adding this to contents instead of exposing this api
        data.contents.z += 1;
        let image = Image::new(
            pos.to_canvas(&data.camera),
            texture,
            CanvasLength::new(1.0 / data.camera.zoom().0),
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
        let mouse = mouse_pos(rl);

        data.contents.overlay.push(Box::new(FilledCircle {
            pos: mouse,
            brush: Brush {
                color: data.brush.color,
                thickness: data.brush.thickness.to_screen(&data.camera),
            },
        }));

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                && let Some(id) = data.image_under_cursor(mouse)
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
        let pos = mouse_pos(rl).to_canvas(&data.camera);
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

        let mouse = mouse_pos(rl);
        data.contents.overlay.push(Box::new(FilledCircle {
            pos: mouse,
            brush: Brush {
                color: data.brush.color,
                thickness: data.brush.thickness.to_screen(&data.camera),
            },
        }));

        if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
            return Transition::Switch(Box::new(DrawingStraight));
        }

        let pos = mouse.to_canvas(&data.camera);
        let line = data
            .contents
            .lines
            .last_mut()
            .expect("A line should be present because we insert a new one on_enter");
        if line.finished {
            data.contents
                .lines
                .push(Line::new(pos, data.brush, data.contents.z));
        } else if CanvasLength::new(line.points.last().unwrap().distance_to(pos))
            .to_screen(&data.camera)
            .0
            >= 5.0
        {
            line.points.push(pos);
        }

        Transition::Stay
    }
}

impl StateHandler for DrawingStraight {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        let pos = mouse_pos(rl).to_canvas(&data.camera);

        data.contents
            .lines
            .push(Line::new(pos, data.brush, data.contents.z));
    }

    fn on_exit(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        let pos = mouse_pos(rl).to_canvas(&data.camera);

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
            data.contents.overlay.push(Box::new(StraightLine {
                start: line.points.last().unwrap().to_screen(&data.camera),
                end: mouse_pos(rl),
                brush: Brush::<ScreenSpace> {
                    color: data.brush.color,
                    thickness: data.brush.thickness.to_screen(&data.camera),
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

        data.camera.update_pos(mouse_delta(rl));

        Transition::Stay
    }
}

impl StateHandler for Erasing {
    fn on_enter(&mut self, data: &mut SceneData, rl: &mut RaylibHandle) {
        rl.show_cursor();
        rl.set_mouse_cursor(MouseCursor::MOUSE_CURSOR_ARROW);

        let start = mouse_pos(rl).to_canvas(&data.camera);
        self.eraser = Some(FilledRect::new(
            CanvasRect::new(start, CanvasSize::new(0.0, 0.0)),
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
            let d = mouse_delta(rl).to_canvas(&data.camera);
            eraser.rect = CanvasRect::new(
                eraser.rect.origin,
                CanvasSize::new(eraser.rect.size.width + d.x, eraser.rect.size.height + d.y),
            );

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
        let image_under_cursor = data.image_under_cursor(mouse_pos(rl)) != Some(self.0);
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
            start_scale: CanvasLength::new(1.0),
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
        let d = mouse_delta(rl).to_canvas(&data.camera);
        let dx = d.x / img.width().0;
        let dy = d.y / img.height().0;
        img.scale *= 1.0 + if dx.abs() > dy.abs() { dx } else { dy };

        Transition::Stay
    }
}

impl MovingImage {
    pub fn new(id: ImageId) -> Self {
        Self {
            id,
            start_pos: CanvasPoint::default(),
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

        if CanvasLength::new(self.start_pos.distance_to(img.pos))
            .to_screen(&data.camera)
            .0
            > 10.0
        {
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
        img.pos += mouse_delta(rl).to_canvas(&data.camera);

        Transition::Stay
    }
}
