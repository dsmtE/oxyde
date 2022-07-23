use super::binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc};

pub struct PingPongTexture {
    label: &'static str,
    view_ping: wgpu::TextureView,
    view_pong: wgpu::TextureView,
    pub bind_group_layout: BindGroupLayoutWithDesc,
    pub state: bool,
}

impl PingPongTexture {
    pub fn from_descriptor(
        device: &wgpu::Device,
        descriptor: &wgpu::TextureDescriptor,
        optional_label: Option<&'static str>, // Optional debug label. This will show up in graphics debuggers for easy identification.
    ) -> Result<Self, wgpu::Error> {
        let texture_ping = device.create_texture(&descriptor);
        let texture_pong = device.create_texture(&descriptor);
        let view_ping = texture_ping.create_view(&wgpu::TextureViewDescriptor::default());
        let view_pong = texture_pong.create_view(&wgpu::TextureViewDescriptor::default());

        let label = optional_label.unwrap_or("");
        let bind_group_layout = BindGroupLayoutBuilder::new()
            .add_binding_fragment(wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            })
            .add_binding_fragment(wgpu::BindingType::Sampler { 0: wgpu::SamplerBindingType::Filtering })
            .create(device, format!("BindGroupLayout: {}", label).as_str());

        Ok(Self {
            label,
            view_ping,
            view_pong,
            bind_group_layout,
            state: false,
        })
    }

    pub fn create_binding_group(&self, device: &wgpu::Device, sampler: &wgpu::Sampler) -> (wgpu::BindGroup, wgpu::BindGroup) {
        let bind_group_ping = BindGroupBuilder::new(&self.bind_group_layout)
            .texture(&self.view_ping)
            .sampler(sampler)
            .create(device, format!("BindGroup(ping): {}", self.label).as_str());

        let bind_group_pong = BindGroupBuilder::new(&self.bind_group_layout)
            .texture(&self.view_pong)
            .sampler(sampler)
            .create(device, format!("BindGroup(pong): {}", self.label).as_str());

        (bind_group_ping, bind_group_pong)
    }

    pub fn toogle_state(&mut self) { self.state = !self.state; }

    pub fn get_target_texture_view(&self) -> &wgpu::TextureView {
        if self.state {
            &self.view_ping
        } else {
            &self.view_pong
        }
    }

    pub fn get_rendered_texture_view(&self) -> &wgpu::TextureView {
        if !self.state {
            &self.view_ping
        } else {
            &self.view_pong
        }
    }
}
