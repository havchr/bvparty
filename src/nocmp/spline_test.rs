/*
This is some preliminary code to
test rendering curves,
todo - we should tesselate, right now we can only render thin lines
and line width is not supported, so we should tesselate into nice lines.
 */
use anyhow::*;
use wgpu::util::DeviceExt;
use crate::nocmp;
use crate::nocmp::bindgrouperoo::BindGrouperoo;
use crate::nocmp::spline_curves::CurvePoint;
use std::fs::File;
use std::io::Read;
use wgpu::StoreOp;

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct SplineVertex {
    position: [f32;3],
    uv: [f32;2],
}

impl SplineVertex{
    fn desc()-> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SplineVertex>() as wgpu::BufferAddress,
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

const spline_resolution : u32 = 1000;

pub struct SplineTest{

    render_pipeline: wgpu::RenderPipeline,
    target_rtt : nocmp::texture::Texture,
    target_rtt_bindgroup : wgpu::BindGroup,
    uniform_groupio : BindGrouperoo,
    vertex_buffer: wgpu::Buffer,
    spline_vertices : [SplineVertex;spline_resolution as usize],
}

impl SplineTest {
    pub(crate) fn update_spline(self: &mut Self,time_seconds:f32) {
        let spline_vertices = &mut self.spline_vertices;
        Self::update_spline_directly( spline_vertices,time_seconds);
    }

    fn update_spline_directly(spline_vertices: &mut [SplineVertex; spline_resolution as usize],time : f32) {
        use nocmp::spline_curves;
        let bezP0 = CurvePoint {x:-0.75,y:time.sin(),z:0.0};
        let bezP1 = CurvePoint {x:-0.75,y:0.5,z:0.0};
        let bezP2 = CurvePoint {x:0.75,y:time.cos(),z:0.0};
        let bezP3 = CurvePoint {x:0.75,y:0.0,z:0.0};

        let bezP0_2 = bezP3.clone();
        let bezP1_2 = CurvePoint {x:-0.75,y:0.5,z:0.0};
        let bezP2_2 = CurvePoint {x:0.75,y:time.cos(),z:0.0};
        let bezP3_2 = CurvePoint {x:0.75,y:0.0,z:0.0};

        let bezzyPs = [bezP0,bezP1,bezP2,bezP3,bezP0_2,bezP1_2,bezP2_2,bezP3_2];
        // Serialize the array to a JSON string.


        let path = "art/heart_curve_2.json";

        // Open the file in read-only mode
        let file = File::open(path).expect("Failed to open file");

        // Read the contents of the file to a string
        let mut contents = String::new();
        let mut reader = std::io::BufReader::new(file);
        reader.read_to_string(&mut contents).expect("Failed to read file");
        println!("{}", &contents);
        let mut bezzyPs: Vec<CurvePoint>= serde_json::from_str(&contents).expect("Failed it");
        for element in bezzyPs.iter_mut() {
            element.x *=0.002;
            element.y *=0.002;
        }

        //let json_str = serde_json::to_string(&bezzyPs).expect("Failed to serialize");

        // Print the JSON string.
        //println!("{}", json_str);
        for i in 0..spline_resolution {
            let t: f32 = i as f32 / spline_resolution as f32;
            let bezCalc = spline_curves::do_bezzy_super_spline_t_01(&bezzyPs, t);
            /*let bezCalc = spline_curves::do_catmull_rom(&bezzyPs, t);
            let bezCalc = spline_curves::do_hermite(&bezzyPs, t);
            let bezCalc = spline_curves::do_b_spline(&bezzyPs, t);*/
            spline_vertices[i as usize].position[0] = bezCalc.x;
            spline_vertices[i as usize].position[1] = bezCalc.y;
            spline_vertices[i as usize].position[2] = bezCalc.z;

            spline_vertices[i as usize].uv[0] = 0.0;
            spline_vertices[i as usize].uv[1] = t;
        }
    }
}


impl SplineTest{

    pub fn create(
        device: &wgpu::Device,
        toylike_uniforms: &nocmp::shadertoy_buffer::ShaderToyUniforms,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        surface_config: &wgpu::SurfaceConfiguration,
        shader_descriptor: wgpu::ShaderModuleDescriptor,
        camera_uniform_buffer : &wgpu::Buffer,
    ) ->Result<Self>{


        let mut spline_vertices: [SplineVertex;spline_resolution as usize]= [
            SplineVertex{
                position: [0.0,0.0,0.0],
                uv : [0.0,0.0]
            }
            ;spline_resolution as usize];
        Self::update_spline_directly(&mut spline_vertices,0.0);
        //let shader = device.create_shader_module(wgpu::include_wgsl!("../shadertoys/shader_buffer_a.wgsl"));
        let shader = device.create_shader_module(shader_descriptor);

        let target_rtt = nocmp::texture::Texture::create_rtt_texture(1024_u32,1024_u32,&device,surface_config.format,Some("target rtt texture")).unwrap();
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
                    SplineVertex::descy(),
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
                topology: wgpu::PrimitiveTopology::LineStrip,
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
            cache: None,
        });


        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer for spline"),
                contents: bytemuck::cast_slice(&spline_vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );



        Ok(Self{
            render_pipeline,
            vertex_buffer,
            target_rtt,
            target_rtt_bindgroup,
            spline_vertices,
            uniform_groupio,
        })
    }

    pub fn get_target_rtt_bindgroup(self: &Self) -> &wgpu::BindGroup{
        &self.target_rtt_bindgroup
    }

    pub fn render_to_own_buffer(
        self: &mut Self,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &nocmp::shadertoy_buffer::ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder,
    )
    {
        self.update_spline(toylike_uniforms.uniforms.iTime);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("My First Render Pass to RTT"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment{
        view: &self.target_rtt.view,
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
        depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        //render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
        render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        //render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
        //render_pass.draw_indexed(0..self.num_indices,0,0..1);
        render_pass.draw(0..spline_resolution,0..1);
        //render_pass.draw(0..self.num_vertices,0..1);
    }

    pub fn render_to_screen(
        self: &mut Self,
        view: &wgpu::TextureView,
        textures_group: &wgpu::BindGroup,
        toylike_uniforms : &nocmp::shadertoy_buffer::ShaderToyUniforms,
        encoder: &mut wgpu::CommandEncoder,
        queue : &wgpu::Queue
    )
    {

        self.update_spline(toylike_uniforms.uniforms.iTime);
        //todo - convert from proof of concept to something more usable.
        queue.write_buffer(&self.vertex_buffer,0,bytemuck::cast_slice(&self.spline_vertices[0..spline_resolution as usize]));
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("My First Render Pass to RTT"),
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        //render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
        //render_pass.set_bind_group(0,&toylike_uniforms.uniform_bind_group,&[]);
        render_pass.set_bind_group(0,&self.uniform_groupio.bind_group,&[]);
        render_pass.set_bind_group(1,textures_group,&[]);
        render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
        render_pass.draw(0..spline_resolution,0..1);
    }
}