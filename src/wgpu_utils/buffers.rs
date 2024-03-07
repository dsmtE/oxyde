use wgpu::{Buffer, BufferAddress, BufferDescriptor, BufferUsages, CommandEncoder, Device, Queue};

// Buffer wrapper for a GPU buffer that can be read or write from the CPU (using intermediate staging buffer)
pub struct StagingBufferWrapper<T: bytemuck::Pod, const READ_OR_WRITE: bool> {
    values: Vec<T>,
    staging_buffer: Buffer,
}

pub fn create_buffer_for_size(device: &Device, usage: BufferUsages, label: Option<&str>, size: BufferAddress) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label,
        size,
        usage,
        mapped_at_creation: false,
    })
}

pub fn create_buffer_from_content(device: &Device, usage: BufferUsages, label: Option<&str>, content: Option<&[u8]>) -> Buffer {
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label,
            contents: content.unwrap_or(&[0u8; 0]),
            usage,
        },
    )
}

impl<T: bytemuck::Pod, const READ_OR_WRITE: bool> StagingBufferWrapper<T, READ_OR_WRITE> {
    pub fn new(device: &Device, size: usize) -> Self {
        let usages =  BufferUsages::COPY_DST | match READ_OR_WRITE {
            true => BufferUsages::MAP_READ,
            false => BufferUsages::COPY_SRC,
        };
        Self {
            values: vec![T::zeroed(); size],
            staging_buffer: create_buffer_for_size(device, usages, None, (size * std::mem::size_of::<T>()) as BufferAddress),
        }
    }

    pub fn new_from_data(device: &Device, slice_content: &[T]) -> Self {
        let usages =  BufferUsages::COPY_DST | match READ_OR_WRITE {
            true => BufferUsages::MAP_READ,
            false => BufferUsages::COPY_SRC,
        };
        Self {
            values: Vec::from(slice_content),
            staging_buffer: create_buffer_from_content(device, usages, None, Some(bytemuck::cast_slice(slice_content))),
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
    
    pub fn map_buffer(
        &mut self,
        callback: Option<impl FnOnce(Result<(), wgpu::BufferAsyncError>) + wgpu::WasmNotSend + 'static>
    ) {

        if let Some(callback) = callback {
            self.staging_buffer.slice(..).map_async(wgpu::MapMode::Read, callback);
        } else {
            self.staging_buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        }
    }

    pub fn read_and_unmap_buffer(&mut self) {
        let bytes_size = self.bytes_size();
        let buffer_slice = self.staging_buffer.slice(..);
        self.values
            .copy_from_slice(bytemuck::cast_slice(&buffer_slice.get_mapped_range()[0..bytes_size]));
        self.staging_buffer.unmap();
    }

    #[inline]
    pub fn len(&self) -> usize { self.values.len() }
    #[inline]
    pub fn is_empty(&self) -> bool { self.values.is_empty() }
    #[inline]
    pub fn bytes_size(&self) -> usize { self.len() * std::mem::size_of::<T>() }
    #[inline]
    pub fn values_as_slice(&self) -> &[T] { self.values.as_slice() }
    #[inline]
    pub fn values_as_slice_mut(&mut self) -> &mut [T] { self.values.as_mut_slice() }
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, T> { self.values.iter() }
    #[inline]
    pub fn clear(&mut self) { self.values.fill(T::zeroed()); }
}

impl<T: bytemuck::Pod, const READ_OR_WRITE: bool> std::ops::Index<usize> for StagingBufferWrapper<T, READ_OR_WRITE> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output { &self.values[index] }
}
