use egui::epaint::Shadow;
use egui::{Context,Visuals};
use egui_wgpu::renderer::ScreenDescriptor;
use egui_wgpu::Renderer;

use egui_winit::State;

use wgpu::{CommandEncoder,Device,Queue,TextureFormat,TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiRenderer {
    pub context: Context,
    state: State,
    renderer : Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format : TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
       todo!("Gotta create this!")
    }
}