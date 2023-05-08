use super::binding_builder;

use wgpu::{
    Device,
    BufferDescriptor,
    BufferUsages,
    Buffer,
    BufferAddress,
    CommandEncoder,
    Queue,
};

use std::mem::size_of;

#[deprecated(note = "better to use binding_builder helpers to create bind_group manually")]
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

pub enum StagingBufferAccess {
    Read,
    Write,
}
// Buffer wrapper for a GPU buffer that can be read or write from the CPU (using intermediate staging buffer)
pub struct StagingBufferWrapper<T: bytemuck::Pod, const READ_OR_WRITE: bool> {
    values: Vec<T>,
    staging_buffer: Buffer,
}

pub fn create_staging_buffer(device: &Device, read_or_write: bool, size: BufferAddress) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: None,
        size,
        usage: BufferUsages::COPY_DST | match read_or_write {
            true => BufferUsages::MAP_READ,
            false => BufferUsages::COPY_SRC,
        },
        mapped_at_creation: false,
    })
}

impl<T: bytemuck::Pod, const READ_OR_WRITE: bool> StagingBufferWrapper<T, READ_OR_WRITE> {
    pub fn new(device: &Device, size: usize) -> Self {
        let buffer_size = (size * size_of::<T>()) as BufferAddress;
        Self {
            values: vec![T::zeroed(); size],
            staging_buffer: create_staging_buffer(device, READ_OR_WRITE, buffer_size),
        }
    }

    pub fn new_from_data(device: &Device, slice_content: &[T]) -> Self {
        let size = slice_content.len();
        let buffer_size = (size * size_of::<T>()) as BufferAddress;
        Self {
            values: Vec::from(slice_content),
            staging_buffer: create_staging_buffer(device, READ_OR_WRITE, buffer_size),
        }
    }

    pub fn encode_write(&mut self, queue: &Queue, command_encoder: &mut CommandEncoder, buffer: &Buffer) {
        // buffer.usage().contains(BufferUsages::COPY_DST);
        let bytes_size = self.bytes_size();
        let bytes: &[u8] = bytemuck::cast_slice(&self.values);
        queue.write_buffer(&self.staging_buffer, 0, &bytes[0..bytes_size]);
        command_encoder.copy_buffer_to_buffer(&self.staging_buffer, 0, buffer, 0, bytes_size as BufferAddress);
    }

    // TODO: find a better way to expose only write or read function (without having to use a const generic bool )
    // maybe trait ?
    pub fn encode_read(&mut self, command_encoder: &mut CommandEncoder, buffer: &Buffer) {
        command_encoder.copy_buffer_to_buffer(buffer, 0, &self.staging_buffer, 0, self.bytes_size() as BufferAddress);
    }

    pub fn map_buffer(&mut self) {
        self.staging_buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    }

    pub fn read_and_unmap_buffer(&mut self) {
        let bytes_size = self.bytes_size();
        let buffer_slice = self.staging_buffer.slice(..);
        self.values.copy_from_slice(bytemuck::cast_slice(&buffer_slice.get_mapped_range()[0..bytes_size]));
        self.staging_buffer.unmap();
    }

    #[inline] pub fn len(&self) -> usize { self.values.len() }
    #[inline] pub fn bytes_size(&self) -> usize { self.len() * size_of::<T>() }
    #[inline] pub fn values_as_slice(&self) -> &[T] { self.values.as_slice() }
    #[inline] pub fn values_as_slice_mut(&mut self) -> &mut [T] { self.values.as_mut_slice() }
    #[inline] pub fn iter(&self) -> core::slice::Iter<'_, T> { self.values.iter() }
    #[inline] pub fn clear(&mut self) { self.values.fill(T::zeroed()); }

}

impl<T: bytemuck::Pod, const READ_OR_WRITE: bool> std::ops::Index<usize> for StagingBufferWrapper<T, READ_OR_WRITE> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}