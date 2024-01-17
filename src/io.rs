use ggegui::Gui;
use ggegui::egui::{self, menu, Window, widgets};
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

use crate::cpu::{self, CPU, ShiftingReg, RegSaveLoadQuirk, JumpBehviour, Quirks};

const DEFAULT_CYCLES_PER_FRAME: u16 = 12;

const DEFAULT_OFF_COLOUR: Color = Color {r: 0.057805423, g: 0.057805423, b: 0.057805423, a: 1.0};
const DEFAULT_ON_COLOUR: Color = Color::WHITE;

const DEFAULT_PIXEL_SIZE: f32 = 16.0;
const MENU_BAR_HEIGHT: f32 = 24.0;
const SCREEN_SIZE: (f32, f32) = (cpu::WIDTH as f32 * DEFAULT_PIXEL_SIZE, cpu::HEIGHT as f32 * DEFAULT_PIXEL_SIZE + MENU_BAR_HEIGHT);

pub struct EmulatorIO {
    pixels_batch: InstanceArray,
    beep_sound: Source,
    cpu: CPU,
    cycles_per_frame: u16,
    gui: Gui,
    config_window_open: bool,
    last_loaded_rom: Option<Vec<u8>>,
    menu_bar_height: f32,
    height_offset: f32,
    width_offset: f32,
    pixel_size: f32,
    pixel_on_colour: Color,
    pixel_off_colour: Color,
}

impl EmulatorIO {
    pub fn new(ctx: &mut Context) -> EmulatorIO {
        let pixel_rect = Image::from_color(
            &ctx.gfx,
            DEFAULT_PIXEL_SIZE as u32,
            DEFAULT_PIXEL_SIZE as u32,
            None,
        );
        let pixels_batch = InstanceArray::new(&ctx.gfx, pixel_rect);

        let mut created = EmulatorIO {
            pixels_batch,
            beep_sound: audio::Source::new(ctx, "/beep.wav").unwrap(),
            cpu: CPU::new(),
            cycles_per_frame: DEFAULT_CYCLES_PER_FRAME,
            gui: Gui::new(ctx),
            menu_bar_height: MENU_BAR_HEIGHT,
            height_offset: 0.0,
            width_offset: 0.0,
            last_loaded_rom: None,
            config_window_open: false,
            pixel_size: DEFAULT_PIXEL_SIZE,
            pixel_off_colour: DEFAULT_OFF_COLOUR,
            pixel_on_colour: DEFAULT_ON_COLOUR,
        };
        
        created.beep_sound.set_repeat(true);

        let rom = vec![0x12, 0x00]; // infinte loop
        created.cpu.load_rom(&rom);

        created.resize_event(ctx, ctx.gfx.drawable_size().0, ctx.gfx.drawable_size().0).unwrap();

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

        for _ in 0..self.cycles_per_frame {
            let old_res = self.cpu.resolution;
            self.cpu.handle_opcode(&pressed_keys);
            if self.cpu.resolution != old_res {
                self.resize_event(ctx, ctx.gfx.drawable_size().0, ctx.gfx.drawable_size().1)?;
            }
        }

        Ok(())
    }

    fn update_gui(&mut self, ctx: &mut Context) -> GameResult {
        if ctx.gfx.drawable_size() == (0.0, 0.0) {
            return Ok(());
        }

        let gui_ctx = &self.gui.ctx();

        let height = egui::TopBottomPanel::top("MenuBar").show(gui_ctx, |ui| {
            menu::bar(ui, |ui| {
                if ui.button("Load ROM").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        ctx.gfx.set_window_title(format!("{} - Fish n CHIP-8", path.file_name().unwrap().to_str().unwrap()).as_str());

                        let quirks = self.cpu.quirks;

                        let rom = fs::read(path).unwrap();
                        self.last_loaded_rom = Some(rom.clone());

                        self.cpu = CPU::new();
                        self.cpu.load_rom(&rom);
                        self.cpu.quirks = quirks;
                        self.resize_event(ctx, ctx.gfx.drawable_size().0, ctx.gfx.drawable_size().1).unwrap();
                    }
                }
                if ui.button("Restart current ROM").clicked() {
                    if let Some(rom) = &self.last_loaded_rom {
                        let quirks = self.cpu.quirks;

                        self.cpu = CPU::new();
                        self.cpu.load_rom(rom);
                        self.cpu.quirks = quirks;
                    }
                }
                if ui.button("Configuration").clicked() {
                    self.config_window_open = true;
                }
                if self.config_window_open {
                    Window::new("Configuration").open(&mut self.config_window_open).resizable(true).show(gui_ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Cyles per frame: ");
                            ui.add(egui::DragValue::new(&mut self.cycles_per_frame));
                            
                            if ui.button("Reset to default").clicked() {
                                self.cycles_per_frame = DEFAULT_CYCLES_PER_FRAME;
                            }
                        });
                        ui.separator();

                        ui.heading("Apperance: ");
                        ui.horizontal(|ui| {
                            ui.label("Pixel size: ");
                            ui.add(egui::DragValue::new(&mut self.pixel_size)).changed().then(|| {
                                    let width = self.pixel_size * self.cpu.width() as f32;
                                    ctx.gfx.set_drawable_size(width, width / 2.0 + self.menu_bar_height).unwrap();
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Background: ");

                            let colour = self.pixel_off_colour;
                            let mut colour = [colour.r, colour.g, colour.b];
                            widgets::color_picker::color_edit_button_rgb(ui, &mut colour);
                            self.pixel_off_colour = Color::new(colour[0], colour[1], colour[2], 100.0);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Foreground: ");

                            let colour = self.pixel_on_colour;
                            let mut colour = [colour.r, colour.g, colour.b];
                            widgets::color_picker::color_edit_button_rgb(ui, &mut colour);
                            self.pixel_on_colour = Color::new(colour[0], colour[1], colour[2], 100.0);
                        });
                        if ui.button("Reset apperance to default").clicked() {
                            self.pixel_off_colour = DEFAULT_OFF_COLOUR;
                            self.pixel_on_colour = DEFAULT_ON_COLOUR;

                            let width = DEFAULT_PIXEL_SIZE * self.cpu.width() as f32;
                            ctx.gfx.set_drawable_size(width, width / 2.0 + self.menu_bar_height).unwrap();
                        }
                        ui.separator();

                        ui.heading("Quirks: ");
                        ui.horizontal(|ui| {
                            ui.label("VF reset on all 8XYO opcodes: ");
                            ui.checkbox(&mut self.cpu.quirks.vf_reset, "");
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
                        if ui.button("Reset quirks to default").clicked() {
                            self.cpu.quirks = Quirks::default();
                        }
                        ui.separator();

                        if ui.button("Reset all to default").clicked() {
                            self.cpu.quirks = Quirks::default();
                            self.cycles_per_frame = DEFAULT_CYCLES_PER_FRAME;

                            let width = DEFAULT_PIXEL_SIZE * self.cpu.width() as f32;
                            ctx.gfx.set_drawable_size(width, width / 2.0 + self.menu_bar_height).unwrap();

                            self.pixel_off_colour = DEFAULT_OFF_COLOUR;
                            self.pixel_on_colour = DEFAULT_ON_COLOUR;
                        }
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
        self.pixels_batch.clear();

        for (col_i, row) in self.cpu.pixels.iter().enumerate() {
            for (row_i, pixel) in row.iter().enumerate() {
                if *pixel {
                    self.pixels_batch.push(
                        DrawParam::new().dest(Vec2::new(
                            row_i as f32 * self.pixel_size + self.width_offset,
                            col_i as f32 * self.pixel_size + self.height_offset + self.menu_bar_height,
                        ))
                        .color(self.pixel_on_colour),
                    );
                }
                else {
                    self.pixels_batch.push(
                        DrawParam::new()
                        .dest(Vec2::new(
                            row_i as f32 * self.pixel_size + self.width_offset,
                            col_i as f32 * self.pixel_size + self.height_offset + self.menu_bar_height,
                        ))
                        .color(self.pixel_off_colour),
                    );
                }
            }
        }

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

    fn text_input_event( &mut self, _ctx: &mut ggez::Context, character: char) -> GameResult {
		self.gui.input.text_input_event(character);
		Ok(())
	}

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> Result<(), ggez::GameError> {
        if width == 0.0 || height == 0.0 {
            return Ok(());
        }

        self.pixel_size = (width / self.cpu.width() as f32).min(height / self.cpu.height() as f32).floor(); // allow resizing from both directions without part of the screen being cut off

        let pixel_rect = Image::from_color(
            &ctx.gfx,
            self.pixel_size as u32,
            self.pixel_size as u32,
            None,
        );
        self.pixels_batch = InstanceArray::new(&ctx.gfx, pixel_rect);
        self.gui.input.resize_event(width, height);

        self.height_offset = ((height - self.menu_bar_height) - self.pixel_size * self.cpu.height() as f32) / 2.0;
        self.width_offset = (width - self.pixel_size * self.cpu.width() as f32) / 2.0;

        Ok(())
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
        .window_mode(ggez::conf::WindowMode::default()
            .dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1)
            .resizable(true)
        )
        .add_resource_path(resource_dir)
        .build()
        .expect("Failed to create game context");

    let game = EmulatorIO::new(&mut ctx);

    event::run(ctx, event_loop, game);
}