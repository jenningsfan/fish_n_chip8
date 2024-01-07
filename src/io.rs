use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::{self, SoundSource, Source};
use ggez::conf::WindowSetup;
use ggez::event::{self, EventHandler};
use ggez::glam::Vec2;
use ggez::graphics::{self, Color, DrawParam, Image, InstanceArray};
use ggez::input::keyboard::{KeyCode, KeyboardContext, KeyInput};

use std::collections::HashSet;
use std::{env, path, fs};

use crate::cpu::{CPU, Chip8IO};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const PIXEL_SIZE: f32 = 16.0;
pub const SCREEN_SIZE: (f32, f32) = (WIDTH as f32 * PIXEL_SIZE, HEIGHT as f32 * PIXEL_SIZE);

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

pub struct EmulatorIO<'a> {
    pixels_batch: InstanceArray,
    pixels_dirty: bool,
    beep_sound: Source,
    cpu: Option<CPU<'a>>,
}

impl EmulatorIO<'_> {
    pub fn new<'a>(ctx: &'a mut Context, rom_path: String) -> EmulatorIO<'a> {
        let pixel_rect = Image::from_color(
            &ctx.gfx,
            PIXEL_SIZE as u32,
            PIXEL_SIZE as u32,
            Some(Color::WHITE),
        );
        let pixels_batch = InstanceArray::new(&ctx.gfx, pixel_rect);

        let mut created = EmulatorIO {
            pixels_batch,
            pixels_dirty: false,
            beep_sound: audio::Source::new(ctx, "/beep.wav").unwrap(),
            cpu: None,
        };

        created.beep_sound.set_repeat(true);

        let mut cpu = CPU::new(&mut created);
        let rom = fs::read(rom_path).unwrap(); // TODO: Error Handling
        cpu.load_rom(&rom);
        created.cpu = Some(cpu);

        created
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

    fn get_pressed_keys(&self, key_ctx: &KeyboardContext) -> HashSet<u8> {
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

impl EventHandler for EmulatorIO<'_> {
    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        let key = self.key_for_keycode(&input.keycode.unwrap());

        if let Some(key) = key  {
            self.cpu.as_mut().unwrap().key_released(key);
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let pressed_keys = self.get_pressed_keys(&ctx.keyboard);
        let cpu = self.cpu.as_mut().unwrap();

        let beep = cpu.timer_tick();

        if beep && !self.beep_sound.playing() {
            self.beep_sound.play(&ctx.audio)?;
        }
        else {
            self.beep_sound.pause();
        }

        for _ in 0..12 {
            cpu.handle_opcode(&pressed_keys);

            if ctx.time.ticks() % 100 == 0 {
                println!("Delta frame time: {:?} ", ctx.time.delta());
                println!("Average FPS: {}", ctx.time.fps());
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let cpu = self.cpu.as_mut().unwrap();

        if !self.pixels_dirty {
            return Ok(());
        }

        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        self.pixels_batch.clear();

        for (col_i, row) in cpu.pixels.iter().enumerate() {
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

impl Chip8IO for EmulatorIO<'_> {
    fn redraw(&mut self) {
        self.pixels_dirty = true;
    }
}

pub fn emulator_main(rom_path: String) {
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

    let game = EmulatorIO::new(&mut ctx, rom_path);

    event::run(ctx, event_loop, game);
}