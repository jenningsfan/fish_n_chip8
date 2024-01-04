use ggez::conf::WindowSetup;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::graphics::{self, Color, Quad, Rect, DrawParam, MeshBuilder, DrawMode, Mesh};
use ggez::event::{self, EventHandler};

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
        }
    }

    fn load_opcode(&self, addr: u16) -> u16 {
        (self.memory[addr as usize] as u16) << 1 | (self.memory[addr as usize + 1] as u16)
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
                // 6XNN - VX += NN
                let reg = &mut self.regs[opcode as usize & 0x0F00];
                *reg += (opcode & 0x00FF) as u8;
            },
            0x8 => {},
            0x9 => {},
            0xA => {},
            0xB => {},
            0xC => {},
            0xD => {},
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