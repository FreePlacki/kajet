use std::{
    ops::{Add, Mul},
    rc::Rc,
};

use arboard::Clipboard;
use minifb::{MouseMode, Window};
use tiny_skia::Point;

use crate::{
    camera::Camera,
    canvas::Canvas,
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
    pub fn new(config: Config, clipboard: Option<Clipboard>, window: &mut Window) -> Self {
        let (width, height) = window.get_size();
        let mut sm = Self {
            state: Box::new(state::Idle),
            data: SceneData::new(width as u32, height as u32, config, clipboard),
        };

        sm.state.on_enter(&mut sm.data, window);
        sm
    }

    pub fn process_frame(&mut self, window: &mut Window) {
        let (width, height) = window.get_size();

        self.data.redraw = self.data.canvas.set_size((width as u32, height as u32));
        self.data.prev_mouse = self.data.mouse;
        self.data.mouse = window
            .get_mouse_pos(MouseMode::Discard)
            .map(|(x, y)| Point::from_xy(x, y));

        self.data.canvas.clear_overlay();

        if let Transition::Switch(mut next) = self.state.step(&mut self.data, window) {
            self.state.on_exit(&mut self.data, window);
            next.on_enter(&mut self.data, window);
            self.state = next;
        }

        self.state.draw(&mut self.data, window);
    }
}

pub struct SceneData {
    pub input_handler: InputHandler,
    pub camera: Camera,
    pub contents: Contents,
    pub canvas: Canvas,
    pub mouse: Option<Point>,
    pub prev_mouse: Option<Point>,
    pub redraw: bool,
    pub command_invoker: CommandInvoker,
    pub brush: Brush,
    pub config: Rc<Config>,
    pub clipboard: Option<Clipboard>,
    color_idx: usize,
}

impl SceneData {
    pub fn new(width: u32, height: u32, config: Config, clipboard: Option<Clipboard>) -> Self {
        let config = Rc::new(config);
        let brush = Brush {
            color: config.colors[0],
            thickness: config.thickness,
        };
        let command_invoker = CommandInvoker::new(config.undo_buffer_size);
        let canvas = Canvas::new(width, height, config.background);
        let input_handler = InputHandler::new(Rc::clone(&config));

        Self {
            canvas,
            camera: Camera::default(),
            config,
            color_idx: 0,
            brush,
            contents: Contents::new(),
            redraw: false,
            mouse: None,
            prev_mouse: None,
            clipboard,
            command_invoker,
            input_handler,
        }
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
        self.color_idx = (self.color_idx as i32 + if forward { 1 } else { -1 })
            .rem_euclid(self.config.colors.len() as i32) as usize;

        self.brush.color = self.config.colors[self.color_idx];
    }

    pub fn image_under_cursor(&self) -> Option<ImageId> {
        let Some(mouse) = self.mouse else {
            return None;
        };

        self.contents
            .images
            .iter()
            .filter(|i| i.in_bounds(mouse, &self.camera))
            .max_by_key(|i| i.z())
            .map(|i| i.id)
    }

    pub fn undo(&mut self) {
        dbg!(&self.command_invoker);
        self.command_invoker.undo(&mut self.contents);
        self.redraw = true;
    }

    pub fn redo(&mut self) {
        self.command_invoker.redo(&mut self.contents);
        self.redraw = true;
    }
}
