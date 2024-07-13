/*
This is some preliminary code to
test rendering meshes,
 */

use anyhow::*;
use wgpu::util::DeviceExt;
use crate::nocmp;
use crate::nocmp::bindgrouperoo::BindGrouperoo;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::ops::Deref;
use cgmath::SquareMatrix;
use wgpu::{BindGroupLayoutDescriptor, Buffer, Queue, StoreOp};
use crate::nocmp::obj_parser::{Face, Mesh};
use crate::nocmp::texture;

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    position: [f32;3],
    normal: [f32;3],
    uv: [f32;2],
}

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniforms{
    color: [f32;4],
}

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform{
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub model_matrix: [[f32; 4]; 4],
}

impl MeshVertex{
    fn desc()-> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
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
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32;3]>()*2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
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

const mesh_resolution: u32 = 1000;

pub struct ObjMeshTest{

    render_pipeline: wgpu::RenderPipeline,
    target_rtt : nocmp::texture::Texture,
    target_rtt_bindgroup : wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    material_uniforms : MaterialUniforms,
    material_uniform_buffer: wgpu::Buffer,
    model_uniform_buffer: wgpu::Buffer,
    bind_group_0 : wgpu::BindGroup,
    bind_group_1 : wgpu::BindGroup,
    bind_group_2 : wgpu::BindGroup,
    pub model_matrix : cgmath::Matrix4<f32>
}

impl ObjMeshTest{

    pub fn create(
        device: &wgpu::Device,
        toylike_uniforms: &nocmp::shadertoy_buffer::ShaderToyUniforms,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_descriptor: wgpu::ShaderModuleDescriptor,
        camera_uniform_buffer : &wgpu::Buffer,
        queue : &wgpu::Queue,
        mesh : &Mesh,
    ) ->Result<Self>{


        let shader = device.create_shader_module(shader_descriptor);

        let dif_tex_1= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("../../art/scroll_test.png"),"testing").unwrap();
       // let dif_tex_2= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("diffuse.png"),"testing imagetest").unwrap();

        let target_rtt = nocmp::texture::Texture::create_rtt_texture(1024_u32,1024_u32,&device,surface_config.format,Some("target rtt texture")).unwrap();
        let (bind_group_layout,target_rtt_bindgroup) = nocmp::texture::setup_texture_stage(
            &device,
            &[&target_rtt],
            Some("target_rtt")
        ).unwrap();

        //todo up next , make camera matrix work, update camera and move things/camera on screen.


        let material_uniforms = MaterialUniforms{ color: [1.0,1.0,1.0,1.0] };
        let model_matrix : cgmath::Matrix4<f32> = cgmath::Matrix4::from_scale(1.0);
        let model_uniforms = ModelUniform{model_matrix : model_matrix.into() };

        //material_uniforms : MaterialUniforms,
        //material_uniform_bindgroup: wgpu::BindGroup,
        //material_uniform_buffer: wgpu::Buffer,

        let material_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("material uniform buffer"),
                contents: bytemuck::cast_slice(&[material_uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let model_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("model uniform buffer"),
                contents: bytemuck::cast_slice(&[model_uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        //todo - a good structure for bind groups?
        //or for now - we just create
        //code per material we wish to setup .. I dunno.
        //it is a lot of code for setting things up.
        //serializing parts of it into some sort of material system would
        //probably make sense at some point.

        let bind_group_layout_0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        });


        let bind_group_layout_1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },

                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true
                        },
                    },
                    count: None,
                }
                ,
                wgpu::BindGroupLayoutEntry{
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ]
        });

        let bind_group_layout_2 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ]
        });


        let bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("bind_group_0"),
            layout: &(bind_group_layout_0),
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: toylike_uniforms.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }
            ],
        });

        let bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("bind_group_1"),
            layout: &(bind_group_layout_1),
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: material_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dif_tex_1.view),
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&dif_tex_1.sampler),
                },

            ],
        });

        let bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("bind_group_2"),
            layout: &(bind_group_layout_2),
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: model_uniform_buffer.as_entire_binding(),
                }
            ],
        });


        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layout_0,
                    &bind_group_layout_1,
                    &bind_group_layout_2
                    //&uniform_bind_group_layout,
                    //&uniform_groupio.bind_group_layout,
                    //&texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline for obj"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers:&[
                    MeshVertex::desc(),
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
            depth_stencil:Some(
                wgpu::DepthStencilState{
                    format: nocmp::texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare : wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }
            ),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });


        /*
        let meshes = match Mesh::parse_from_file(mesh_file){
            Result::Ok(meshes) => {
            meshes
            }
            Err(e) => {
                println!("failed to load mesh file {mesh_file}");
                //return anyhow::Result::Err();
                return Err(anyhow!("Failed to load mesh file: {}",e))
            }
        };

        let mut meshes : Vec<Mesh> = meshes.into_values().collect();
        let mesh :&Mesh = &meshes[0];

         */

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("obj vertex buffer"),
                contents: bytemuck::cast_slice(&mesh.real_verts.as_slice()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("obj index Buffer"),
                contents: bytemuck::cast_slice(&mesh.faces.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let face_size =  mesh.feces[0].face_indices.len();
        let num_indices= (mesh.feces.len() * face_size)  as u32;
        let num_indices= mesh.faces.len() as u32;


        Ok(Self{
            render_pipeline,
            target_rtt,
            target_rtt_bindgroup,
            index_buffer,
            vertex_buffer,
            num_indices,
            material_uniforms,
            material_uniform_buffer,
            model_uniform_buffer,
            bind_group_0,
            bind_group_1,
            bind_group_2,
            model_matrix
        })
    }

    pub fn get_target_rtt_bindgroup(self: &Self) -> &wgpu::BindGroup{
        &self.target_rtt_bindgroup
    }

    pub fn render_to_screen_no_clear(
        self: &mut Self,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &nocmp::shadertoy_buffer::ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder
    )
    {

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My First Render Pass to the screen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment{
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some
                (wgpu::RenderPassDepthStencilAttachment
                {
                    view: &depth_view,
                    depth_ops: Some(
                        wgpu::Operations
                        {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }
                    ),
                    stencil_ops:None
                }
                ),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);

        //this one has camera stuff in it.
        render_pass.set_bind_group(0,&self.bind_group_0,&[]);
        render_pass.set_bind_group(1,&self.bind_group_1,&[]);
        render_pass.set_bind_group(2,&self.bind_group_2,&[]);
        //render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        //render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
    }

    pub fn push_modelview(self: &mut Self, queue: &Queue){

        let model_uniforms = ModelUniform{model_matrix : self.model_matrix.into() };
        queue.write_buffer(&self.model_uniform_buffer,0,bytemuck::cast_slice(&[model_uniforms]));
    }

    pub fn render_to_screen(
        self: &mut Self,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &nocmp::shadertoy_buffer::ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder
    )
    {

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My First Render Pass to the screen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment{
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some
                (wgpu::RenderPassDepthStencilAttachment
                {
                    view: &depth_view,
                    depth_ops: Some(
                        wgpu::Operations
                        {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }
                    ),
                    stencil_ops:None
                }
                ),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);

        //this one has camera stuff in it.
        render_pass.set_bind_group(0,&self.bind_group_0,&[]);
        render_pass.set_bind_group(1,&self.bind_group_1,&[]);
        render_pass.set_bind_group(2,&self.bind_group_2,&[]);
        //render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        //render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
    }
}