use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::{self, SoundSource, Source};
use ggez::conf::WindowSetup;
use ggez::event::{self, EventHandler};
use ggez::glam::Vec2;
use ggez::graphics::{self, Color, DrawParam, Image, InstanceArray};
use ggez::input::keyboard::{KeyCode, KeyboardContext, KeyInput};

use std::collections::HashSet;
use std::{env, path, fs};

use crate::cpu::{self, CPU};

const PIXEL_SIZE: f32 = 16.0;
pub const SCREEN_SIZE: (f32, f32) = (cpu::WIDTH as f32 * PIXEL_SIZE, cpu::HEIGHT as f32 * PIXEL_SIZE);

pub struct EmulatorIO {
    pixels_batch: InstanceArray,
    beep_sound: Source,
    cpu: CPU,
}

impl EmulatorIO {
    pub fn new(ctx: &mut Context, rom_path: String) -> EmulatorIO {
        let pixel_rect = Image::from_color(
            &ctx.gfx,
            PIXEL_SIZE as u32,
            PIXEL_SIZE as u32,
            Some(Color::WHITE),
        );
        let pixels_batch = InstanceArray::new(&ctx.gfx, pixel_rect);

        let mut created = EmulatorIO {
            pixels_batch,
            beep_sound: audio::Source::new(ctx, "/beep.wav").unwrap(),
            cpu: CPU::new(),
        };

        created.beep_sound.set_repeat(true);

        let rom = fs::read(rom_path).unwrap(); // TODO: Error Handling
        created.cpu.load_rom(&rom);

        created
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

impl EventHandler for EmulatorIO {
    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        let key = self.key_for_keycode(&input.keycode.unwrap());

        if let Some(key) = key  {
            self.cpu.key_released(key);
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let pressed_keys = self.get_pressed_keys(&ctx.keyboard);

        if self.cpu.timer_tick() && !self.beep_sound.playing() {
            self.beep_sound.play(&ctx.audio)?;
        }
        else {
            self.beep_sound.pause();
        }

        for _ in 0..12 {
            self.cpu.handle_opcode(&pressed_keys);

            if ctx.time.ticks() % 100 == 0 {
                println!("Delta frame time: {:?} ", ctx.time.delta());
                println!("Average FPS: {}", ctx.time.fps());
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        if !self.cpu.pixels_dirty {
            return Ok(());
        }

        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        self.pixels_batch.clear();

        for (col_i, row) in self.cpu.pixels.iter().enumerate() {
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

        self.cpu.pixels_dirty = false;

        canvas.draw(&self.pixels_batch, DrawParam::new());
        canvas.finish(ctx)
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