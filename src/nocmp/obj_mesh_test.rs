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
use wgpu::StoreOp;
use crate::nocmp::obj_parser::{Face, Mesh};
use crate::nocmp::texture;

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    position: [f32;3],
    normal: [f32;3],
    uv: [f32;2],
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
    uniform_groupio : BindGrouperoo,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl ObjMeshTest{

    pub fn create(
        device: &wgpu::Device,
        toylike_uniforms: &nocmp::shadertoy_buffer::ShaderToyUniforms,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_descriptor: wgpu::ShaderModuleDescriptor,
        camera_uniform_buffer : &wgpu::Buffer,
    ) ->Result<Self>{


        let shader = device.create_shader_module(shader_descriptor);

        let target_rtt = nocmp::texture::Texture::create_rtt_texture(1024_u32,1024_u32,&device,Some("target rtt texture")).unwrap();
        let (bind_group_layout,target_rtt_bindgroup) = nocmp::texture::setup_texture_stage(
            &device,
            &[&target_rtt],
            Some("target_rtt")
        ).unwrap();

        //todo up next , make camera matrix work, update camera and move things/camera on screen.

        //todo make proper good structure for this bindgrouperoo thing..
        //right now, it is a mess we are trying to understand.
        //but here at least, we are creating
        // (binding 0) uniform buffers with app-uniforms like iTime etc accessible from both vertex and fragment
        // (binding 1) camera uniforms accessible from the Vertex shader
        let uniform_groupio= nocmp::bindgrouperoo::BindGrouperoo::new(&device,
        &[wgpu::ShaderStages::VERTEX_FRAGMENT,
                 wgpu::ShaderStages::VERTEX],
       &[&toylike_uniforms.uniform_buffer,
                      &camera_uniform_buffer],Some("bindGroup uniform thing"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    //&uniform_bind_group_layout,
                    &uniform_groupio.bind_group_layout,
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





        let mesh = Mesh::parse_from_file("art/big_test.obj").unwrap();

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
            uniform_groupio,
            index_buffer,
            vertex_buffer,
            num_indices,
        })
    }

    pub fn get_target_rtt_bindgroup(self: &Self) -> &wgpu::BindGroup{
        &self.target_rtt_bindgroup
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
                        r: 0.1,
                        g: 0.8,
                        b: 0.3,
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
        render_pass.set_bind_group(0,&self.uniform_groupio.bind_group,&[]);
        //render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices,0,0..1);
    }
}