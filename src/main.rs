use ggez::conf::WindowSetup;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::graphics::{self, Color, Quad, Rect, DrawParam, MeshBuilder, DrawMode, Mesh};
use ggez::event::{self, EventHandler};
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

// Now we define the pixel size of each tile, which we make 32x32 pixels.
const PIXEL_WIDTH: i32 = 16;
const PIXEL_HEIGHT: i32 = 16;
// Next we define how large we want our actual window to be by multiplying
// the components of our grid size by its corresponding pixel size.
const SCREEN_SIZE: (f32, f32) = (
    WIDTH as f32 * PIXEL_WIDTH as f32,
    HEIGHT as f32 * PIXEL_HEIGHT as f32,
);

const RAM_SIZE: usize = 4096;

fn main() {
    // Make a Context.
    let (mut ctx, event_loop) =
        ContextBuilder::new("fish_n_chip8", "jenningsfan")
        .window_setup(WindowSetup::default().title("Fish n CHIP-8"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .build()
        .expect("Failed to create game context");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.
    let my_game = Game::new(&mut ctx);

    // Run!
    event::run(ctx, event_loop, my_game);
}

struct Game {
    pixels: Vec<Vec<bool>>,
    memory: Vec<u8>,
    stack: Vec<u16>,
    regs: [u8; 16],
    addr_reg: u16,
    pc: u16,
    rng: ThreadRng,
}

impl Game {
    pub fn new(ctx: &mut Context) -> Game {
        Game {
            pixels: vec![vec![true; WIDTH]; HEIGHT],
            memory: vec![0; RAM_SIZE],
            stack: vec![],
            regs: [0; 16],
            addr_reg: 0,
            pc: 0x200,
            rng: thread_rng(),
        }
    }

    fn load_opcode(&self, addr: u16) -> u16 {
        (self.memory[addr as usize] as u16) << 16 | (self.memory[addr as usize + 1] as u16)
    }

    fn handle_opcode(&mut self) {
        let opcode = self.load_opcode(self.pc);
        self.pc += 2;

        match opcode & 0xF000 {
            0x0 => {
                match opcode {
                    0x00E0 => self.pixels = vec![vec![true; WIDTH]; HEIGHT], // clears the screen
                    0x00EE => self.pc = self.stack.pop().expect("Stack should not be empty"), // return from a subroutine
                    unsopported => panic!("Unsopported opcode {:#06x}", unsopported),
                }
            },
            0x1 => {
                // 1NNN - Jumps to address NNN
                self.pc = opcode & 0x0FFF;
            },
            0x2 => {
                // 2NNN - call subroutine
                let addr = opcode & 0x0FFF;
                self.pc = addr;
                self.stack.push(addr);
            },
            0x3 => {
                // 3XNN - skip next instruction if VX == NN
                let reg = self.regs[opcode as usize & 0x0F00];
                if reg as u16 == opcode & 0x00FF {
                    self.pc += 2;
                }
            },
            0x4 => {
                // 4XNN - skip next instruction if VX != NN
                let reg = self.regs[opcode as usize & 0x0F00];
                if reg as u16 != opcode & 0x00FF {
                    self.pc += 2;
                }
            },
            0x5 => {
                // 5XY0 - skip next instruction if VX == VY
                let reg_x = self.regs[opcode as usize & 0x0F00];
                let reg_y = self.regs[opcode as usize & 0x00F0];
                if reg_x == reg_y {
                    self.pc += 2;
                }
            },
            0x6 => {
                // 6XNN - sets VX to NN
                let reg = &mut self.regs[opcode as usize & 0x0F00];
                *reg = (opcode & 0x00FF) as u8;
            },
            0x7 => {
                // 7XNN - VX += NN
                let reg = &mut self.regs[opcode as usize & 0x0F00];
                *reg += (opcode & 0x00FF) as u8;
            },
            0x8 => {
                // 8XYO - perform operation - on VX and VY
                let reg_y = self.regs[opcode as usize & 0x00F0];
                let reg_x = &mut self.regs[opcode as usize & 0x0F00];
                
                match opcode & 0x000F {
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
                    },
                    0x5 => {
                        // 8XY5 - VX -= VY. VF is set to 0 if underflow happened. only lower 8 bits are kept
                        let result = reg_x.wrapping_sub(reg_y);
                        *reg_x = result;
                        self.regs[15] = if *reg_x > reg_y { 1 } else { 0 };
                    },
                    0x6 => {
                        // 8XY6 - VX >>= 1. VF is set to LSB of VX before shift
                        let before_shift = *reg_x;
                        *reg_x >>= 1;
                        self.regs[15] = before_shift & 1;
                    },
                    0x7 => {
                        // 8XY7 - VX = VY - VX. VF is set to 0 if underflow happened. only lower 8 bits are kept
                        let before_add = *reg_x;
                        let result = reg_y.wrapping_sub(*reg_x);
                        *reg_x = result;
                        self.regs[15] = if reg_y > before_add { 1 } else { 0 };
                    },
                    0xE => {
                        // 8XYE - VX <<= 1. VF is set to MSB of VX before shift
                        let before_shift = *reg_x;
                        *reg_x <<= 1;
                        self.regs[15] = before_shift & 0b1000_0000;
                    },
                    _ => panic!("Unsopported opcode {:#06x}", opcode),
                };
            },
            0x9 => {
                // 9XY0 - skip next instruction if VX != VY
                let reg_x = self.regs[opcode as usize & 0x0F00];
                let reg_y = self.regs[opcode as usize & 0x00F0];
                if reg_x != reg_y {
                    self.pc += 2;
                }
            },
            0xA => self.addr_reg = opcode & 0x0FFF, // ANNN - sets I to NNN
            0xB => self.pc = self.regs[0] as u16 + opcode as u16 & 0x0FFF,  // BXNN jump to NNN + V0
            0xC => {
                // CXNN - VX = rand & NN; rand 0-255
                let result = self.rng.gen::<u8>() & (opcode & 0x00FF) as u8;
                let reg = &mut self.regs[opcode as usize & 0x0F00];
                *reg = result;
            },
            0xD => {
                // DXYN - Draw sprit to coord (VX, VY) - width 8 pixels, height N pixels.
                //        Read from memory location I. VF set to 1 if any pixels erased
                let col = self.regs[opcode as usize & 0x0F00];
                let row = self.regs[opcode as usize & 0x00F0];
                let rows = opcode & 0x000F;
                let sprite = &self.memory[self.addr_reg as usize..(self.addr_reg + rows) as usize];
                self.regs[15] = 0;

                for (row_i, sprite_row) in sprite.iter().enumerate() {
                    for col_i in 0..8 {
                        let sprite_pixel = (sprite_row & (1 << col_i)) == 1;
                        let screen_pixel = self.pixels[row_i][col_i];

                        if sprite_pixel != screen_pixel {
                            self.pixels[row_i][col_i] = true;
                        }

                        // if gone from set to unset then set VF to 1
                        if screen_pixel == true && self.pixels[row_i][col_i] == false {
                            self.regs[15] = 1;
                        }
                    }
                }
            },
            0xE => {},
            0xF => {},
            _ => panic!("should only be a nibble"),
        };
    }
}

impl EventHandler for Game {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        self.handle_opcode();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        
        for (col_i, row) in self.pixels.iter().enumerate() {
            for (row_i, pixel) in row.iter().enumerate() {
                if *pixel {
                    let pixel_rect = Rect::new_i32(
                        row_i as i32 * PIXEL_WIDTH,
                        col_i as i32 * PIXEL_HEIGHT,
                        PIXEL_WIDTH,
                        PIXEL_HEIGHT,
                    );

                    canvas.draw(
                        &Quad,
                        DrawParam::new()
                            .dest_rect(pixel_rect)
                            .color(Color::WHITE)
                    );
                }
            }
        }
        
        canvas.finish(ctx)
    }
}