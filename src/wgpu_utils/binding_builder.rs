pub struct BindGroupLayoutWithDesc {
    pub layout: wgpu::BindGroupLayout,
    pub entries: Vec<wgpu::BindGroupLayoutEntry>,
}

#[derive(Default)]
pub struct BindGroupLayoutBuilder {
    entries: Vec<wgpu::BindGroupLayoutEntry>,
    next_binding_index: u32,
}

impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_raw_binding(mut self, binding: wgpu::BindGroupLayoutEntry) -> Self {
        self.next_binding_index = binding.binding + 1;
        self.entries.push(binding);
        self
    }

    pub fn add_binding(self, visibility: wgpu::ShaderStages, ty: wgpu::BindingType) -> Self {
        let binding: u32 = self.next_binding_index;
        self.add_raw_binding(wgpu::BindGroupLayoutEntry { binding, visibility, ty, count: None })
    }

    // convenient helpers
    pub fn add_binding_compute(self, ty: wgpu::BindingType) -> Self { self.add_binding(wgpu::ShaderStages::COMPUTE, ty) }

    pub fn add_binding_fragment(self, ty: wgpu::BindingType) -> Self { self.add_binding(wgpu::ShaderStages::FRAGMENT, ty) }

    pub fn add_binding_vertex(self, ty: wgpu::BindingType) -> Self { self.add_binding(wgpu::ShaderStages::VERTEX, ty) }

    pub fn add_binding_rendering(self, ty: wgpu::BindingType) -> Self { self.add_binding(wgpu::ShaderStages::VERTEX_FRAGMENT, ty) }

    pub fn create(self, device: &wgpu::Device, label: Option<&str>) -> BindGroupLayoutWithDesc {
        BindGroupLayoutWithDesc {
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &self.entries,
                label: Some(format!("BindGroupLayout: {}", label.unwrap_or("unknown")).as_str()),
            }),
            entries: self.entries,
        }
    }
}

pub struct BindGroupBuilder<'a> {
    layout_with_desc: &'a BindGroupLayoutWithDesc,
    entries: Vec<wgpu::BindGroupEntry<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(layout_with_desc: &'a BindGroupLayoutWithDesc) -> Self { BindGroupBuilder { layout_with_desc, entries: Vec::new() } }

    // Uses same binding index as binding group layout at the same ordering
    pub fn resource(mut self, resource: wgpu::BindingResource<'a>) -> Self {
        assert!(self.entries.len() < self.layout_with_desc.entries.len());
        self.entries.push(wgpu::BindGroupEntry {
            binding: self.layout_with_desc.entries[self.entries.len()].binding,
            resource,
        });
        self
    }

    // convenient helpers
    pub fn sampler(self, sampler: &'a wgpu::Sampler) -> Self { self.resource(wgpu::BindingResource::Sampler(sampler)) }
    pub fn texture(self, texture_view: &'a wgpu::TextureView) -> Self { self.resource(wgpu::BindingResource::TextureView(texture_view)) }

    pub fn create(&self, device: &wgpu::Device, label: Option<&str>) -> wgpu::BindGroup {
        assert_eq!(self.entries.len(), self.layout_with_desc.entries.len());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout_with_desc.layout,
            entries: &self.entries,
            label: Some(format!("BindGroup: {}", label.unwrap_or("unknown")).as_str()),
        })
    }
}
