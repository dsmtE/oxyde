mod app;
mod egui_wgpu_renderer;
mod input;
pub mod wgpu_utils;

pub use app::*;
pub use input::InputsState;

pub extern crate egui;
pub extern crate wgpu;
pub extern crate winit;

#[macro_use]
extern crate log;
