use std::cmp;

use arrayvec::ArrayVec;
use rand::Rng;

pub const PIXELS_PER_ROW: usize = 64;
pub const PIXELS_PER_COLUMN: usize = 32;
pub const PIXELS_PER_SCREEN: usize = PIXELS_PER_COLUMN * PIXELS_PER_ROW;
pub const STACK_SIZE: usize = 16;
pub const RAM_SIZE: usize = 4_096;
pub const ROM_INITIAL_POSITION: usize = 0x200;
pub const FONT_INITIAL_POSITION: usize = 0x50;

const FONT_SET: &[u8] = &[
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Keypad {
    pub previous_frame_keys: [bool; 16],
    pub current_frame_keys: [bool; 16],
}

impl Keypad {
    pub fn update_keys(&mut self, current_frame_keys: [bool; 16]) {
        self.previous_frame_keys = self.current_frame_keys;
        self.current_frame_keys = current_frame_keys;
    }

    fn first_released_keypress(&self) -> Option<usize> {
        self.previous_frame_keys
            .into_iter()
            .zip(self.current_frame_keys.into_iter())
            .position(|(was_pressed, is_pressed)| was_pressed && !is_pressed)
    }

    fn first_pressed_keypress(&self) -> Option<usize> {
        self.current_frame_keys
            .into_iter()
            .position(|is_pressed| is_pressed)
    }
}

pub struct Chip8 {
    memory: [u8; RAM_SIZE],
    pub screen: [bool; PIXELS_PER_SCREEN],
    /// Program counter; the current instruction in memory
    pc: u16,
    /// Index register
    i: u16,
    stack: ArrayVec<u16, STACK_SIZE>,
    delay_timer: u8,
    sound_timer: u8,
    v: [u8; 16],
    pub should_redraw: bool,
    pub keypad: Keypad,
}

enum Platforms {
    CosmacVip,
    Amiga,
}

enum NextInstruction {
    Next,
    Skip,
    Jump(u16),
    Stay,
}

impl NextInstruction {
    pub fn skip_if(condition: bool) -> Self {
        if condition {
            NextInstruction::Skip
        } else {
            NextInstruction::Next
        }
    }
}

impl Chip8 {
    pub fn new() -> Self {
        let memory = {
            let mut memory = [0; RAM_SIZE];
            // write the font
            memory[FONT_INITIAL_POSITION..FONT_INITIAL_POSITION + FONT_SET.len()].copy_from_slice(FONT_SET);
            memory
        };
        Self {
            memory,
            screen: [false; PIXELS_PER_SCREEN],
            pc: 0x200,
            i: 0,
            stack: ArrayVec::new(),
            delay_timer: 0,
            sound_timer: 0,
            v: [0; 16],
            should_redraw: false,
            keypad: Keypad::default(),
        }
    }

    pub fn load_rom(&mut self, rom: &'_ [u8]) {
        let start = 0x200;
        let end = 0x200 + rom.len();
        self.memory[start..end].copy_from_slice(rom);
    }

    pub fn tick(&mut self) {
        // fetch instruction from memory
        let pc = self.pc as usize;
        let instruction = u16::from_be_bytes([self.memory[pc], self.memory[pc + 1]]);
        // decode instruction
        let nibbles = decode_instruction_into_nibbles(instruction);
        let (x, y, n) = {
            let [_, x, y, n] = nibbles;
            (x as usize, y as usize, n)
        };
        let nn = (instruction & 0x00FF) as u8;
        let nnn = instruction & 0x0FFF;

        self.pc += 2;
        // execute instruction
        let next_instruction = match nibbles {
            [0x0, 0x0, 0xE, 0x0] => self.execute_00e0(),
            [0x0, 0x0, 0xE, 0xE] => self.execute_00ee(),
            [0x1, _, _, _] => self.execute_1nnn(nnn),
            [0x2, _, _, _] => self.execute_2nnn(nnn),
            [0x3, _, _, _] => self.execute_3xnn(x, nn),
            [0x4, _, _, _] => self.execute_4xnn(x, nn),
            [0x5, _, _, 0x0] => self.execute_5xy0(x, y),
            [0x6, _, _, _] => self.execute_6xnn(x, nn),
            [0x7, _, _, _] => self.execute_7xnn(x, nn),
            [0x8, _, _, 0x0] => self.execute_8xy0(x, y),
            [0x8, _, _, 0x1] => self.execute_8xy1(x, y),
            [0x8, _, _, 0x2] => self.execute_8xy2(x, y),
            [0x8, _, _, 0x3] => self.execute_8xy3(x, y),
            [0x8, _, _, 0x4] => self.execute_8xy4(x, y),
            [0x8, _, _, 0x5] => self.execute_8xy5(x, y),
            [0x8, _, _, 0x6] => self.execute_8xy6(x, y),
            [0x8, _, _, 0x7] => self.execute_8xy7(x, y),
            [0x8, _, _, 0xE] => self.execute_8xye(x, y),
            [0xA, _, _, _] => self.execute_annn(nnn),
            [0xB, _, _, _] => self.execute_bnnn(nnn),
            [0xC, _, _, _] => self.execute_cxnn(x, nn),
            [0xD, _, _, _] => self.execute_dxyn(x, y, n),
            [0xE, _, 0x9, 0xE] => self.execute_ex9e(x),
            [0xE, _, 0xA, 0x1] => self.execute_exa1(x),
            [0xF, _, 0x0, 0x7] => self.execute_fx07(x),
            [0xF, _, 0x1, 0x5] => self.execute_fx15(x),
            [0xF, _, 0x1, 0x8] => self.execute_fx18(x),
            [0xF, _, 0x1, 0xE] => self.execute_fx1e(x),
            [0xF, _, 0x0, 0xA] => self.execute_fx0a(x),
            [0xF, _, 0x2, 0x9] => self.execute_fx29(x),
            [0xF, _, 0x3, 0x3] => self.execute_fx33(x),
            [0xF, _, 0x5, 0x5] => self.execute_fx55(x),
            [0xF, _, 0x6, 0x5] => self.execute_fx65(x),
            [0x9, _, _, 0x0] => self.execute_9xy0(x, y),
            _ => todo!(),
        };

        self.pc = match next_instruction {
            NextInstruction::Next => self.pc,
            NextInstruction::Skip => self.pc + 2,
            NextInstruction::Jump(addr) => addr,
            NextInstruction::Stay => self.pc - 2,
        }
    }

    // 00E0 - Clear screen
    fn execute_00e0(&mut self) -> NextInstruction {
        self.screen = [false; PIXELS_PER_SCREEN];
        self.should_redraw = true;
        NextInstruction::Next
    }

    fn execute_00ee(&mut self) -> NextInstruction {
        NextInstruction::Jump(self.stack.pop().unwrap())
    }

    // 1NNN - Jump
    fn execute_1nnn(&mut self, nnn: u16) -> NextInstruction {
        NextInstruction::Jump(nnn)
    }

    fn execute_2nnn(&mut self, nnn: u16) -> NextInstruction {
        self.stack.push(self.pc);
        NextInstruction::Jump(nnn)
    }

    fn execute_3xnn(&mut self, x: usize, nn: u8) -> NextInstruction {
        NextInstruction::skip_if(self.v[x] == nn)
    }

    fn execute_4xnn(&mut self, x: usize, nn: u8) -> NextInstruction {
        NextInstruction::skip_if(self.v[x] != nn)
    }

    fn execute_5xy0(&mut self, x: usize, y: usize) -> NextInstruction {
        NextInstruction::skip_if(self.v[x] == self.v[y])
    }

    // 6XNN - Set register VX
    fn execute_6xnn(&mut self, x: usize, nn: u8) -> NextInstruction {
        self.v[x] = nn;
        NextInstruction::Next
    }

    // 7XNN - Add to register VX
    fn execute_7xnn(&mut self, x: usize, nn: u8) -> NextInstruction {
        self.v[x] = self.v[x].wrapping_add(nn);
        NextInstruction::Next
    }

    fn execute_8xy0(&mut self, x: usize, y: usize) -> NextInstruction {
        self.v[x] = self.v[y];
        NextInstruction::Next
    }

    fn execute_8xy1(&mut self, x: usize, y: usize) -> NextInstruction {
        self.v[x] |= self.v[y];
        self.v[0xF] = 0;
        NextInstruction::Next
    }

    fn execute_8xy2(&mut self, x: usize, y: usize) -> NextInstruction {
        self.v[x] &= self.v[y];
        self.v[0xF] = 0;
        NextInstruction::Next
    }

    fn execute_8xy3(&mut self, x: usize, y: usize) -> NextInstruction {
        self.v[x] ^= self.v[y];
        self.v[0xF] = 0;
        NextInstruction::Next
    }

    fn execute_8xy4(&mut self, x: usize, y: usize) -> NextInstruction {
        let (result, overflowed) = self.v[x].overflowing_add(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = if overflowed { 1 } else { 0 };
        NextInstruction::Next
    }

    fn execute_8xy5(&mut self, x: usize, y: usize) -> NextInstruction {
        let (result, underflowed) = self.v[x].overflowing_sub(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = if underflowed { 0 } else { 1 };
        NextInstruction::Next
    }

    fn execute_8xy6(&mut self, x: usize, y: usize) -> NextInstruction {
        // Put the value of VY into VX
        // Shift VX 1 bit to the right
        // Set VF to the bit that was shifted out
        self.v[x] = self.v[y];
        let rotated_bit = self.v[x] & 0x1;
        self.v[x] >>= 1;
        self.v[0xF] = rotated_bit;
        NextInstruction::Next
    }

    fn execute_8xy7(&mut self, x: usize, y: usize) -> NextInstruction {
        let (result, underflowed) = self.v[y].overflowing_sub(self.v[x]);
        self.v[x] = result;
        self.v[0xF] = if underflowed { 0 } else { 1 };
        NextInstruction::Next
    }

    fn execute_8xye(&mut self, x: usize, y: usize) -> NextInstruction {
        self.v[x] = self.v[y];
        let rotated_bit = (self.v[x] >> 7) & 0b1;
        self.v[x] <<= 1;
        self.v[0xF] = rotated_bit;
        NextInstruction::Next
    }

    // ANNN - Set index register I
    fn execute_annn(&mut self, nnn: u16) -> NextInstruction {
        self.i = nnn;
        NextInstruction::Next
    }

    fn execute_bnnn(&mut self, nnn: u16) -> NextInstruction {
        NextInstruction::Jump(nnn + self.v[0x0] as u16)
    }

    fn execute_cxnn(&mut self, x: usize, nn: u8) -> NextInstruction {
        let random: u8 = rand::thread_rng().gen();
        self.v[x] = random & nn;
        NextInstruction::Next
    }

    // DXYN - Display and draw
    fn execute_dxyn(&mut self, x: usize, y: usize, n: u8) -> NextInstruction {
        // get X and Y coordinates
        println!("x = {}, y = {}, n = {}", x, y, n);
        let i = (self.v[y] % 32) as usize;
        let j = (self.v[x] % 64) as usize;
        self.v[0xF] = 0;

        println!("i = {}, j = {}", i, j);

        let end_downwards = cmp::min(i + n as usize, 32);
        let end_to_right = cmp::min(j + 8, 64);

        println!("end_downwards = {}, end_to_right = {}", end_downwards, end_to_right);

        for (column_iter, column_index) in (i..end_downwards).enumerate() {
            let sprite_byte = self.memory[self.i as usize + column_iter];
            for (row_iter, row_index) in (j..end_to_right).enumerate() {
                let sprite_pixel = (sprite_byte >> (7 - row_iter)) & 0b1;
                let pixel_index = column_index * PIXELS_PER_ROW + row_index;
                let screen_pixel = self.screen[pixel_index];
                if sprite_pixel == 1 {
                    if screen_pixel == true {
                        self.v[0xF] = 1;
                    }
                    self.screen[pixel_index] ^= true;
                }
            }
        }

        self.should_redraw = true;
        NextInstruction::Next
    }

    fn execute_ex9e(&mut self, x: usize) -> NextInstruction {
        NextInstruction::skip_if(self.keypad.current_frame_keys[self.v[x] as usize])
    }

    fn execute_exa1(&mut self, x: usize) -> NextInstruction {
        NextInstruction::skip_if(!self.keypad.current_frame_keys[self.v[x] as usize])
    }

    fn execute_fx07(&mut self, x: usize) -> NextInstruction {
        self.v[x] = self.delay_timer;
        NextInstruction::Next
    }

    fn execute_fx15(&mut self, x: usize) -> NextInstruction {
        self.delay_timer = self.v[x];
        NextInstruction::Next
    }

    fn execute_fx18(&mut self, x: usize) -> NextInstruction {
        self.sound_timer = self.v[x];
        NextInstruction::Next
    }

    // TODO: implement altering VF on overflow above 0FFF
    fn execute_fx1e(&mut self, x: usize) -> NextInstruction {
        self.i += self.v[x] as u16;
        NextInstruction::Next
    }

    fn execute_fx0a(&mut self, x: usize) -> NextInstruction {
        if let Some(key) = self.keypad.first_pressed_keypress() {
            self.v[x] = key as u8;
            NextInstruction::Next
        } else {
            NextInstruction::Stay
        }
    }

    fn execute_fx29(&mut self, x: usize) -> NextInstruction {
        let vx = self.v[x];
        let offset = vx * 5;
        self.i = FONT_INITIAL_POSITION as u16 + offset as u16;
        NextInstruction::Next
    }

    fn execute_fx33(&mut self, x: usize) -> NextInstruction {
        let numbers = convert_to_binary_coded_decimal(self.v[x]);

        // set
        let i = self.i as usize;
        self.memory[i..i + 3].copy_from_slice(&numbers);
        self.i = self.i + x as u16 + 1;
        NextInstruction::Next
    }

    fn execute_fx65(&mut self, x: usize) -> NextInstruction {
        let i = self.i as usize;
        let memory_range = i..i + x + 1;
        self.v[0..=x].copy_from_slice(&self.memory[memory_range]);
        self.i = self.i + x as u16 + 1;
        NextInstruction::Next
    }

    // Store V0 to VX (inclusive) in memory
    fn execute_fx55(&mut self, x: usize) -> NextInstruction {
        let i = self.i as usize;
        let memory_range = i..i + x + 1;
        self.memory[memory_range].copy_from_slice(&self.v[0..=x]);
        self.i = self.i + x as u16 + 1;
        NextInstruction::Next
    }

    fn execute_9xy0(&mut self, x: usize, y: usize) -> NextInstruction {
        NextInstruction::skip_if(self.v[x] != self.v[y])
    }
}

pub fn decode_instruction_into_nibbles(instruction: u16) -> [u8; 4] {
    [
        ((instruction & 0xF000) >> 12) as u8,
        ((instruction & 0x0F00) >> 8) as u8,
        ((instruction & 0x00F0) >> 4) as u8,
        (instruction & 0x000F) as u8,
    ]
}

pub fn convert_to_binary_coded_decimal(num: u8) -> [u8; 3] {
    let units = num % 10;
    let hundreds = (num - units) / 100;
    let decimals = (num - (hundreds * 100) - units) / 10;

    [hundreds, decimals, units]
}

pub fn point_from_index(index: usize) -> (usize, usize) {
    (index / PIXELS_PER_ROW, index % PIXELS_PER_ROW)
}

pub fn index_from_point((i, j): (usize, usize)) -> usize {
    i * PIXELS_PER_ROW + j
}

// write programs at 0x200

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_nibbles_are_correctly_decoded() {
        let test_cases = [
            (0x00E0, [0x0, 0x0, 0xE, 0x0]),
            (0x1234, [0x1, 0x2, 0x3, 0x4]),
            (0x6A17, [0x6, 0xA, 0x1, 0x7]),
        ];

        for (instruction, expected_result) in test_cases {
            assert_eq!(
                decode_instruction_into_nibbles(instruction),
                expected_result
            );
        }
    }

    #[test]
    fn numbers_are_correctly_converted_to_binary_coded_decimal() {
        let test_cases = [
            (0, [0, 0, 0]),
            (1, [0, 0, 1]),
            (14, [0, 1, 4]),
            (67, [0, 6, 7]),
            (146, [1, 4, 6]),
            (249, [2, 4, 9]),
            (255, [2, 5, 5]),
        ];

        for (test_case, expected_result) in test_cases {
            assert_eq!(convert_to_binary_coded_decimal(test_case), expected_result);
        }
    }

    #[test]
    fn point_is_correctly_converted_to_index() {
        let test_cases = [(0, (0, 0)), (1, (0, 1)), (66, (1, 2)), (2047, (31, 63))];

        for (expected_result, test_case) in test_cases {
            assert_eq!(index_from_point(test_case), expected_result);
        }
    }

    #[test]
    fn index_is_correctly_converted_to_point() {
        let test_cases = [(0, (0, 0)), (1, (0, 1)), (66, (1, 2)), (2047, (31, 63))];

        for (test_case, expected_result) in test_cases {
            assert_eq!(point_from_index(test_case), expected_result);
        }
    }
}