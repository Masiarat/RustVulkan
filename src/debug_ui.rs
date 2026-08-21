//! Self-contained egui panel used to stress-test creation/cleanup of egui
//! textures (and, transitively, the Vulkan images/descriptor sets backing
//! them in `TextureManager`).
//!
//! This module only talks to the public `egui::Context` API
//! (`Context::load_texture` and dropping `TextureHandle`s), it does not know
//! anything about `Renderer` internals besides the single, optional
//! `gpu_texture_count` number passed into `show`. Because of that it can be
//! removed again by simply deleting this file and the few call sites in
//! `app.rs` that wire it up.

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};

/// A window that lets the user repeatedly create and free egui textures to
/// exercise the renderer's texture/image allocation and cleanup code paths.
pub struct ResourceTestPanel {
    open: bool,
    textures: Vec<TextureHandle>,
    next_id: u64,
    texture_size: u32,
    batch_size: u32,
    total_created: u64,
    total_freed: u64,
}

impl Default for ResourceTestPanel {
    fn default() -> Self {
        Self {
            open: true,
            textures: Vec::new(),
            next_id: 0,
            texture_size: 64,
            batch_size: 1,
            total_created: 0,
            total_freed: 0,
        }
    }
}

impl ResourceTestPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the panel. `gpu_texture_count`, when available, is the number
    /// of textures currently tracked by the renderer's `TextureManager`
    /// (i.e. actual live Vulkan images), which lets us confirm that freeing
    /// textures on the egui side really results in GPU resources being
    /// destroyed.
    pub fn show(&mut self, ctx: &Context, gpu_texture_count: Option<usize>) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        egui::Window::new("🧪 Resource Cleanup Tester")
            .open(&mut open)
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.label(
                    "Stress-test creation and cleanup of textures/images \
                     created through egui.",
                );
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Texture size:");
                    ui.add(egui::Slider::new(&mut self.texture_size, 4..=512).suffix("px"));
                });
                ui.horizontal(|ui| {
                    ui.label("Batch size:");
                    ui.add(egui::Slider::new(&mut self.batch_size, 1..=64));
                });

                ui.horizontal(|ui| {
                    if ui.button("➕ Create").clicked() {
                        for _ in 0..self.batch_size {
                            self.create_texture(ctx);
                        }
                    }
                    if ui.button("➖ Free one").clicked() {
                        self.free_one();
                    }
                    if ui.button("🗑 Free all").clicked() {
                        self.free_all();
                    }
                });

                ui.separator();
                ui.label(format!("Live egui textures: {}", self.textures.len()));
                ui.label(format!("Total created: {}", self.total_created));
                ui.label(format!("Total freed: {}", self.total_freed));
                if let Some(count) = gpu_texture_count {
                    ui.label(format!("Live GPU textures (renderer): {}", count));
                }

                if !self.textures.is_empty() {
                    ui.separator();
                    ui.label("Preview (first 8):");
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for handle in self.textures.iter().take(8) {
                                ui.image((handle.id(), egui::vec2(48.0, 48.0)));
                            }
                        });
                    });
                }
            });
        self.open = open;
    }

    fn create_texture(&mut self, ctx: &Context) {
        let size = self.texture_size.max(1) as usize;
        let color = hue_to_color32((self.next_id as f32 * 47.0) % 360.0);
        let image = ColorImage::filled([size, size], color);
        let handle = ctx.load_texture(
            format!("resource_test_{}", self.next_id),
            image,
            TextureOptions::LINEAR,
        );
        self.textures.push(handle);
        self.next_id += 1;
        self.total_created += 1;
    }

    fn free_one(&mut self) {
        if self.textures.pop().is_some() {
            self.total_freed += 1;
        }
    }

    fn free_all(&mut self) {
        self.total_freed += self.textures.len() as u64;
        self.textures.clear();
    }
}

/// Small, self-contained HSV -> RGB conversion so this module does not need
/// to depend on any egui-internal color helpers.
fn hue_to_color32(hue_degrees: f32) -> Color32 {
    let h = hue_degrees / 60.0;
    let sector = h.floor() as i32 % 6;
    let f = h - h.floor();
    let (r, g, b) = match sector {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
