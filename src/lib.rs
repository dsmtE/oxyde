mod app;
mod egui_wgpu_renderer;
mod input;
pub mod wgpu_utils;

pub use app::*;
pub use input::InputsState;

pub extern crate egui;
pub extern crate wgpu;
pub extern crate winit;
pub extern crate bytemuck;

pub extern crate anyhow;

#[cfg(feature = "log")]
#[macro_use]
pub extern crate log;

#[cfg(feature = "log")]
mod logging;

#[cfg(feature = "log")]
pub use logging::*;
