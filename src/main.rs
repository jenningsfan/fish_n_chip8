use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::{self, SoundSource, Source};
use ggez::conf::WindowSetup;
use ggez::event::{self, EventHandler};
use ggez::glam::Vec2;
use ggez::graphics::{self, Color, DrawParam, Image, InstanceArray};
use ggez::input::keyboard::{KeyCode, KeyboardContext, KeyInput};

use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const PIXEL_SIZE: f32 = 16.0;
const SCREEN_SIZE: (f32, f32) = (WIDTH as f32 * PIXEL_SIZE, HEIGHT as f32 * PIXEL_SIZE);

const RAM_SIZE: usize = 4096;

const FONT_DATA: [u8; 5 * 16] = [
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

const FONT_START: usize = 0x50;
const FONT_END: usize = FONT_START + FONT_DATA.len();

fn main() {
    let args: Vec<String> = env::args().collect();

    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let (mut ctx, event_loop) = ContextBuilder::new("fish_n_chip8", "jenningsfan")
        .window_setup(WindowSetup::default().title("Fish n CHIP-8"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .add_resource_path(resource_dir)
        .build()
        .expect("Failed to create game context");

    let mut game = Game::new(&mut ctx);
    game.load_rom(args[1].as_str()); // TODO: Error Handling

    event::run(ctx, event_loop, game);
}

struct Game {
    pixels: Vec<Vec<bool>>,
    pixels_batch: InstanceArray,
    pixels_dirty: bool,
    delay_timer: u8,
    sound_timer: u8,
    beep_sound: Source,
    pressed_key: Option<u8>,
    ignore_keys: HashSet<u8>,
    waiting_for_key_press: bool,
    memory: Vec<u8>,
    stack: Vec<u16>,
    regs: [u8; 16],
    addr_reg: u16,
    pc: u16,
    rng: ThreadRng,
}

impl Game {
    pub fn new(ctx: &mut Context) -> Game {
        let pixel_rect = Image::from_color(
            &ctx.gfx,
            PIXEL_SIZE as u32,
            PIXEL_SIZE as u32,
            Some(Color::WHITE),
        );
        let pixels_batch = InstanceArray::new(&ctx.gfx, pixel_rect);

        let mut created = Game {
            pixels: vec![vec![true; WIDTH]; HEIGHT],
            pixels_batch,
            pixels_dirty: false,
            delay_timer: 0,
            sound_timer: 0,
            beep_sound: audio::Source::new(ctx, "/beep.wav").unwrap(),
            pressed_key: None,
            waiting_for_key_press: false,
            ignore_keys: HashSet::new(),
            memory: vec![0; RAM_SIZE],
            stack: vec![],
            regs: [0; 16],
            addr_reg: 0,
            pc: 0x200,
            rng: thread_rng(),
        };

        created.memory[FONT_START..FONT_END].copy_from_slice(&FONT_DATA);
        created.beep_sound.set_repeat(true);

        created
    }

    fn load_rom(&mut self, path: &str) {
        let rom = fs::read(path).unwrap(); // TODO: Error Handling
        self.memory[0x200..0x200 + rom.len()].copy_from_slice(&rom);
    }

    fn load_opcode(&self, addr: u16) -> u16 {
        (self.memory[addr as usize] as u16) << 8 | (self.memory[addr as usize + 1] as u16)
    }

    fn handle_opcode(&mut self, key_ctx: &KeyboardContext) {
        let opcode = self.load_opcode(self.pc);
        let opcode_type = (opcode & 0xF000) >> 12;        // TAAA
        let reg_x = (opcode as usize & 0x0F00) >> 8;    // AXAA
        let reg_y = (opcode as usize & 0x00F0) >> 4;    // AAYA
        let nnn = opcode & 0x0FFF;                        // ANNN
        let nn = (opcode & 0x00FF) as u8;                         // AANN
        let n = (opcode & 0x000F) as u8;                          // AAAN

        self.pc += 2;

        match opcode_type {
            0x0 => {
                match opcode {
                    0x00E0 => {
                        // 00E0 - clear screen
                        self.pixels = vec![vec![false; WIDTH]; HEIGHT];
                        self.pixels_dirty = true;
                    }
                    0x00EE => self.pc = {
                        // 00EE - return from a subroutine
                        self.stack.pop().expect("Stack should not be empty")
                    },
                    unsopported => panic!("Unsopported opcode {:#06x}", unsopported),
                }
            }
            0x1 => {
                // 1NNN - Jumps to address NNN
                self.pc = nnn;
            }
            0x2 => {
                // 2NNN - call subroutine
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            0x3 => {
                // 3XNN - skip next instruction if VX == NN
                if self.regs[reg_x] == nn {
                    self.pc += 2;
                }
            }
            0x4 => {
                // 4XNN - skip next instruction if VX != NN
                if self.regs[reg_x] != nn {
                    self.pc += 2;
                }
            }
            0x5 => {
                // 5XY0 - skip next instruction if VX == VY
                if self.regs[reg_x] == self.regs[reg_y] {
                    self.pc += 2;
                }
            }
            0x6 => {
                // 6XNN - sets VX to NN
                self.regs[reg_x] = nn;
            }
            0x7 => {
                // 7XNN - VX += NN
                self.regs[reg_x] = (self.regs[reg_x]).wrapping_add(nn);
            }
            0x8 => {
                // 8XYO - perform operation - on VX and VY
                let reg_y = self.regs[reg_y];
                let reg_x = &mut self.regs[reg_x];

                match n {
                    // 8XY0 - 8XY3 are fairly self-explanatory
                    0x0 => *reg_x = reg_y,
                    0x1 => *reg_x |= reg_y,
                    0x2 => *reg_x &= reg_y,
                    0x3 => *reg_x ^= reg_y,
                    0x4 => {
                        // 8XY4 - VX += VY. VF is set to 1 if overflow happened. only lower 8 bits are kept
                        let result = reg_x.wrapping_add(reg_y);
                        let overflow = reg_x.overflowing_add(reg_y).1;
                        *reg_x = result;
                        self.regs[15] = if overflow { 1 } else { 0 };
                    }
                    0x5 => {
                        // 8XY5 - VX -= VY. VF is set to 0 if underflow happened. only lower 8 bits are kept
                        let before_sub = *reg_x;
                        let result = reg_x.wrapping_sub(reg_y);
                        *reg_x = result;
                        self.regs[15] = if before_sub >= reg_y { 1 } else { 0 };
                    }
                    0x6 => {
                        // 8XY6 - VX >>= 1. VF is set to LSB of VX before shift
                        let before_shift = *reg_x;
                        *reg_x >>= 1;
                        self.regs[15] = before_shift & 1;
                    }
                    0x7 => {
                        // 8XY7 - VX = VY - VX. VF is set to 0 if underflow happened. only lower 8 bits are kept
                        let before_add = *reg_x;
                        let result = reg_y.wrapping_sub(*reg_x);
                        *reg_x = result;
                        self.regs[15] = if reg_y >= before_add { 1 } else { 0 };
                    }
                    0xE => {
                        // 8XYE - VX <<= 1. VF is set to MSB of VX before shift
                        let before_shift = *reg_x;
                        *reg_x <<= 1;
                        self.regs[15] = (before_shift & 0b1000_0000) >> 7;
                    }
                    _ => panic!("Unsopported opcode {:#06x}", opcode),
                };
            }
            0x9 => {
                // 9XY0 - skip next instruction if VX != VY
                if self.regs[reg_x] != self.regs[reg_y] {
                    self.pc += 2;
                }
            }
            0xA => self.addr_reg = nnn, // ANNN - sets I to NNN
            0xB => self.pc = self.regs[0] as u16 + nnn, // BXNN jump to NNN + V0
            0xC => {
                // CXNN - VX = rand & NN; rand 0-255
                self.regs[reg_x] = self.rng.gen::<u8>() & nn;
            }
            0xD => {
                // DXYN - Draw sprit to coord (VX, VY) - width 8 pixels, height N pixels.
                //        Read from memory location I. VF set to 1 if any pixels erased
                let col = self.regs[reg_x] as usize;
                let row = self.regs[reg_y] as usize;
                let rows = n;
                let sprite = &self.memory[self.addr_reg as usize..(self.addr_reg + rows as u16) as usize];
                self.regs[15] = 0;

                for (row_i, sprite_row) in sprite.iter().enumerate() {
                    for col_i in 0..8 {
                        if col_i + col >= WIDTH || row_i + row >= HEIGHT {
                            break;
                        }

                        let sprite_pixel = (*sprite_row & (1 << (7 - col_i))) == 1 << (7 - col_i); // the 7 - col_i is to make the sprite_row be read in the correct direction
                        let screen_pixel = self.pixels[row_i + row][col_i + col];

                        if sprite_pixel != screen_pixel {
                            self.pixels[row_i + row][col_i + col] = true;
                            self.pixels_dirty = true;
                        } else {
                            self.pixels[row_i + row][col_i + col] = false;
                            self.pixels_dirty = true;
                        }

                        // if gone from set to unset then set VF to 1
                        if screen_pixel == true && self.pixels[row_i + row][col_i + col] == false {
                            self.regs[15] = 1;
                        }
                    }
                }
            }
            0xE => {
                match opcode & 0x00FF {
                    0x9E => {
                        // EX9E - skip next instruction if key in VX pressed
                        if self.is_key_pressed(key_ctx, self.regs[reg_x]) {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        // EXA1 - skip next instruction if key in VX not pressed
                        if !self.is_key_pressed(key_ctx, self.regs[reg_x]) {
                            self.pc += 2;
                        }
                    }
                    _ => panic!("Unsopported opcode {:#06x}", opcode),
                }
            }
            0xF => {
                match nn {
                    0x07 => {
                        // FX07 - Sets VX to delay time
                        self.regs[reg_x] = self.delay_timer;
                    }
                    0x0A => {
                        // FX0A - Get key. Blocking instruction. Waits for key input and then puts it in VX. However, timers should still decrement
                        if self.pressed_key == None {
                            if !self.waiting_for_key_press {
                                self.ignore_keys = self.get_key_input(&key_ctx);
                                self.waiting_for_key_press = true;
                            }

                            self.pc -= 2;
                        }
                        else if let Some(key) = self.pressed_key {
                            self.regs[reg_x] = key;
                            self.ignore_keys = HashSet::new();
                            self.pressed_key = None;
                            self.waiting_for_key_press = false;
                        }
                    }
                    0x15 => {
                        // FX15 - Delay timer = VX
                        self.delay_timer = self.regs[reg_x];
                    }
                    0x18 => {
                        // FX18 - Sound timer = VX
                        self.sound_timer = self.regs[reg_x];
                    }
                    0x1E => {
                        // FX1E - I += VX. VF not affected
                        self.addr_reg += self.regs[reg_x] as u16;
                    }
                    0x29 => {
                        // FX29 - I = addr of hex character in VX
                        let reg = self.regs[reg_x] as u16;
                        self.addr_reg = FONT_START as u16 + reg * 5;
                    }
                    0x33 => {
                        // FX33 - Store BCD of VX in I. I is hundreds. I + 1 tens. I + 2 units.
                        let mut bcd: u32 = self.regs[reg_x] as u32;

                        for _ in 0..8 {
                            if bcd & 0x00F00 >= 0x00500 {
                                bcd += 0x00300;
                            }
                            if bcd & 0x0F000 >= 0x05000 {
                                bcd += 0x03000;
                            }
                            if bcd & 0xF0000 >= 0x50000 {
                                bcd += 0x30000;
                            }
                            bcd <<= 1;
                        }

                        self.memory[self.addr_reg as usize] = ((bcd & 0xF0000) >> 16) as u8;
                        self.memory[self.addr_reg as usize + 1] = ((bcd & 0x0F000) >> 12) as u8;
                        self.memory[self.addr_reg as usize + 2] = ((bcd & 0x00F00) >> 8) as u8;
                    }
                    0x55 => {
                        // FX55 - Dump regs V0 - VX(inclusive) to I - I + X. I is unmodified
                        let total_regs = reg_x as u16 + 1;

                        for i in 0..total_regs {
                            self.memory[(self.addr_reg + i) as usize] = self.regs[(i) as usize];
                        }
                    }
                    0x65 => {
                        // FX65 - Load regs V0 - VX(inclusive) from I - I + X. I is unmodified
                        let total_regs = reg_x as u16 + 1;

                        for i in 0..total_regs {
                            self.regs[i as usize] = self.memory[(self.addr_reg + i) as usize];
                        }
                    }
                    _ => panic!("Unsopported opcode {:#06x}", opcode),
                }
            }
            _ => panic!("should only be a nibble"),
        };
    }

    fn is_key_pressed(&self, key_ctx: &KeyboardContext, key: u8) -> bool {
        let keycode = match key {
            0x1 => KeyCode::Key1,
            0x2 => KeyCode::Key2,
            0x3 => KeyCode::Key3,
            0xC => KeyCode::Key4,
            0x4 => KeyCode::Q,
            0x5 => KeyCode::W,
            0x6 => KeyCode::E,
            0xD => KeyCode::R,
            0x7 => KeyCode::A,
            0x8 => KeyCode::S,
            0x9 => KeyCode::D,
            0xE => KeyCode::F,
            0xA => KeyCode::Z,
            0x0 => KeyCode::X,
            0xB => KeyCode::C,
            0xF => KeyCode::V,
            unknown => panic!("{unknown} is not a valid CHIP-8 key"),
        };
        key_ctx.is_key_pressed(keycode)
    }

    fn key_for_keycode(&self, keycode: &KeyCode) -> Option<u8> {
        match *keycode {
            KeyCode::Key1 => return Some(0x1),
            KeyCode::Key2 => return Some(0x2),
            KeyCode::Key3 => return Some(0x3),
            KeyCode::Key4 => return Some(0xC),
            KeyCode::Q => return Some(0x4),
            KeyCode::W => return Some(0x5),
            KeyCode::E => return Some(0x6),
            KeyCode::R => return Some(0xD),
            KeyCode::A => return Some(0x7),
            KeyCode::S => return Some(0x8),
            KeyCode::D => return Some(0x9),
            KeyCode::F => return Some(0xE),
            KeyCode::Z => return Some(0xA),
            KeyCode::X => return Some(0x0),
            KeyCode::C => return Some(0xB),
            KeyCode::V => return Some(0xF),
            _ => return None,
        };
    }

    fn get_key_input(&self, key_ctx: &KeyboardContext) -> HashSet<u8> {
        let pressed = key_ctx.pressed_keys();
        let mut pressed_nums: HashSet<u8> = HashSet::new();
        
        for key in pressed {
            if let Some(key) = self.key_for_keycode(key) {
                pressed_nums.insert(key);
            }
        }

        pressed_nums
    }
}

impl EventHandler for Game {
    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        if !self.waiting_for_key_press {
            return Ok(());
        }

        let key = self.key_for_keycode(&input.keycode.unwrap());

        if let Some(key) = key  {
            if !self.ignore_keys.remove(&key) {
                self.pressed_key = Some(key);
            }
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            if !self.beep_sound.playing() {
                self.beep_sound.play(&ctx.audio)?;
            }
        } else {
            self.beep_sound.pause();
        }

        for _ in 0..12 {
            self.handle_opcode(&ctx.keyboard);

            if ctx.time.ticks() % 100 == 0 {
                println!("Delta frame time: {:?} ", ctx.time.delta());
                println!("Average FPS: {}", ctx.time.fps());
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        if !self.pixels_dirty {
            return Ok(());
        }

        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        self.pixels_batch.clear();

        for (col_i, row) in self.pixels.iter().enumerate() {
            for (row_i, pixel) in row.iter().enumerate() {
                if *pixel {
                    self.pixels_batch.push(
                        DrawParam::new().dest(Vec2::new(
                            row_i as f32 * PIXEL_SIZE,
                            col_i as f32 * PIXEL_SIZE,
                        )),
                    );
                }
            }
        }

        self.pixels_dirty = false;

        canvas.draw(&self.pixels_batch, DrawParam::new());
        canvas.finish(ctx)
    }
}
