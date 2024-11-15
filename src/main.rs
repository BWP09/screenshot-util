use std::borrow::Cow;
use std::process;
use std::fs;
use std::path;

use chrono;
use screenshots::Screen;
use mouse_position::mouse_position::Mouse;
use arboard::Clipboard;
use eframe::egui::{self, Color32, Margin, Pos2, Rect, Rounding, Shadow, Stroke};
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::{DynamicImage, GenericImageView, RgbaImage};
use dirs;

// -- TODO --
// add option to copy and/or open path

fn capture_screen_under_mouse(path: &String) {
    let (mouse_x, mouse_y) = match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => (x, y),
        Mouse::Error => {
            println!("Error getting mouse position");
            (0, 0)
        }
    };

    let screen = Screen::from_point(mouse_x, mouse_y).unwrap();

    let full_image = screen.capture().unwrap();

    full_image.save(path).expect("Error saving image!");
}

#[allow(unused)]
fn capture_all() {
    let screens = Screen::all().unwrap();

    for screen in screens {
        println!("capturer {screen:?}");
        let mut image = screen.capture().unwrap();
        image
            .save(format!("target/{}.png", screen.display_info.id))
            .expect("Error saving image!");

        // image = screen.capture_area(300, 300, 300, 300).unwrap();
        // image
        //     .save(format!("target/{}-2.png", screen.display_info.id))
        //     .expect("Error saving cropped image!");
    }
}

fn reverse_split(input: &str, delimiter: char) -> (&str, &str) {
    let mut parts = input.rsplitn(2, delimiter);

    let last = parts.next().unwrap();
    let rest = parts.next().unwrap_or("");

    (rest, last)
}

fn get_datetime() -> String {
    chrono::offset::Local::now().format("%F_%T").to_string()
}

fn set_clipboard_image(image_data: &RgbaImage) {
    let img = arboard::ImageData {
        width: image_data.dimensions().0 as usize,
        height: image_data.dimensions().1 as usize,
        bytes: Cow::from(image_data.as_raw())
    };

    let mut clipboard = Clipboard::new().unwrap();
    clipboard.set_image(img).expect("Error setting clipboard image!");
}

fn create_directory_if_not_exists(path: &str) -> std::io::Result<()> {
    let path = path::Path::new(path);

    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    Ok(())
}

fn main() {
    let dir_path = dirs::picture_dir().map(|path| path.join("screenshot-util")).expect("Could not access pictures dir!").to_str().unwrap().to_string();
    create_directory_if_not_exists(&dir_path).expect("Could not create screenshot-util dir!");

    let path = format!("{}/{}.png", dir_path, get_datetime());

    capture_screen_under_mouse(&path);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };

    let _ = eframe::run_native("screenshot-util - Image Cropper", options, Box::new(|_cc| Ok(Box::new(ScreenshotUtil::default(&path)))));
}

struct ScreenshotUtil {
    original_image: DynamicImage,
    original_path: String,
    original_dimensions: (u32, u32),
    texture: Option<TextureHandle>,
    cropping_rect: Option<egui::Rect>,
    rect_min: Option<Pos2>,
    rect_max: Option<Pos2>,
    crop_rect_min: Option<Pos2>,
    crop_rect_max: Option<Pos2>,
    first_down: bool,
    image_offset_x: f32,
    image_offset_y: f32,
}

impl ScreenshotUtil {
    fn default(path: &String) -> Self {
        let img = image::open(path).expect("Could not open image file!");
        let dimensions = img.dimensions();

        Self {
            original_image: img,
            original_path: path.clone(),
            original_dimensions: dimensions,
            texture: None,
            cropping_rect: None,
            rect_min: None,
            rect_max: None,
            crop_rect_min: None,
            crop_rect_max: None,
            first_down: false,
            image_offset_x: 0.0,
            image_offset_y: 0.0,
        }
    }

    fn fix_pos(&self, pos_min: &mut Pos2, pos_max: &mut Pos2) {
        if pos_max.x < pos_min.x {
            let temp_x = pos_max.x;
            pos_max.x = pos_min.x;
            pos_min.x = temp_x;
        }

        if pos_max.y < pos_min.y {
            let temp_y = pos_max.y;
            pos_max.y = pos_min.y;
            pos_min.y = temp_y;
        }
    }

    fn copy_rect(&self, rect: Rect) {
        let min = rect.min;
        let max = rect.max;
        let crop_area = (
            (min.x - self.image_offset_x) as u32,
            (min.y - self.image_offset_y) as u32,
            ((max.x - min.x)) as u32,
            ((max.y - min.y)) as u32,
        );

        let image_data = self.original_image.crop_imm(
            crop_area.0,
            crop_area.1,
            crop_area.2,
            crop_area.3,
        ).to_rgba8();

        let (path, _) = reverse_split(&self.original_path, '.');

        let path = format!("{}-cropped_at-{}.png", path, get_datetime());
        image_data.save(path).expect("Error saving cropped image!");

        set_clipboard_image(&image_data);
    }

    fn get_stroke_width(&self, pos_min: Pos2, pos_max: Pos2) -> f32 {
        let size_x = pos_max.x - pos_min.x;
        let size_y = pos_max.y - pos_min.y;

        if size_x >= 25.0 && size_y >= 25.0 {
            2.0
        }

        else {
            1.0
        }
    }

    fn get_circle_radius(&self) -> f32 {
        if let (Some(ref mut pos_min), Some(ref mut pos_max)) = (self.rect_min, self.rect_max) {
            self.fix_pos(pos_min, pos_max);

            let size_x = pos_max.x - pos_min.x;
            let size_y = pos_max.y - pos_min.y;
    
            if size_x >= 25.0 && size_y >= 25.0 {
                3.0
            }

            else {
                1.5
            }
        }
        
        else {
            2.0
        }
    }
}

impl eframe::App for ScreenshotUtil {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        let window_rect = ctx.input(|i: &egui::InputState| i.screen_rect());
        self.image_offset_x = (window_rect.width() - self.original_dimensions.0 as f32) / 2.0;
        self.image_offset_y = (window_rect.height() - self.original_dimensions.1 as f32) / 2.0;

        egui::CentralPanel::default()
            .frame(egui::Frame {
                inner_margin: Margin::symmetric(self.image_offset_x, self.image_offset_y),
                outer_margin: Margin::ZERO,
                rounding: Rounding::ZERO,
                shadow: Shadow::NONE,
                stroke: Stroke::NONE,
                fill: Color32::from_gray(0x1b)
            })
            .show(ctx, |ui| {
                // Load image into a texture if not already done
                if self.texture.is_none() {
                    let size = [self.original_image.width() as _, self.original_image.height() as _];
                    let color_image = ColorImage::from_rgba_unmultiplied(size, self.original_image.to_rgba8().as_flat_samples().as_slice());
                    self.texture = Some(ui.ctx().load_texture("image", color_image, TextureOptions::default()));
                }

                if let Some(ref texture) = self.texture {
                    // Display the image
                    ui.image(texture);

                    // Exit if [Esc] is pressed
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        fs::remove_file(self.original_path.clone()).expect("Could not remove image file!");
                        process::exit(0);
                    }

                    // Set corners with dragging
                    if ui.input(|i| i.pointer.primary_down()) {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            if !self.first_down {
                                self.rect_min = Some(pos);
                                self.first_down = true;
                            }
                            
                            else {
                                self.rect_max = Some(pos);
                            }
                        }
                    }

                    // Drag logic
                    else if ui.input(|i| i.pointer.primary_released()) && self.first_down {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            self.rect_max = Some(pos);
                            self.first_down = false;
                        }
                    }

                    else if ui.input(|i| i.pointer.secondary_down()) {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            self.rect_max = Some(pos);
                        }
                    }

                    // Crop image and copy (no exit) on [Enter]
                    else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let pos_min: Pos2;
                        let pos_max: Pos2;
                        
                        if let (Some(mut pos_min_2), Some(mut pos_max_2)) = (self.rect_min, self.rect_max) {
                            self.fix_pos(&mut pos_min_2, &mut pos_max_2);

                            pos_min = pos_min_2;
                            pos_max = pos_max_2;
                        }
                        
                        else {
                            pos_min = Pos2::new(self.image_offset_x, self.image_offset_y);
                            pos_max = Pos2::new(self.original_dimensions.0 as f32 + self.image_offset_x, self.original_dimensions.1 as f32 + self.image_offset_y);
                        }

                        let rect = egui::Rect::from_min_max(pos_min, pos_max);
                        self.cropping_rect = Some(rect.clone());
                        self.crop_rect_min = Some(pos_min);
                        self.crop_rect_max = Some(pos_max);

                        self.copy_rect(rect);
                    }

                    // Crop image, copy, and exit on [Space]
                    else if ui.input(|i| i.key_pressed(egui::Key::Space)) {
                        let pos_min: Pos2;
                        let pos_max: Pos2;

                        if let (Some(mut pos_min_2), Some(mut pos_max_2)) = (self.rect_min, self.rect_max) {
                            self.fix_pos(&mut pos_min_2, &mut pos_max_2);
                            
                            pos_min = pos_min_2;
                            pos_max = pos_max_2;
                        }
                        
                        else {
                            pos_min = Pos2::new(0.0,0.0);
                            pos_max = Pos2::new(self.original_dimensions.0 as f32, self.original_dimensions.1 as f32);
                        }

                        let rect = egui::Rect::from_min_max(pos_min, pos_max);

                        self.copy_rect(rect);

                        process::exit(0);
                    }

                    // Draw circles for rect corners
                    if let Some(pos_min) = self.rect_min {
                        ui.painter().circle_filled(pos_min, self.get_circle_radius() + 2.0, egui::Color32::from_hex("#00ffff").expect("Invalid hex color!"));
                    }
                    
                    // Draw circles for rect corners
                    if let Some(pos_max) = self.rect_max {
                        ui.painter().circle_filled(pos_max, self.get_circle_radius() + 2.0, egui::Color32::RED);
                    }
                    
                    // Draw preview cropping rect
                    if let (Some(mut pos_min), Some(mut pos_max)) = (self.rect_min, self.rect_max) {
                        self.fix_pos(&mut pos_min, &mut pos_max);
                        
                        ui.painter().rect_stroke(egui::Rect::from_min_max(pos_min, pos_max), 2.0, (self.get_stroke_width(pos_min, pos_max), egui::Color32::GOLD));
                    }
                    
                    // Draw cropping rect if available
                    if let (Some(ref rect), Some(mut pos_min), Some(mut pos_max)) = (self.cropping_rect, self.crop_rect_min, self.crop_rect_max) {
                        self.fix_pos(&mut pos_min, &mut pos_max);

                        ui.painter().rect_stroke(*rect, 2.0, (self.get_stroke_width(pos_min, pos_max), egui::Color32::GREEN));
                    }
                }
            });
    }
}
