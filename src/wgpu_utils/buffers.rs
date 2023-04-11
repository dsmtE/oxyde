use super::binding_builder;

pub struct SingleBufferWrapper {
    buffer: wgpu::Buffer,
    bind_group_layout_with_desc: binding_builder::BindGroupLayoutWithDesc,
    bind_group: wgpu::BindGroup,
}

impl SingleBufferWrapper {
    pub fn new(
        device: &wgpu::Device,
        size: u64,
        usages: wgpu::BufferUsages,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        label: Option<&str>
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(format!("{} Buffer", label.unwrap_or("unknown")).as_str()),
            size,
            usage: usages,
            mapped_at_creation: false,
        });

        let (bind_group_layout_with_desc, bind_group) =
            Self::create_layout_and_bind_group(device, &buffer, visibility, ty, has_dynamic_offset, wgpu::BufferSize::new(size), label);

        Self {
            buffer,
            bind_group_layout_with_desc,
            bind_group,
        }
    }

    pub fn new_from_data<T>(
        device: &wgpu::Device,
        slice_content: &[T],
        usages: wgpu::BufferUsages,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        label: Option<&str>
    ) -> Self 
    where T: bytemuck::Pod {
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("{} Buffer", label.unwrap_or("unknown")).as_str()),
                contents: bytemuck::cast_slice(&slice_content),
                usage: usages,
            },
        );

        let min_binding_size = wgpu::BufferSize::new((slice_content.len() * std::mem::size_of::<T>()) as u64);
        let (bind_group_layout_with_desc, bind_group) =
            Self::create_layout_and_bind_group(device, &buffer, visibility, ty, has_dynamic_offset, min_binding_size, label);

        Self {
            buffer,
            bind_group_layout_with_desc,
            bind_group,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind_group }

    pub fn buffer(&self) -> &wgpu::Buffer { &self.buffer }

    pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.bind_group_layout_with_desc.layout }

    fn create_layout_and_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<wgpu::BufferSize>,
        label: Option<&str>,
    ) -> (binding_builder::BindGroupLayoutWithDesc, wgpu::BindGroup) {
        let label = label.unwrap_or("unknown");

        let bind_group_layout_with_desc = binding_builder::BindGroupLayoutBuilder::new()
        .add_binding(visibility, wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset,
            min_binding_size,
        })
        .create(device, Some(format!("{} BindGroupLayout", label).as_str()));

        
        let bind_group = binding_builder::BindGroupBuilder::new(&bind_group_layout_with_desc)
        .resource(buffer.as_entire_binding())
        .create(device, Some(format!("{} BindGroup", label).as_str()));

        (bind_group_layout_with_desc, bind_group)
    }
}