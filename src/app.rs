use std::sync::Arc;
use winit::{
    event::{self, ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    keyboard,
    window::{Window, WindowBuilder},
};

use anyhow::Result;

use crate::{
    egui_wgpu_renderer::EguiRenderer,
    input::{InputsState, SystemState, WinitEventHandler},
    wgpu_utils::render_handles::{RenderInstance, SurfaceHandle, DeviceHandle},
};

use egui_wgpu::ScreenDescriptor;

pub struct AppState {
    pub window: std::sync::Arc<Window>,

    pub render_instance: RenderInstance,
    pub surface_handle: SurfaceHandle<'static>,

    pub clear_color: wgpu::Color,

    pub egui_renderer: EguiRenderer,

    pub input_state: InputsState,
    pub system_state: SystemState,

    pub control_flow: ControlFlow,

    last_frame_time: std::time::Instant,
    target_frame_duration: std::time::Duration,
}

impl AppState {
    pub fn set_fullscreen(&mut self) {
        self.window
            .set_fullscreen(Some(winit::window::Fullscreen::Borderless(self.window.primary_monitor())));
    }

    pub fn set_target_fps(&mut self, fps: u32) { self.target_frame_duration = std::time::Duration::from_micros((1_000_000.0 / fps as f32) as u64); }
}

pub trait App {
    fn create(_app_state: &mut AppState) -> Self;

    fn update(&mut self, _app_state: &mut AppState) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _app_state: &mut AppState) -> Result<()> { Ok(()) }

    fn render(&mut self, _app_state: &mut AppState, _output_view: &wgpu::TextureView) -> Result<()> { Ok(()) }
    // fn called after queue submit
    fn post_render(&mut self, _app_state: &mut AppState) -> Result<()> { Ok(()) }

    fn cleanup(&mut self) -> Result<()> { Ok(()) }

    fn on_mouse(&mut self, _app_state: &mut AppState, _button: &MouseButton, _button_state: &ElementState) -> Result<()> { Ok(()) }
    fn on_key(&mut self, _app_state: &mut AppState, _event: &event::KeyEvent) -> Result<()> { Ok(()) }

    fn handle_event<T: 'static>(&mut self, _app_state: &mut AppState, _event: &Event<T>) -> Result<()> { Ok(()) }
}

pub struct AppConfig {
    pub is_resizable: bool,
    pub title: &'static str,
    #[cfg(feature = "icon")]
    pub icon: Option<&'static str>,
    pub control_flow: ControlFlow,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            is_resizable: false,
            title: "Application",
            #[cfg(feature = "icon")]
            icon: None,
            control_flow: ControlFlow::Poll,
        }
    }
}

pub struct RenderingConfig {
    pub power_preference: wgpu::PowerPreference,
    pub device_features: wgpu::Features,
    pub device_limits: wgpu::Limits,
    pub backend: wgpu::Backends,
    pub window_surface_present_mode: wgpu::PresentMode,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::default(),
            device_features: wgpu::Features::default(),
            device_limits: wgpu::Limits::default(),
            backend: wgpu::Backends::PRIMARY,
            // FIFO, will cap the display rate at the displays framerate. This is essentially VSync.
            // https://docs.rs/wgpu/0.10.1/wgpu/enum.PresentMode.html
            window_surface_present_mode: wgpu::PresentMode::Fifo,
        }
    }
}

pub fn run_application<T: App + 'static>(app_config: AppConfig, rendering_config: RenderingConfig) -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    #[allow(unused_mut)]
    let mut window_builder: WindowBuilder = WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(app_config.is_resizable)
        .with_transparent(false)
        .with_title(app_config.title);

    #[cfg(feature = "icon")]
    if let Some(icon_path) = app_config.icon {
        let image = image::io::Reader::open(icon_path)?.decode()?.into_rgba8();
        let (width, height) = image.dimensions();
        let icon = winit::window::Icon::from_rgba(image.into_raw(), width, height)?;
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    // if let Some(default_dimension) = config.default_dimension {
    //     let (width, height) = default_dimension;
    //     window_builder = window_builder.with_inner_size(PhysicalSize::new(width, height));
    // }

    let window = Arc::new(window_builder.build(&event_loop)?);

    let window_dimensions = window.inner_size();


    let mut render_instance = RenderInstance::new(Some(rendering_config.backend), None);
    let mut surface_handle = pollster::block_on(render_instance.create_render_surface(
        window.clone(),
        window_dimensions.width,
        window_dimensions.height,
        rendering_config.window_surface_present_mode,
        None,
    ))?;

    let surface_device_handle = &render_instance.devices[surface_handle.device_handle_id];
    
    surface_handle.set_present_mode(&surface_device_handle.device, rendering_config.window_surface_present_mode);

    let egui_renderer = EguiRenderer::new(&surface_device_handle.device, surface_handle.format(), None, 1, &window);

    let mut app_state = AppState {
        window,

        render_instance,
        surface_handle,

        clear_color: wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 },

        egui_renderer,

        input_state: InputsState::default(),
        system_state: SystemState::new(window_dimensions),

        control_flow: app_config.control_flow,

        last_frame_time: std::time::Instant::now(),
        target_frame_duration: std::time::Duration::from_micros(16_666),
    };

    let (tx, rx) = std::sync::mpsc::channel::<wgpu::Error>();
    app_state.render_instance.device_from_surface_handle(&app_state.surface_handle).device.on_uncaptured_error(Box::new(move |e: wgpu::Error| {
        tx.send(e).expect("sending error failed");
    }));

    let mut app = T::create(&mut app_state);

    app_state.render_instance.device_from_surface_handle(&app_state.surface_handle).device.on_uncaptured_error(Box::new(|err| panic!("{}", err)));

    if let Ok(err) = rx.try_recv() {
        panic!("{}", err);
    }

    // Run
    event_loop.run(move |event, elwt| {
        if let Err(error) = run_loop(&mut app, &mut app_state, event, elwt) {
            eprintln!("Application Error: {}", error);
        }
    })?;

    Ok(())
}

fn run_loop<T: 'static>(app: &mut impl App, app_state: &mut AppState, event: Event<T>, elwt: &EventLoopWindowTarget<T>) -> Result<()> {
    app_state.input_state.handle_event(&event);
    app_state.system_state.handle_event(&event);

    if let Event::WindowEvent { event: window_event, .. } = &event {
        let _ = app_state.egui_renderer.handle_window_event(&app_state.window, window_event);
    }

    app.handle_event(app_state, &event)?;

    match event {
        Event::WindowEvent { ref event, .. } => match event {
            WindowEvent::Resized(physical_size) => {
                // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                // See: https://github.com/rust-windowing/winit/issues/208
                // This solves an issue where the app would panic when minimizing on Windows.
                if physical_size.width > 0 && physical_size.height > 0 {
                    let surface_device = &app_state.render_instance.device_from_surface_handle(&app_state.surface_handle).device;
                    app_state.surface_handle.resize(surface_device, physical_size.width, physical_size.height)?;
                    // On macos the window needs to be redrawn manually after resizing
                    app_state.window.request_redraw();
                }
            },
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    event::KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: keyboard::PhysicalKey::Code(keyboard::KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                elwt.exit();
            },
            WindowEvent::MouseInput { button, state, .. } => app.on_mouse(app_state, button, state)?,
            WindowEvent::KeyboardInput { event, .. } => {
                app.on_key(app_state, event)?;
            },
            WindowEvent::RedrawRequested => {
                match app_state.surface_handle.get_current_texture() {
                    Ok(output) => {
                        render_app(app, app_state, output)?;
                    },
                    // TODO: Reconfigure the surface if lost
                    // Err(wgpu::SurfaceError::Lost) => { }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }

                app.post_render(app_state)?;
            },
            _ => (),
        },
        Event::AboutToWait => {
            app.update(app_state)?;

            let now = std::time::Instant::now();
            let next_frame_time = app_state.last_frame_time + app_state.target_frame_duration;
            if now > next_frame_time {
                #[cfg(feature = "log")]
                log::warn!(
                    "We are running behind the target frame rate of {:.0} fps (current frame took {:?} (~ {:.0} fps ))",
                    1.0 / app_state.target_frame_duration.as_secs_f32(),
                    now - app_state.last_frame_time,
                    1.0 / (now - app_state.last_frame_time).as_secs_f32()
                );
                
            } else {
                spin_sleep::sleep(next_frame_time.duration_since(now));
            }
            app_state.last_frame_time = std::time::Instant::now();

            app_state.window.request_redraw();
        },
        Event::LoopExiting => {
            app.cleanup()?;
        },
        _ => (),
    }

    Ok(())
}

pub fn render_app(app: &mut impl App, app_state: &mut AppState, output: wgpu::SurfaceTexture) -> Result<()> {
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

    app.render(app_state, &view)?;

    // draw UI
    app_state.egui_renderer.begin_frame(&app_state.window);
    app.render_gui(app_state)?;
    let egui_output = app_state.egui_renderer.end_frame();

    // let window_dimensions = app_state.window.inner_size();

    let output_size = output.texture.size();

    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [output_size.width, output_size.height],
        pixels_per_point: app_state.window.scale_factor() as f32,
    };

    let DeviceHandle { device: surface_device, queue: surface_queue, .. } = app_state.render_instance.device_from_surface_handle(&app_state.surface_handle);

    let mut egui_encoder = surface_device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render UI Encoder") });
    app_state.egui_renderer.draw_output(
        egui_output,
        surface_device,
        surface_queue,
        &mut egui_encoder,
        &app_state.window,
        &view,
        screen_descriptor,
    );
    surface_queue.submit(Some(egui_encoder.finish()));

    output.present();

    Ok(())
}

// Update the viewport of the render pass to match the available rect of the gui
pub fn fit_viewport_to_gui_available_rect(render_pass: &mut wgpu::RenderPass, _app_state: &AppState) {
    let window_scale_factor = _app_state.window.scale_factor() as f32;
    // // It must be multiplied by window scale factor as render pass use physical pixels screen size
    let available_rect = _app_state.egui_renderer.context().available_rect();
    let available_rect_size = available_rect.size();

    render_pass.set_viewport(
        available_rect.min.x * window_scale_factor,
        available_rect.min.y * window_scale_factor,
        available_rect_size.x * window_scale_factor,
        available_rect_size.y * window_scale_factor,
        0.0,
        1.0,
    );
}
