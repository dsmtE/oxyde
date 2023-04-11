use super::binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc};

use wgpu::{self, BindGroupLayout};

pub struct PingPongBuffer {
    bind_group_layout_builder_descriptor: BindGroupLayoutWithDesc,
    ping_buffer: wgpu::Buffer,
    pong_buffer: wgpu::Buffer,
    pub ping_bind_group: wgpu::BindGroup,
    pub pong_bind_group: wgpu::BindGroup,
    state: bool,
}

impl PingPongBuffer {
    pub fn from_buffer_descriptor(device: &wgpu::Device, descriptor: &wgpu::BufferDescriptor) -> Self {
        // TODO: add suffix to label on descriptor using method map_label
        let ping_buffer = device.create_buffer(descriptor);
        let pong_buffer = device.create_buffer(descriptor);

        let (bind_group_layout_builder_descriptor, ping_bind_group, pong_bind_group) =
            Self::create_layout_and_bind_group(device, &ping_buffer, &pong_buffer, descriptor.label, descriptor.size);

        Self {
            bind_group_layout_builder_descriptor,
            ping_buffer,
            pong_buffer,
            ping_bind_group,
            pong_bind_group,
            state: false,
        }
    }

    pub fn from_buffer_init_descriptor(device: &wgpu::Device, descriptor: &wgpu::util::BufferInitDescriptor) -> Self {
        let ping_buffer = wgpu::util::DeviceExt::create_buffer_init(device, descriptor);
        let pong_buffer = wgpu::util::DeviceExt::create_buffer_init(device, descriptor);

        let (bind_group_layout_builder_descriptor, ping_bind_group, pong_bind_group) =
            Self::create_layout_and_bind_group(device, &ping_buffer, &pong_buffer, descriptor.label, descriptor.contents.len() as u64);

        Self {
            bind_group_layout_builder_descriptor,
            ping_buffer,
            pong_buffer,
            ping_bind_group,
            pong_bind_group,
            state: false,
        }
    }

    pub fn create_layout_and_bind_group(
        device: &wgpu::Device,
        ping_buffer: &wgpu::Buffer,
        pong_buffer: &wgpu::Buffer,
        label: Option<&str>,
        size: u64,
    ) -> (BindGroupLayoutWithDesc, wgpu::BindGroup, wgpu::BindGroup) {
        let label = label.unwrap_or("unknown");

        let bind_group_layout_builder_descriptor = BindGroupLayoutBuilder::new()
            .add_binding_compute(wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size),
            })
            .add_binding_compute(wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size),
            })
            .create(device, Some(format!("{} ping_pong_bind_group_layout", label).as_str()));

        let ping_bind_group = BindGroupBuilder::new(&bind_group_layout_builder_descriptor)
            .resource(ping_buffer.as_entire_binding())
            .resource(pong_buffer.as_entire_binding())
            .create(device, Some(format!("{} ping_bind_group", label).as_str()));

        let pong_bind_group = BindGroupBuilder::new(&bind_group_layout_builder_descriptor)
            .resource(pong_buffer.as_entire_binding())
            .resource(ping_buffer.as_entire_binding())
            .create(device, Some(format!("{} pong_bind_group", label).as_str()));

        (bind_group_layout_builder_descriptor, ping_bind_group, pong_bind_group)
    }
    pub fn get_current_bind_group(&self) -> &wgpu::BindGroup {
        if self.state {
            &self.ping_bind_group
        } else {
            &self.pong_bind_group
        }
    }

    pub fn get_next_bind_group(&self) -> &wgpu::BindGroup {
        if self.state {
            &self.pong_bind_group
        } else {
            &self.ping_bind_group
        }
    }

    pub fn swap_state(&mut self) {
        self.state = !self.state;
    }

    pub fn get_target_buffer(&self) -> &wgpu::Buffer {
        if self.state {
            &self.pong_buffer
        } else {
            &self.ping_buffer
        }
    }

    pub fn layout(&self) -> &BindGroupLayout { &self.bind_group_layout_builder_descriptor.layout }
}
