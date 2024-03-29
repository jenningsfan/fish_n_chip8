use std::collections::HashSet;

use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;

const LOW_RES_FONT: [u8; 5 * 16] = [
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

const HIGH_RES_FONT: [u8; 10 * 16] = [
    0xFF, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, // 0
    0x18, 0x78, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0xFF, 0xFF, // 1
    0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, // 2
    0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 3
    0xC3, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, 0x03, 0x03, 0x03, 0x03, // 4
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 5
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, // 6
    0xFF, 0xFF, 0x03, 0x03, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, // 7
    0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, // 8
    0xFF, 0xFF, 0xC3, 0xC3, 0xFF, 0xFF, 0x03, 0x03, 0xFF, 0xFF, // 9
    0x7E, 0xFF, 0xC3, 0xC3, 0xC3, 0xFF, 0xFF, 0xC3, 0xC3, 0xC3, // A
    0xFC, 0xFC, 0xC3, 0xC3, 0xFC, 0xFC, 0xC3, 0xC3, 0xFC, 0xFC, // B
    0x3C, 0xFF, 0xC3, 0xC0, 0xC0, 0xC0, 0xC0, 0xC3, 0xFF, 0x3C, // C
    0xFC, 0xFE, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xFE, 0xFC, // D
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, // E
    0xFF, 0xFF, 0xC0, 0xC0, 0xFF, 0xFF, 0xC0, 0xC0, 0xC0, 0xC0  // F
];

const HIGH_RES_FONT_START: usize = LOW_RES_FONT_END;
const HIGH_RES_FONT_END: usize = HIGH_RES_FONT_START + HIGH_RES_FONT.len();

const LOW_RES_FONT_START: usize = 0x50;
const LOW_RES_FONT_END: usize = LOW_RES_FONT_START + LOW_RES_FONT.len();

#[derive(PartialEq, Clone, Copy)]
pub enum RegSaveLoadQuirk {
    Unchanged,
    X,
    XPlusOne,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ShiftingReg {
    VX,
    VY,
}

#[derive(PartialEq, Clone, Copy)]
pub enum JumpBehviour {
    BNNN,
    BXNN,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ScrollingBehviour {
    Modern,
    Legacy,
}

#[derive(Clone, Copy)]
pub struct Quirks {
    pub vf_reset: bool,
    pub shifting: ShiftingReg,
    pub reg_save_load: RegSaveLoadQuirk,
    pub jump: JumpBehviour,
    pub screen_wrap: bool,
    pub scrolling: ScrollingBehviour,
}

impl Quirks {
    pub fn default() -> Self {
        Self {
            shifting: ShiftingReg::VX,
            vf_reset: false,
            reg_save_load: RegSaveLoadQuirk::Unchanged,
            jump: JumpBehviour::BNNN,
            screen_wrap: false,
            scrolling: ScrollingBehviour::Modern,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Resolution {
    HighRes,
    LowRes,
}

pub struct CPU {
    pub pixels: Vec<Vec<bool>>,
    pub resolution: Resolution,
    pub quirks: Quirks,
    memory: [u8; RAM_SIZE],
    delay_timer: u8,
    sound_timer: u8,
    pressed_key: Option<u8>,
    ignore_keys: HashSet<u8>,
    waiting_for_key_press: bool,
    stack: Vec<u16>,
    regs: [u8; 16],
    addr_reg: u16,
    pc: u16,
    rng: ThreadRng,
}

impl CPU {
    pub fn new() -> CPU {
        let mut created = Self {
            pixels: vec![vec![false; WIDTH]; HEIGHT],
            resolution: Resolution::LowRes,
            quirks: Quirks::default(),
            memory: [0; RAM_SIZE],
            delay_timer: 0,
            sound_timer: 0,
            pressed_key: None,
            ignore_keys: HashSet::new(),
            waiting_for_key_press: false,
            stack: vec![],
            regs: [0; 16],
            addr_reg: 0,
            pc: 0x200,
            rng: thread_rng(),
        };

        created.memory[LOW_RES_FONT_START..LOW_RES_FONT_END].copy_from_slice(&LOW_RES_FONT);
        created.memory[HIGH_RES_FONT_START..HIGH_RES_FONT_END].copy_from_slice(&HIGH_RES_FONT);

        created
    }

    pub fn load_rom(&mut self, rom: &Vec<u8>) {
        self.memory[0x200..0x200 + rom.len()].copy_from_slice(rom);
    }

    pub fn key_released(&mut self, key: u8) {
        if self.waiting_for_key_press && !self.ignore_keys.remove(&key) {
            self.pressed_key = Some(key);
        }
    }

    pub fn timer_tick(&mut self) -> bool{
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            return true;
        } else {
            return false;
        }
    }

    pub fn height(&self) -> usize {
        self.pixels.len()
    }

    pub fn width(&self) -> usize {
        self.pixels[0].len()
    }

    pub fn handle_opcode(&mut self, pressed_keys: &HashSet<u8>) {
        let opcode = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[self.pc as usize + 1] as u16);
        let opcode_type = (opcode & 0xF000) >> 12;      // TAAA
        let reg_x = (opcode as usize & 0x0F00) >> 8;    // AXAA
        let reg_y = (opcode as usize & 0x00F0) >> 4;    // AAYA
        let nnn = opcode & 0x0FFF;                      // ANNN
        let nn = (opcode & 0x00FF) as u8;               // AANN
        let n = (opcode & 0x000F) as u8;                // AAAN

        self.pc += 2;

        match opcode_type {
            0x0 => {
                if opcode & 0xFFF0 == 0x00C0 {
                    // 00CN: Scroll display N pixels down; in low resolution mode, N/2 pixels
                    self.pixels.remove(self.height() - 1);
                    self.pixels.remove(self.height() - 1);

                    for _ in 0..n {
                        self.pixels.insert(0, vec![false; self.width()]);
                    }
                }
                else {
                    match opcode {
                        0x00E0 => {
                            // 00E0 - clear screen
                            self.pixels = vec![vec![false; self.width()]; self.height()];
                        }
                        0x00EE => self.pc = {
                            // 00EE - return from a subroutine
                            self.stack.pop().expect("Stack should not be empty")
                        },
                        0x00FB => {
                            // 00FB - scroll right by 4 pixels in highres or 2 in lowres SUPERCHIP
                            for row in self.pixels.iter_mut() {
                                let mut new_row = vec![false, false, false, false];
                                new_row.append(&mut row[..row.len() - 4].to_vec());
                                *row = new_row;
                            }
                        },
                        0x00FC => {
                            // 00FC - scroll left by 4 pixels in highres or 2 in lowres SUPERCHIP
                            for row in self.pixels.iter_mut() {
                                let mut new_row = vec![false, false, false, false];
                                *row = row[4..].to_vec();
                                row.append(&mut new_row);
                            }
                        },
                        0x00FD => {
                            // 00FD - exit interperter SUPERCHIP
                            self.load_rom(&vec![0x12, 0x00]); // just go to infinte loop
                        },
                        0x00FE => {
                            // 00FE - enable lowres SUPERCHIP
                            self.pixels = vec![vec![false; WIDTH]; HEIGHT];
                            self.resolution = Resolution::LowRes;
                        },
                        0x00FF => {
                            // 00FF - enable highres SUPERCHIP
                            self.pixels = vec![vec![false; WIDTH * 2]; HEIGHT * 2];
                            self.resolution = Resolution::HighRes;
                        },
                        unsopported => panic!("Unsopported opcode {:#06x} at {:#06x}", unsopported, self.pc),
                    }
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
                if self.quirks.vf_reset {
                    self.regs[15] = 0;
                }

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
                        let mut reg = match self.quirks.shifting {
                            ShiftingReg::VX => *reg_x,
                            ShiftingReg::VY => reg_y,
                        };

                        let before_shift = reg;
                        reg >>= 1;
                        *reg_x = reg;
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
                        let mut reg = match self.quirks.shifting {
                            ShiftingReg::VX => *reg_x,
                            ShiftingReg::VY => reg_y,
                        };

                        let before_shift = reg;
                        reg <<= 1;
                        *reg_x = reg;
                        self.regs[15] = (before_shift & 0b1000_0000) >> 7;
                    }
                    _ => panic!("Unsopported opcode {:#06x} at {:#06x}", opcode, self.pc),
                };
            }
            0x9 => {
                // 9XY0 - skip next instruction if VX != VY
                if self.regs[reg_x] != self.regs[reg_y] {
                    self.pc += 2;
                }
            }
            0xA => self.addr_reg = nnn, // ANNN - sets I to NNN
            0xB => {
                // BNNN jump to NNN + V0
                // BXNN jump to XNN + VX
                match self.quirks.jump {
                    JumpBehviour::BNNN => self.pc = self.regs[0] as u16 + nnn,
                    JumpBehviour::BXNN => self.pc = self.regs[reg_x] as u16 + nnn as u16,
                }
            }
            0xC => {
                // CXNN - VX = rand & NN; rand 0-255
                self.regs[reg_x] = self.rng.gen::<u8>() & nn;
            }
            0xD => {
                // DXYN - Draw sprit to coord (VX, VY) - width 8 pixels, height N pixels.
                //        Read from memory location I. VF set to 1 if any pixels erased
                let start_col = self.regs[reg_x] as usize % self.width();
                let start_row = self.regs[reg_y] as usize % self.height();
                let rows = n;

                if rows == 0 {
                    let rows = 16;
                    let sprite: Vec<u16> = self.memory[self.addr_reg as usize..(self.addr_reg + rows * 2 as u16) as usize].to_vec()
                        .chunks_exact(2)
                        .into_iter()
                        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
                        .collect();
                    self.regs[15] = 0;
    
                    for (row, sprite_row) in sprite.iter().enumerate() {
                        let mut row = start_row + row;
                        if row > self.height() {
                            if self.quirks.screen_wrap {
                                row = row % self.height();
                            }
                            else {
                                break;
                            }
                        }
                        self.draw_sprite(start_col, row, (*sprite_row & 0xFF) as u8);
                        self.draw_sprite(start_col + 8, row, (*sprite_row >> 8) as u8);
                    }
                }
                else {
                    let sprite = &self.memory[self.addr_reg as usize..(self.addr_reg + rows as u16) as usize].to_vec();
                    self.regs[15] = 0;
    
                    for (row, sprite_row) in sprite.iter().enumerate() {
                        let mut row = start_row + row;
                        if row > self.height() {
                            if self.quirks.screen_wrap {
                                row = row % self.height();
                            }
                            else {
                                break;
                            }
                        }
                        self.draw_sprite(start_col, row, *sprite_row);
                    }
                } 
            }
            0xE => {
                match opcode & 0x00FF {
                    0x9E => {
                        // EX9E - skip next instruction if key in VX pressed
                        if pressed_keys.contains(&self.regs[reg_x]) {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        // EXA1 - skip next instruction if key in VX not pressed
                        if !pressed_keys.contains(&self.regs[reg_x]) {
                            self.pc += 2;
                        }
                    }
                    _ => panic!("Unsopported opcode {:#06x} at {:#06x}", opcode, self.pc),
                }
            }
            0xF => {
                match nn {
                    0x07 => {
                        // FX07 - Sets VX to delay time
                        self.regs[reg_x] = self.delay_timer;
                    },
                    0x0A => {
                        // FX0A - Get key. Blocking instruction. Waits for key input and then puts it in VX. However, timers should still decrement
                        if self.pressed_key == None {
                            if !self.waiting_for_key_press {
                                self.ignore_keys = pressed_keys.clone();
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
                    },
                    0x15 => {
                        // FX15 - Delay timer = VX
                        self.delay_timer = self.regs[reg_x];
                    },
                    0x18 => {
                        // FX18 - Sound timer = VX
                        self.sound_timer = self.regs[reg_x];
                    },
                    0x1E => {
                        // FX1E - I += VX. VF not affected
                        self.addr_reg += self.regs[reg_x] as u16;
                    },
                    0x29 => {
                        // FX29 - I = addr of hex character in VX
                        let reg = self.regs[reg_x] as u16;
                        self.addr_reg = LOW_RES_FONT_START as u16 + reg * 5;
                    },
                    0x30 => {
                        // FX29 - I = addr of hex character in VX
                        let reg = self.regs[reg_x] as u16;
                        self.addr_reg = HIGH_RES_FONT_START as u16 + reg * 10;
                    },
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
                    },
                    0x55 => {
                        // FX55 - Dump regs V0 - VX(inclusive) to I - I + X. I is unmodified
                        let total_regs = reg_x as u16 + 1;

                        for i in 0..total_regs {
                            self.memory[(self.addr_reg + i) as usize] = self.regs[(i) as usize];
                        }

                        match self.quirks.reg_save_load {
                            RegSaveLoadQuirk::Unchanged => {},
                            RegSaveLoadQuirk::X => self.addr_reg += total_regs,
                            RegSaveLoadQuirk::XPlusOne => self.addr_reg += total_regs + 1,
                        };
                    },
                    0x65 => {
                        // FX65 - Load regs V0 - VX(inclusive) from I - I + X. I is unmodified
                        let total_regs = reg_x as u16 + 1;

                        for i in 0..total_regs {
                            self.regs[i as usize] = self.memory[(self.addr_reg + i) as usize];
                        }

                        match self.quirks.reg_save_load {
                            RegSaveLoadQuirk::Unchanged => {},
                            RegSaveLoadQuirk::X => self.addr_reg += total_regs,
                            RegSaveLoadQuirk::XPlusOne => self.addr_reg += total_regs + 1,
                        };
                    },
                    0x75 => {},
                    0x85 => {},
                    _ => panic!("Unsopported opcode {:#06x} at {:#06x}", opcode, self.pc),
                }
            }
            _ => panic!("should only be a nibble"),
        };
    }

    fn draw_sprite(&mut self, start_col: usize, row: usize, sprite_row: u8) {
        for col_i in 0..8 {
            let mut col = col_i + start_col;

            if col >= self.width() {
                if self.quirks.screen_wrap {
                    col = col % self.width();
                }
                else {
                    break;
                }
            }

            let sprite_pixel = (sprite_row & (1 << (7 - col_i))) == 1 << (7 - col_i); // the 7 - col_i is to make the sprite_row be read in the correct direction
            let screen_pixel = self.pixels[row][col];
            
            if sprite_pixel != screen_pixel {
                self.pixels[row][col] = true;
            } else {
                self.pixels[row][col] = false;
            }

            // if gone from set to unset then set VF to 1
            if screen_pixel == true && self.pixels[row][col] == false {
                self.regs[15] = 1;
            }
        }

    }
}