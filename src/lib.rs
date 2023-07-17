use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::util::DeviceExt;
use instant::Duration;


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
    iTime:f32,
    pad1:f32,
    pad2:f32,
    pad3:f32,
}

impl Uniforms{
    fn new()->Self {
        Uniforms{
            iMouse: [0.0,0.0,0.0,0.0],
            iTime: 0.0,
            pad1: 0.0,
            pad2: 0.0,
            pad3: 0.0,
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

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(window).await;
    let mut last_render_time = instant::Instant::now(); 


    event_loop.run(move |event, _, control_flow |{
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                input: 
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        ..
                                    },
                                    ..
                            } => *control_flow = ControlFlow::Exit,
                            WindowEvent::Resized(physical_size) => {
                                state.resize(*physical_size);
                            }
                        WindowEvent::ScaleFactorChanged {new_inner_size,..} => {
                            //new_inner_size is &&mut so we have to deref twice.
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                let now = instant::Instant::now();
                let delta_time = now - last_render_time;
                state.update(delta_time);
                last_render_time = now;
                match state.render() {
                    Ok(_) => {}
                    //reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    //System out of memory ooops.
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::RedrawEventsCleared => {
                //RedrawRequested willonly trigger once,unlesswemanually request it
                state.window().request_redraw();
           }
            _=> {}
        }
    });
}

use winit::window::Window;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    render_pipeline_rtt: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer:wgpu::Buffer,
    uniform_bind_group:wgpu::BindGroup,
    uniforms:Uniforms,
    window: Window,
    num_indices: u32,
    texture_rtt:wgpu::Texture,
    diffuse_bind_group:wgpu::BindGroup,

}

impl State {
    //Creating some ofthe wgpu types requires async code
    async fn new(window: Window) -> Self {
        let size = window.inner_size();


        //The instance is a handle to our GPU
        //Backends::all => Vulkan , Metal, DX12,Browser ,WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor{
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // #Safety
        //
        // The surface needs to live as long as the window that created
        // it , State owns the window , sol this should be safe?.. I guess.
        let surface = unsafe{ instance.create_surface(&window) }.unwrap();


        //load image
        let diffuse_bytes = include_bytes!("diffuse.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();


        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        //let us create the deviceand queue
        let(device,queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: if cfg!(target_arch = "wasm32"){
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None,
        ).await.unwrap();


        let diffuse_texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: diffuse_texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("Diffuse_texture"),
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            diffuse_texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u : wgpu::AddressMode::ClampToEdge,
            address_mode_v : wgpu::AddressMode::ClampToEdge,
            address_mode_w : wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });



        let texture_bind_group_layout = 
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable:true
                            },
                        },
                        count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );



        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Rgba8Unorm)
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device,&config);

        //let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaper_shader.wgsl"));



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
                    format: config.format,
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


        let texture_descriptor_rtt =wgpu::TextureDescriptor {
            label:Some("Rtt Texture"),
                size: wgpu::Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
                },
                mip_level_count:1,
                sample_count:1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT |wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats:&[],
        };

        let texture_rtt = device.create_texture(&texture_descriptor_rtt);

        let render_pipeline_rtt = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline RTT"),
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
                    format: config.format,
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
                    
        Self{
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            render_pipeline_rtt,
            vertex_buffer,
            index_buffer,
            uniform_bind_group,
            uniform_buffer,
            num_indices,
            uniforms,
            window,
            texture_rtt,
            diffuse_bind_group,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device,&self.config);
        }
    }

    fn input(&mut self,event: &WindowEvent) ->bool {
        false
    }

    fn update(&mut self,delta_time: instant::Duration) {
        self.uniforms.iTime += delta_time.as_secs_f32();
        self.uniforms.iMouse[0] = 0.5;
        self.uniforms.iMouse[1] = 0.5;
        self.queue.write_buffer(&self.uniform_buffer,0,bytemuck::cast_slice(&[self.uniforms]));
    }

    fn render(&mut self) -> Result<(),wgpu::SurfaceError> {
        let output= self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_view_rtt = self.texture_rtt.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        //I am trying to render to texture but it says my format is just too wrong
        /*
        {
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
    

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
            render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices,0,0..1);
            //render_pass.draw(0..self.num_vertices,0..1);
        }
        */

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("My First Render Pass"),
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
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
    

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0,&self.uniform_bind_group,&[]);
            render_pass.set_bind_group(1,&self.diffuse_bind_group,&[]);
            render_pass.set_vertex_buffer(0,self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..),wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices,0,0..1);
            //render_pass.draw(0..self.num_vertices,0..1);
        }


        //submit will accept anythingthatimplements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

