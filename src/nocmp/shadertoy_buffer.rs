/*
This creates a pipeline with a render-texture like a buffer in shadertoy.
 */
use anyhow::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32;3],
    color: [f32;3],
}


#[repr(C)]
#[derive(Debug,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
struct Uniforms{
    iMouse:[f32;4],
    iResolution:[f32;2],
    iTime:f32,
    pad1:f32,
}

impl Uniforms{
    fn new()->Self {
        Uniforms{
            iMouse: [0.0,0.0,0.0,0.0],
            iTime: 0.0,
            iResolution: [0.0,0.0],
            pad1: 0.0,
        }
    }
}


impl Vertex {
    fn desc()-> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32;3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }
            ]
        }
    }

    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3,1 => Float32x3];

    fn descy() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTICES: &[Vertex]= &[
    Vertex { position: [-1.0, -1.0, 0.0], color: [1.0, 0.0, 0.0] }, //Bottom left
    Vertex { position: [1.0, -1.0, 0.0], color: [0.0, 1.0, 0.0] }, //Bottom right
    Vertex { position: [-1.0, 1.0, 0.0], color: [0.0, 0.0, 1.0] },//Upper left
    Vertex { position: [1.0, 1.0, 0.0], color: [0.0, 0.0, 1.0] },//Upper right
];

const INDICES: &[u16] = &[
    0,1,2,
    2,1,3
];
pub(crate) struct ShaderToylikeBuffer{

    render_pipeline_rtt: wgpu::RenderPipeline,
    //can probably be shared
    uniform_buffer:wgpu::Buffer,
    uniform_bind_group:wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniforms:Uniforms,
    texture_rtt:wgpu::Texture,
    diffuse_bind_group:wgpu::BindGroup,
    num_indices: u32,
}

impl ShaderToylikeBuffer{

    pub fn create(
        device: &wgpu::Device,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        surface_config: &wgpu::SurfaceConfiguration,
    ) ->Result<Self>{

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shadertoys/shader_buffer_a.wgsl"));

        let uniforms = Uniforms::new();

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility:wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count:None,
                }
            ],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding:0,
                    resource:uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("uniform_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers:&[
                    Vertex::descy(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask:wgpu::ColorWrites::ALL,
                })],
            }),

            primitive:wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                //Setting this to anything other than Fill requires
                //Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                //Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                //Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil:None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        todo!("create this");
        /*
        Ok(Self{
            render_pipeline_rtt,
            uniform_bind_group,
            diffuse_bind_group,
            vertex_buffer,
            index_buffer,
            num_indices,
            texture_rtt,

        })*/
        /*Ok(Self{

        })*/
    }

    pub fn render(self: &Self, encoder: &mut wgpu::CommandEncoder)
    {
        let texture_view_rtt = self.texture_rtt.create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("My First Render Pass to RTT"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment{
        view: &texture_view_rtt,
        resolve_target: None,
        ops: wgpu::Operations {
        load: wgpu::LoadOp::Clear(wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
        }),
        store: true,
        },
        })],
        depth_stencil_attachment: None,
        });


        render_pass.set_pipeline(&self.render_pipeline_rtt);
        render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
        render_pass.set_bind_group(1,&self.diffuse_bind_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
        //render_pass.draw(0..self.num_vertices,0..1);
    }
}