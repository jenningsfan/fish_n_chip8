use ggegui::Gui;
use ggegui::egui::{self, menu, Window};
use rfd;

use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::{self, SoundSource, Source};
use ggez::conf::WindowSetup;
use ggez::event::{self, EventHandler};
use ggez::glam::Vec2;
use ggez::graphics::{Canvas, Color, DrawParam, Image, InstanceArray};
use ggez::input::keyboard::{KeyCode, KeyboardContext, KeyInput};

use std::collections::HashSet;
use std::{env, path, fs};

use crate::cpu::{self, CPU, ShiftingReg, RegSaveLoadQuirk, JumpBehviour};

const PIXEL_SIZE: f32 = 16.0;
const MENU_BAR_HEIGHT: f32 = 24.0;
pub const SCREEN_SIZE: (f32, f32) = (cpu::WIDTH as f32 * PIXEL_SIZE, cpu::HEIGHT as f32 * PIXEL_SIZE + MENU_BAR_HEIGHT);

pub struct EmulatorIO {
    pixels_batch: InstanceArray,
    beep_sound: Source,
    cpu: CPU,
    gui: Gui,
    quirks_window_open: bool,
    menu_bar_height: f32, // probs not best practice
}

impl EmulatorIO {
    pub fn new(ctx: &mut Context) -> EmulatorIO {
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
            gui: Gui::new(ctx),
            menu_bar_height: 0.0,
            quirks_window_open: false,
        };
        
        created.beep_sound.set_repeat(true);

        let rom = vec![0x12, 0x00]; // infinte loop
        created.cpu.load_rom(&rom);

        created
    }

    fn key_for_keycode(&self, keycode: Option<&KeyCode>) -> Option<u8> {
        if let Some(keycode) = keycode {
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
        else {
            return None;
        }
    }

    fn get_pressed_keys(&self, key_ctx: &KeyboardContext) -> HashSet<u8> {
        let pressed = key_ctx.pressed_keys();
        let mut pressed_nums: HashSet<u8> = HashSet::new();
        
        for key in pressed {
            if let Some(key) = self.key_for_keycode(Some(key)) {
                pressed_nums.insert(key);
            }
        }

        pressed_nums
    }

    fn update_cpu(&mut self, ctx: &mut Context) -> GameResult {
        let pressed_keys = self.get_pressed_keys(&ctx.keyboard);

        if self.cpu.timer_tick() {
            self.beep_sound.play_later()?;
        }
        else {
            self.beep_sound.stop(&ctx.audio)?;
        }

        for _ in 0..12 {
            self.cpu.handle_opcode(&pressed_keys);
        }

        Ok(())
    }

    fn update_gui(&mut self, ctx: &mut Context) -> GameResult {
        let gui_ctx = &self.gui.ctx();

        let height = egui::TopBottomPanel::top("MenuBar").show(gui_ctx, |ui| {
            menu::bar(ui, |ui| {
                if ui.button("Load ROM").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let rom = fs::read(path).unwrap(); // TODO: Error Handling
                        self.cpu = CPU::new();
                        self.cpu.load_rom(&rom);
                    }
                }
                if ui.button("Configure quirks").clicked() {
                    self.quirks_window_open = true;
                }
                if self.quirks_window_open {
                    Window::new("Quirks").open(&mut self.quirks_window_open).show(gui_ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("vF reset on all 8XYO opcodes: ");
                            ui.checkbox(&mut self.cpu.quirks.VF_reset, "");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Shifting opcodes operate on: ");
                            ui.selectable_value(&mut self.cpu.quirks.shifting, ShiftingReg::VX, "vX");
                            ui.selectable_value(&mut self.cpu.quirks.shifting, ShiftingReg::VY, "vY");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Register save/load opcode behaviour: ");
                            ui.selectable_value(&mut self.cpu.quirks.reg_save_load, RegSaveLoadQuirk::Unchanged, "Do not modify I");
                            ui.selectable_value(&mut self.cpu.quirks.reg_save_load, RegSaveLoadQuirk::X, "I = I + X");
                            ui.selectable_value(&mut self.cpu.quirks.reg_save_load, RegSaveLoadQuirk::XPlusOne, "I = I + X + 1");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Jump opcode behaviour: ");
                            ui.selectable_value(&mut self.cpu.quirks.jump, JumpBehviour::BNNN, "BNNN");
                            ui.selectable_value(&mut self.cpu.quirks.jump, JumpBehviour::BXNN, "BXNN");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Sprites wrap at edges of screen: ");
                            ui.checkbox(&mut self.cpu.quirks.screen_wrap, "");
                        });
                    });
                }
            });
        }).response.rect.height();

        self.gui.update(ctx);
        self.menu_bar_height = height;
        //ctx.gfx.set_drawable_size(SCREEN_SIZE.0, SCREEN_SIZE.1 as f32 + height)?; // make room for whole game

        Ok(())
    }

    fn draw_gui(&mut self, canvas: &mut Canvas) {
        canvas.draw(
            &self.gui, 
            DrawParam::default().dest(Vec2::ZERO),
        );
    }

    fn draw_pixel_grid(&mut self, _ctx: &mut Context, canvas: &mut Canvas) {
        // if !self.cpu.pixels_dirty {
        //     return
        // }
        self.pixels_batch.clear();

        for (col_i, row) in self.cpu.pixels.iter().enumerate() {
            for (row_i, pixel) in row.iter().enumerate() {
                if *pixel {
                    self.pixels_batch.push(
                        DrawParam::new().dest(Vec2::new(
                            row_i as f32 * PIXEL_SIZE,
                            col_i as f32 * PIXEL_SIZE + MENU_BAR_HEIGHT,
                        )),
                    );
                }
            }
        }

        self.cpu.pixels_dirty = false;
        canvas.draw(&self.pixels_batch, DrawParam::new());
    }
}

impl EventHandler for EmulatorIO {
    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        let key = self.key_for_keycode(input.keycode.as_ref());

        if let Some(key) = key  {
            self.cpu.key_released(key);
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.update_cpu(ctx)?;
        self.update_gui(ctx)?;

        if ctx.time.ticks() % 100 == 0 {
            println!("Delta frame time: {:?} ", ctx.time.delta());
            println!("Average FPS: {}", ctx.time.fps());
        }

        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        
        self.draw_pixel_grid(ctx, &mut canvas);
        self.draw_gui(&mut canvas);

        canvas.finish(ctx)
    }
}

pub fn emulator_main() {
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

    let game = EmulatorIO::new(&mut ctx);

    event::run(ctx, event_loop, game);
}