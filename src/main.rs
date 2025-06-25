use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

fn main() {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    let mut window = Window::new("Kajet", WIDTH, HEIGHT, WindowOptions::default())
        .expect("Unable to create the window");

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
            let screen_pos = ((y as usize) * WIDTH) + x as usize;

            if window.get_mouse_down(MouseButton::Left) {
                buffer[screen_pos] = 0x00ffffff;
            }

            if window.get_mouse_down(MouseButton::Right) {
                buffer[screen_pos] = 0x00000000;
            }
        }

        if let Some((scroll_x, scroll_y)) = window.get_scroll_wheel() {
            println!("Scrolling {} - {}", scroll_x, scroll_y);
        }

        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
