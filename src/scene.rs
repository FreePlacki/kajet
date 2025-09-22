use std::{
    ops::{Add, Mul},
    rc::Rc,
};

use arboard::Clipboard;
use raylib::{RaylibHandle, RaylibThread};
use widok::{
    Camera, CanvasLength, CanvasSize, CanvasSpace, CanvasToScreenScale, ScreenPoint, ToCanvas,
};

use crate::{
    command::CommandInvoker,
    config::Config,
    graphics::{Brush, Contents, Drawable, ImageId},
    input::InputHandler,
    state::{self, StateHandler, Transition},
};

pub struct Scene {
    state: Box<dyn StateHandler>,
    data: SceneData,
}

impl Scene {
    pub fn new(config: Config, clipboard: Option<Clipboard>, rl: &mut RaylibHandle) -> Self {
        let mut sm = Self {
            state: Box::new(state::Idle),
            data: SceneData::new(config, clipboard),
        };

        sm.state.on_enter(&mut sm.data, rl);
        sm
    }

    pub fn process_frame(&mut self, thread: &RaylibThread, rl: &mut RaylibHandle) {
        if let Transition::Switch(mut next) = self.state.step(&mut self.data, thread, rl) {
            self.state.on_exit(&mut self.data, rl);
            next.on_enter(&mut self.data, rl);
            self.state = next;
        }

        self.state.draw(&mut self.data, thread, rl);
        self.data.contents.overlay.clear();
    }
}

pub struct SceneData {
    pub input_handler: InputHandler,
    pub camera: Camera,
    pub contents: Contents,
    pub command_invoker: CommandInvoker,
    pub brush: Brush<CanvasSpace>,
    pub config: Rc<Config>,
    pub clipboard: Option<Clipboard>,
    color_idx: usize,
}

impl SceneData {
    pub fn new(config: Config, clipboard: Option<Clipboard>) -> Self {
        let config = Rc::new(config);
        let brush = Brush {
            color: config.colors[0],
            thickness: CanvasLength::new(config.thickness),
        };
        let command_invoker = CommandInvoker::new(config.undo_buffer_size);
        let input_handler = InputHandler::new(Rc::clone(&config));
        let camera = Camera::new(CanvasSize::new(0.0, 0.0));

        Self {
            camera,
            config,
            color_idx: 0,
            brush,
            contents: Contents::new(),
            clipboard,
            command_invoker,
            input_handler,
        }
    }

    pub fn update_thickness(&mut self, scroll_y: f32) {
        self.brush.thickness = self
            .brush
            .thickness
            .add(CanvasLength::new(scroll_y.signum()))
            .min(CanvasLength::new(30.0))
            .max(CanvasLength::new(1.0));
    }

    pub fn update_zoom(&mut self, scroll_y: f32) {
        let new_zoom = self
            .camera
            .zoom()
            .0
            .mul(1.0 + scroll_y.signum() * 0.2 * self.config.scroll_sensitivity)
            .clamp(0.1, 30.0);
        self.camera.update_zoom(CanvasToScreenScale::new(new_zoom));
    }

    pub fn update_color(&mut self, forward: bool) {
        self.color_idx = (self.color_idx as i32 + if forward { 1 } else { -1 })
            .rem_euclid(self.config.colors.len() as i32) as usize;

        self.brush.color = self.config.colors[self.color_idx];
    }

    pub fn image_under_cursor(&self, mouse: ScreenPoint) -> Option<ImageId> {
        self.contents
            .images
            .iter()
            .filter(|i| i.in_bounds(mouse.to_canvas(&self.camera)))
            .max_by_key(|i| i.z())
            .map(|i| i.id)
    }
}
