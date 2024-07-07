/*
This creates a pipeline with a render-texture like a buffer in shadertoy.
 */
use anyhow::*;
use wgpu::StoreOp;
use wgpu::util::DeviceExt;
use crate::nocmp;
use crate::nocmp::camera::CameraUniform;

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32;3],
    color: [f32;3],
}


#[repr(C)]
#[derive(Debug,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
pub struct Uniforms{
    pub iMouse:[f32;4],
    pub iResolution:[f32;2],
    pub iTime:f32,
    pub pad1:f32,
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

//Typically owned by "app and shared between shadertoylike buffers
pub struct ShaderToyUniforms{
    pub uniform_buffer:wgpu::Buffer,
    pub uniform_bind_group:wgpu::BindGroup,
    pub uniform_bind_group_layout :wgpu::BindGroupLayout,
   pub uniforms:Uniforms,
}

impl ShaderToyUniforms {
    pub fn new(
        device: &wgpu::Device,
    )->Result<Self> {

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
        Ok(Self{
            uniform_bind_group,
            uniform_bind_group_layout,
            uniform_buffer,
            uniforms,
        })
    }

    pub fn uni(self:&mut Self)->&mut Uniforms{
       &mut self.uniforms
    }

    pub fn push_buffer_to_gfx_card(self: &Self,queue: &wgpu::Queue){
        queue.write_buffer(&self.uniform_buffer,0,bytemuck::cast_slice(&[self.uniforms]));
    }
}

pub struct ShaderToylikeBuffer{

    render_pipeline: wgpu::RenderPipeline,
    target_rtt : nocmp::texture::Texture,
    target_rtt_bindgroup : wgpu::BindGroup,
    //can probably be shared
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}


impl ShaderToylikeBuffer{

    pub fn create(
        device: &wgpu::Device,
        toylike_uniforms: &ShaderToyUniforms,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_descriptor: wgpu::ShaderModuleDescriptor,
    ) ->Result<Self>{

        //let shader = device.create_shader_module(wgpu::include_wgsl!("../shadertoys/shader_buffer_a.wgsl"));
        let shader = device.create_shader_module(shader_descriptor);

        let target_rtt = nocmp::texture::Texture::create_rtt_texture(4096_u32*2,4096_u32*2,&device,Some("target rtt texture")).unwrap();
        let (bind_group_layout,target_rtt_bindgroup) = nocmp::texture::setup_texture_stage(
            &device,
            &[&target_rtt],
            Some("target_rtt")
        ).unwrap();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    //&uniform_bind_group_layout,
                    &toylike_uniforms.uniform_bind_group_layout,
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
                compilation_options: Default::default(),
                buffers:&[
                    Vertex::descy(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
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


        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );


        let num_indices= INDICES.len() as u32;

        Ok(Self{
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            target_rtt,
            target_rtt_bindgroup,
        })
    }

    pub fn get_target_rtt_bindgroup(self: &Self) -> &wgpu::BindGroup{
        &self.target_rtt_bindgroup
    }

    pub fn render_to_own_buffer(
        self: &Self,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder,
    )
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("My First Render Pass to RTT"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment{
        view: &self.target_rtt.view,
        resolve_target: None,
        ops: wgpu::Operations {
        load: wgpu::LoadOp::Clear(wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
        }),
        store: StoreOp::Store,
        },
        })],
        depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        //render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
        render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
        //render_pass.draw(0..self.num_vertices,0..1);
    }

    pub fn render_to_screen(
        self: &Self,
        view: &wgpu::TextureView,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder
    )
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My First Render Pass to RTT"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment{
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });


        render_pass.set_pipeline(&self.render_pipeline);
        //render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
        render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
    }
}