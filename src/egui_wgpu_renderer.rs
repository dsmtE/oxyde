use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::{State, EventResponse};
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();
        let viewport_id = egui_context.viewport_id();
        let egui_state = egui_winit::State::new(egui_context, viewport_id, &window, Some(window.scale_factor() as f32), None);
        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn context(&self) -> &Context {
        &self.state.egui_ctx()
    }

    pub fn draw_ui(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        run_ui: impl FnOnce(&Context),
    ) {
        let raw_input = self.state.take_egui_input(window);
        let full_output = self.context().run(raw_input, |ui| {
            run_ui(ui);
        });

        self.draw_output(
            full_output,
            device,
            queue,
            encoder,
            window,
            window_surface_view,
            screen_descriptor,
        );
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.context().begin_frame(raw_input);
    }

    pub fn end_frame(&mut self) -> egui::FullOutput {
        self.context().end_frame()
    }

    pub fn draw_output(
        &mut self,
        full_output: egui::FullOutput,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        self.state.handle_platform_output(&window, full_output.platform_output);

        let tris = self.context().tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer.update_buffers(&device, &queue, encoder, &tris, &screen_descriptor);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &window_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }

}