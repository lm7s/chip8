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

const CATPPUCCIN_MOCHA_BASE: Color = Color::RGB(30, 30, 46);
const CATPPUCCIN_MOCHA_YELLOW: Color = Color::RGB(249, 226, 175);

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("CHIP-8 Emulator", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    
    canvas.set_draw_color(CATPPUCCIN_MOCHA_BASE);
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut chip8 = Chip8::new();
    let instructions_per_frame = 5;
    let rom =
            // std::fs::read("/mnt/Demoiselle/游戏/ROMs/CHIP-8/games/Pong (1 player).ch8").unwrap();
            std::fs::read("./ROMs/test/5-quirks.ch8").unwrap();
    chip8.load_rom(&rom);
    'running: loop {
        // Parse events
        let mut new_frame_keys = chip8.keypad.current_frame_keys;
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
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(key) = get_keypad_button_from_scancode(scancode) {
                        new_frame_keys[key] = false;
                    }
                }
                _ => {}
            }
        }

        // Update keys
        chip8.keypad.update_keys(new_frame_keys);

        
        // Tick emulator
        for _ in 0..instructions_per_frame {
            chip8.tick();
        }

        // Draw screen if needed
        if chip8.should_redraw {
            // Clear screen
            canvas.set_draw_color(CATPPUCCIN_MOCHA_BASE);
            canvas.clear();

            // Draw pixels
            canvas.set_draw_color(CATPPUCCIN_MOCHA_YELLOW);
            chip8
                .screen
                .into_iter()
                .enumerate()
                .filter(|(_, is_on)| *is_on)
                .for_each(|(index, _)| {
                    let rect = get_rect_dimensions_from_index(index);
                    canvas.fill_rect(rect).unwrap();
                });

            // Don't draw again until requested 
            chip8.should_redraw = false;
        }
        
        // Present canvas
        canvas.present();

        // Sleep
        std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0));
    };
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
