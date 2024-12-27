use std::time::Duration;

use sdl2::{
    event::Event,
    keyboard::Scancode,
    pixels::Color, rect::Rect,
};

use chip8_core::{point_from_index, Chip8, PIXELS_PER_COLUMN, PIXELS_PER_ROW};

const SQUARE_SIZE: u32 = 20;
const SCREEN_WIDTH: u32 = PIXELS_PER_ROW as u32 * SQUARE_SIZE;
const SCREEN_HEIGHT: u32 = PIXELS_PER_COLUMN as u32 * SQUARE_SIZE;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("CHIP-8 Emulator", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    // let blue_latte = Color::RGB(30, 102, 245);
    let base_mocha = Color::RGB(30, 30, 46);
    let yellow_mocha = Color::RGB(249, 226, 175);
    canvas.set_draw_color(base_mocha);
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut chip8 = Chip8::new();
    let instructions_per_frame = 3;
    let rom =
            std::fs::read("/home/flynn/开发者/开发中/chip8/ROMs/test/4-flags.ch8").unwrap();
    chip8.load_rom(&rom);
    'running: loop {
        // Parse events
        let mut new_frame_keys = [false; 16];
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = get_keypad_button_from_scancode(scancode) {
                        new_frame_keys[key] = true;
                    }
                }
                _ => {}
            }
        }

        // Update keys
        println!("previous keys = {:?}, new keys = {:?}", chip8.keypad.current_frame_keys, new_frame_keys);
        chip8.keypad.update_keys(new_frame_keys);

        // Tick emulator
        for _ in 1..=16 {
            chip8.tick();
        }

        // Draw screen if needed
        if chip8.should_redraw {
            // Clear screen
            canvas.set_draw_color(base_mocha);
            canvas.clear();

            // Make draw loop
            canvas.set_draw_color(yellow_mocha);
            chip8
                .screen
                .into_iter()
                .enumerate()
                .filter(|(_, is_on)| *is_on)
                .for_each(|(index, _)| {
                    let rect = get_rect_dimensions_from_index(index);
                    canvas.fill_rect(rect).unwrap();
                });
            chip8.should_redraw = false;
        }
        
        // Present canvas
        canvas.present();

        // Sleep
        std::thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
    }
}

fn get_rect_dimensions_from_index(index: usize) -> Rect {
    let (i, j) = point_from_index(index);

    Rect::new(
        j as i32 * SQUARE_SIZE as i32,
        i as i32 * SQUARE_SIZE as i32,
        SQUARE_SIZE, 
        SQUARE_SIZE
    )
}

fn get_keypad_button_from_scancode(scancode: Scancode) -> Option<usize> {
    let key = match scancode {
        Scancode::Num1 => 0x1,
        Scancode::Num2 => 0x2,
        Scancode::Num3 => 0x3,
        Scancode::Num4 => 0xC,
        Scancode::Q => 0x4,
        Scancode::W => 0x5,
        Scancode::E => 0x6,
        Scancode::R => 0xD,
        Scancode::A => 0x7,
        Scancode::S => 0x8,
        Scancode::D => 0x9,
        Scancode::F => 0xE,
        Scancode::Z => 0xA,
        Scancode::X => 0x0,
        Scancode::C => 0xB,
        Scancode::V => 0xF,
        _ => return None,
    };
    Some(key)
}
