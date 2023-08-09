/*
This file creates some helper functions to make bind group stuff a little bit less verbose
at least that is the intention.
 */



pub struct BindGrouperoo{
    pub bind_group_layout: wgpu::BindGroupLayout ,
    pub bind_group : wgpu::BindGroup,
}

impl BindGrouperoo {
    pub fn new(
        device: &wgpu::Device,
        visibility: &[wgpu::ShaderStages],
        uniform_buffers: &[&wgpu::Buffer],
        label: Option<&str>
    )->Self {
        let mut bind_group_layouts: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();
        let mut bind_group_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        for i in 0..visibility.len() {
            let layout_entry = wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility:visibility[i],
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            };
            bind_group_layouts.push(layout_entry);

        }
        let bind_group_layout = device.create_bind_group_layout( & wgpu::BindGroupLayoutDescriptor {
            entries: & bind_group_layouts.as_slice(),
            label
            }
        );

        for i in 0..uniform_buffers.len() {
            let bind_group_entry = wgpu::BindGroupEntry {
                binding: i as u32,
                resource: uniform_buffers[i].as_entire_binding(),
            };
           bind_group_entries.push(bind_group_entry);
        }

        let bind_group_descriptor = wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: bind_group_entries.as_slice(),
            label: Some("uniform_bind_group"),
        };

        let bind_group = device.create_bind_group(
             &bind_group_descriptor
        );
        Self{bind_group_layout,bind_group}

    }


}