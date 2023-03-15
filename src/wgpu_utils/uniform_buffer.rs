// good wrapper taken from Wumpf in his project blub (https://github.com/Wumpf/blub)
use std::marker::PhantomData;

pub struct UniformBuffer<Content> {
    buffer: wgpu::Buffer,
    content_type: PhantomData<Content>,
    previous_content: Vec<u8>,
}

impl<Content: bytemuck::Pod> UniformBuffer<Content> {
    fn name() -> &'static str {
        let type_name = std::any::type_name::<Content>();
        let pos = type_name.rfind(':').unwrap();
        &type_name[(pos + 1)..]
    }

    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: Vec::new(),
        }
    }

    pub fn new_with_data(device: &wgpu::Device, initial_content: &Content) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("UniformBuffer: {}", Self::name())),
            size: std::mem::size_of::<Content>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        let mapped_memory = buffer.slice(..);
        mapped_memory.get_mapped_range_mut().clone_from_slice(bytemuck::bytes_of(initial_content));
        buffer.unmap();

        UniformBuffer {
            buffer,
            content_type: PhantomData,
            previous_content: bytemuck::bytes_of(initial_content).to_vec(),
        }
    }

    pub fn update_content(&mut self, queue: &wgpu::Queue, content: Content) {
        let new_content = bytemuck::bytes_of(&content);
        if self.previous_content == new_content {
            return;
        }
        // Could do partial updates since we know the previous state.
        queue.write_buffer(&self.buffer, 0, new_content);
        self.previous_content = new_content.to_vec();
    }

    pub fn binding_resource(&self) -> wgpu::BindingResource { self.buffer.as_entire_binding() }
}


pub struct UniformBufferWrapper<Content> {
    content: Content,
    uniform_buffer: UniformBuffer<Content>,
    bind_group_layout_with_desc: super::binding_builder::BindGroupLayoutWithDesc,
    bind_group: wgpu::BindGroup,
}

impl<Content: bytemuck::Pod> UniformBufferWrapper<Content> {
    pub fn new(device: &wgpu::Device, content: Content, visibility: wgpu::ShaderStages) -> Self {
        let uniform_buffer = UniformBuffer::new_with_data(device, &content);

        let bind_group_layout_with_desc = super::binding_builder::BindGroupLayoutBuilder::new()
            .add_binding(visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Content>() as _),
            })
            .create(device, Some(&format!("BindGroupLayout: {}", UniformBuffer::<Content>::name())));

        let bind_group = super::binding_builder::BindGroupBuilder::new(&bind_group_layout_with_desc)
            .resource(uniform_buffer.binding_resource())
            .create(device, Some(&format!("BindGroup: {}", UniformBuffer::<Content>::name())));

        UniformBufferWrapper {
            content,
            uniform_buffer,
            bind_group_layout_with_desc,
            bind_group,
        }
    }

    pub fn update_content(&mut self, queue: &wgpu::Queue) {
        self.uniform_buffer.update_content(queue, self.content);
    }

    pub fn content(&mut self) -> &mut Content { &mut self.content }

    pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind_group }

    pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.bind_group_layout_with_desc.layout }
}