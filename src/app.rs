use crate::debug_ui::ResourceTestPanel;
use crate::renderer::Renderer;
use std::cell::RefCell;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

pub struct App {
    // Nowe pola dla egui
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    // Panel do testowania tworzenia/zwalniania tekstur i obrazów z egui
    resource_test_panel: ResourceTestPanel,

    renderer: Option<Renderer>,
    window: Option<Window>,
    frame_number: RefCell<usize>,
}

impl App {
    pub fn default() -> App {
        App {
            window: None,
            renderer: None,
            frame_number: RefCell::new(0),
            egui_ctx: egui::Context::default(),
            egui_state: None,
            resource_test_panel: ResourceTestPanel::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(Window::default_attributes()).expect("Failed to create window");

        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        self.window = Some(window);
        self.egui_state = Some(egui_state);

        match crate::renderer::Renderer::new(
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            self.window.as_ref().unwrap()
        ) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => eprintln!("Failed to create renderer: {}", e),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.egui_state {
            if let Some(window) = &self.window {
                let response = state.on_window_event(window, &event);
                if response.consumed {
                    return;
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let state = self.egui_state.as_mut().unwrap();

                let raw_input = state.take_egui_input(window);

                let gpu_texture_count = self
                    .renderer
                    .as_ref()
                    .map(|renderer| renderer.egui_painter.texture_count());

                let mut full_output = self.egui_ctx.run_ui(raw_input, |ctx| {
                    egui::Window::new("⚙ Moje Pierwsze UI w Vulkanie!").show(ctx, |ui| {
                        ui.label("Działa! Jesteśmy podpięci pod silnik.");
                        ui.separator();
                        if ui.button("Kliknij mnie!").clicked() {
                            println!("Przycisk został kliknięty!");
                        }
                        ui.color_edit_button_rgb(self.renderer.as_mut().unwrap().compute_drawer.gradient_color_top_rgb());
                        ui.color_edit_button_rgb(self.renderer.as_mut().unwrap().compute_drawer.gradient_color_bottom_rgb());
                    });

                    self.resource_test_panel.show(ctx, gpu_texture_count);
                });

                state.handle_platform_output(window, full_output.platform_output);

                let pixels_per_point = window.scale_factor() as f32;
                let primitives = self.egui_ctx.tessellate(full_output.shapes, pixels_per_point);

                let screen_size = [window.inner_size().width, window.inner_size().height];
                let mut frame_number = self.frame_number.borrow_mut();
                *frame_number += 1;

                let _log_message = format!("Frame number: {}", *frame_number);
                match self.renderer.as_mut().unwrap().draw(
                    *frame_number,
                    &primitives,
                    &full_output.textures_delta,
                    screen_size,
                    pixels_per_point
                ) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Draw error: {}", e),
                }
                full_output.textures_delta.clear();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}