use anyhow::{Context, Result};
use egui_wgpu_backend::RenderPass;

use egui_winit_platform::{Platform, PlatformDescriptor};
use std::time::Instant;
use wgpu::CommandEncoder;
use winit::{event::Event, window::Window};

pub use egui_wgpu_backend::ScreenDescriptor;

// We repaint the UI every frame, so no custom repaint signal is needed
// struct RepaintSignal;
// impl egui::backend::RepaintSignal for RepaintSignal {
//     fn request_repaint(&self) {}
// }

pub struct Gui {
    platform: Platform,
    start_time: Instant,
    last_frame_start: Instant,
    previous_frame_time: Option<f32>,
    pub available_rect: egui::Rect,
}

impl Gui {
    pub fn new(screen_descriptor: ScreenDescriptor) -> Self {
        // We use the egui_winit_platform crate as the platform.
        let platform = Platform::new(PlatformDescriptor {
            physical_width: screen_descriptor.physical_width,
            physical_height: screen_descriptor.physical_height,
            scale_factor: screen_descriptor.scale_factor as f64,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });

        Self {
            platform,
            start_time: Instant::now(),
            previous_frame_time: None,
            last_frame_start: Instant::now(),
            available_rect: egui::Rect::EVERYTHING,
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>) { self.platform.handle_event(event); }

    pub fn context(&self) -> egui::Context { self.platform.context() }

    pub fn start_frame(&mut self, _scale_factor: f32) {
        self.platform.update_time(self.start_time.elapsed().as_secs_f64());

        // Begin to draw the UI frame.
        self.last_frame_start = Instant::now();
        self.platform.begin_frame();
    }

    pub fn end_frame(&mut self, window: &Window) -> egui::FullOutput {
        self.available_rect = self.context().available_rect();
        let frame_time = self.last_frame_start.elapsed().as_secs_f32();
        self.previous_frame_time = Some(frame_time);

        self.platform.end_frame(Some(window))
    }
}

pub struct GuiRenderWgpu {
    pub renderpass: RenderPass,
}

impl GuiRenderWgpu {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat, msaa_samples: u32) -> Self {
        Self {
            renderpass: RenderPass::new(device, output_format, msaa_samples),
        }
    }

    pub fn render(
        &mut self,
        context: egui::Context,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &ScreenDescriptor,
        encoder: &mut CommandEncoder,
        output_view: &wgpu::TextureView,
        gui_output: egui::FullOutput,
    ) -> Result<()> {
        // TODO: how handle not repaint ui if isn't needed
        // if gui_output.needs_repaint {

        self.renderpass.add_textures(device, queue, &gui_output.textures_delta)?;

        let paint_jobs = context.tessellate(gui_output.shapes);

        self.renderpass.update_buffers(device, queue, &paint_jobs, screen_descriptor);

        self.renderpass
            .execute(encoder, output_view, &paint_jobs, screen_descriptor, None)
            .context("Failed to execute egui renderpass!")?;

        // Remove unused textures
        self.renderpass.remove_textures(gui_output.textures_delta).unwrap();

        Ok(())
    }
}
