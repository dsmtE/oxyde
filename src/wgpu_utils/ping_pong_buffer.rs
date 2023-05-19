use super::binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder, BindGroupLayoutWithDesc};

use wgpu::{self, BindGroupLayout};

pub struct PingPongBuffer {
    ping_buffer: wgpu::Buffer,
    pong_buffer: wgpu::Buffer,
    ping_pong_bind_group_layout_builder_descriptor: BindGroupLayoutWithDesc,
    ping_pong_bind_group: wgpu::BindGroup,
    pong_ping_bind_group: wgpu::BindGroup,
    single_buffer_bind_group_layout_builder_descriptor: BindGroupLayoutWithDesc,
    ping_bind_group: wgpu::BindGroup,
    pong_bind_group: wgpu::BindGroup,
    state: bool,
}

impl PingPongBuffer {
    pub fn from_buffer_descriptor(
        device: &wgpu::Device,
        descriptor: &wgpu::BufferDescriptor,
        single_buffer_visibility: wgpu::ShaderStages,
        ping_pong_buffer_visibility: wgpu::ShaderStages,
    ) -> Self {
        // TODO: add suffix to label on descriptor using method map_label
        let ping_buffer = device.create_buffer(descriptor);
        let pong_buffer = device.create_buffer(descriptor);

        let (
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            single_buffer_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
        ) = Self::create_layout_and_bind_group(
            device,
            &ping_buffer,
            &pong_buffer,
            single_buffer_visibility,
            ping_pong_buffer_visibility,
            descriptor.label,
            descriptor.size,
        );

        Self {
            ping_buffer,
            pong_buffer,
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            single_buffer_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
            state: false,
        }
    }

    pub fn from_buffer_init_descriptor(
        device: &wgpu::Device,
        descriptor: &wgpu::util::BufferInitDescriptor,
        single_buffer_visibility: wgpu::ShaderStages,
        ping_pong_buffer_visibility: wgpu::ShaderStages,
    ) -> Self {
        let ping_buffer = wgpu::util::DeviceExt::create_buffer_init(device, descriptor);
        let pong_buffer = wgpu::util::DeviceExt::create_buffer_init(device, descriptor);

        let (
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            single_buffer_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
        ) = Self::create_layout_and_bind_group(
            device,
            &ping_buffer,
            &pong_buffer,
            single_buffer_visibility,
            ping_pong_buffer_visibility,
            descriptor.label,
            descriptor.contents.len() as u64,
        );

        Self {
            ping_buffer,
            pong_buffer,
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            single_buffer_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
            state: false,
        }
    }

    pub fn create_layout_and_bind_group(
        device: &wgpu::Device,
        ping_buffer: &wgpu::Buffer,
        pong_buffer: &wgpu::Buffer,
        single_buffer_visibility: wgpu::ShaderStages,
        ping_pong_buffer_visibility: wgpu::ShaderStages,
        label: Option<&str>,
        size: u64,
    ) -> (
        BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
        BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
    ) {
        let label = label.unwrap_or("unknown");

        let ping_pong_bind_group_layout_builder_descriptor = BindGroupLayoutBuilder::new()
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size),
                },
            )
            .create(device, Some(format!("{} ping_pong_bind_group_layout", label).as_str()));

        let ping_pong_bind_group = BindGroupBuilder::new(&ping_pong_bind_group_layout_builder_descriptor)
            .resource(ping_buffer.as_entire_binding())
            .resource(pong_buffer.as_entire_binding())
            .create(device, Some(format!("{} ping_pong_bind_group", label).as_str()));

        let pong_ping_bind_group = BindGroupBuilder::new(&ping_pong_bind_group_layout_builder_descriptor)
            .resource(pong_buffer.as_entire_binding())
            .resource(ping_buffer.as_entire_binding())
            .create(device, Some(format!("{} pong_ping_bind_group", label).as_str()));

        let single_buffer_bind_group_layout_builder_descriptor = BindGroupLayoutBuilder::new()
            .add_binding(
                single_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size),
                },
            )
            .create(device, Some(format!("{} buffer_bind_group_layout", label).as_str()));

        let ping_bind_group = BindGroupBuilder::new(&single_buffer_bind_group_layout_builder_descriptor)
            .resource(ping_buffer.as_entire_binding())
            .create(device, Some(format!("{} ping_bind_group", label).as_str()));

        let pong_bind_group = BindGroupBuilder::new(&single_buffer_bind_group_layout_builder_descriptor)
            .resource(pong_buffer.as_entire_binding())
            .create(device, Some(format!("{} pong_bind_group", label).as_str()));

        (
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            single_buffer_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
        )
    }
    pub fn get_current_ping_pong_bind_group(&self) -> &wgpu::BindGroup {
        if self.state {
            &self.ping_pong_bind_group
        } else {
            &self.pong_ping_bind_group
        }
    }

    pub fn swap_state(&mut self) { self.state = !self.state; }

    pub fn get_current_source_bind_group(&self) -> &wgpu::BindGroup {
        if self.state {
            &self.ping_bind_group
        } else {
            &self.pong_bind_group
        }
    }

    pub fn get_current_target_bind_group(&self) -> &wgpu::BindGroup {
        if self.state {
            &self.pong_bind_group
        } else {
            &self.ping_bind_group
        }
    }

    pub fn get_current_source_buffer(&self) -> &wgpu::Buffer {
        if self.state {
            &self.ping_buffer
        } else {
            &self.pong_buffer
        }
    }

    pub fn get_current_target_buffer(&self) -> &wgpu::Buffer {
        if self.state {
            &self.pong_buffer
        } else {
            &self.ping_buffer
        }
    }

    pub fn get_ping_pong_bind_group_layout(&self) -> &BindGroupLayout { &self.ping_pong_bind_group_layout_builder_descriptor.layout }
    pub fn get_buffer_bind_group_layout(&self) -> &BindGroupLayout { &self.single_buffer_bind_group_layout_builder_descriptor.layout }
}
