mod nocmp;

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
                        WindowEvent::CursorMoved {position,..} => {
                            state.update_mousexy(position.x,position.y);
                        }

                        WindowEvent::MouseInput{state:element_state,button,..} => {
                            //handle mouse input event...
                            state.update_mouse_event(element_state,button);
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
    rtt_bind_group:wgpu::BindGroup,
    dif_tex_1: nocmp::texture::Texture,
    dif_tex_2: nocmp::texture::Texture,
    rtt_tex: nocmp::texture::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    buffer_a: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_b: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_screen: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    toylike_uniforms: nocmp::shadertoy_buffer::ShaderToyUniforms,

}

impl State {

    pub fn update_mousexy(&mut self, mx: f64, my: f64) {
        self.uniforms.iMouse[0] = mx as f32;
        self.uniforms.iMouse[1] = my as f32;

        self.toylike_uniforms.uniforms.iMouse[0] = mx as f32;
        self.toylike_uniforms.uniforms.iMouse[1] = my as f32;
    }

    pub fn update_mouse_event(&mut self, element_state:&ElementState , button: &MouseButton) {
        match(element_state,button)  {
            (ElementState::Pressed, MouseButton::Left) => {
                self.uniforms.iMouse[2] = 1.0;
                self.toylike_uniforms.uniforms.iMouse[2] = 1.0;
            }
            (ElementState::Released, MouseButton::Left) => {
                self.uniforms.iMouse[2] = 0.0;
                self.toylike_uniforms.uniforms.iMouse[2] = 0.0;
            }
            (ElementState::Pressed, MouseButton::Right) => {
                self.uniforms.iMouse[3] = 1.0;
                self.toylike_uniforms.uniforms.iMouse[3] = 1.0;
            }
            (ElementState::Released, MouseButton::Right) => {
                self.uniforms.iMouse[3] = 0.0;
                self.toylike_uniforms.uniforms.iMouse[3] = 0.0;
            }
            _ => {}
        }
    }
    //Creating some of the wgpu types requires async code
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


        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        //let us create the device and queue
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

        let dif_tex_1= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("diffuse.png"),"testing").unwrap();
        let dif_tex_2= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("diffuse.png"),"testing imagetest").unwrap();
        let rtt_tex= nocmp::texture::Texture::create_rtt_texture(1024,1024,&device,Some("rtt_nocmp_test")).unwrap();

        let (texture_bind_group_layout,texture_bind_group) = nocmp::texture::setup_texture_stage(&device, &[&dif_tex_1], Some("Just one texture")).unwrap();

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
        let shader2 = device.create_shader_module(wgpu::include_wgsl!("shadertoys/shader_buffer_b.wgsl"));

        let toylike_uniforms = nocmp::shadertoy_buffer::ShaderToyUniforms::new(&device).unwrap();

        let buffer_a = nocmp::shadertoy_buffer::ShaderToylikeBuffer::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/shader_buffer_a.wgsl")
        ).unwrap();

        let buffer_b = nocmp::shadertoy_buffer::ShaderToylikeBuffer::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/shader_buffer_b.wgsl")
        ).unwrap();

        let buffer_screen = nocmp::shadertoy_buffer::ShaderToylikeBuffer::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/shader_buffer_screen.wgsl")
        ).unwrap();


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
                module: &shader2,
                entry_point: "vs_main",
                buffers:&[
                    Vertex::descy(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader2,
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
        let (rtt_bind_group_layout,rtt_bind_group) = nocmp::texture::setup_texture_stage(&device,&[&rtt_tex],Some("rtt texture stage")).unwrap();

        let render_pipeline_layout_rtt=
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout rtt"),
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &rtt_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let render_pipeline_rtt = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline RTT"),
            layout: Some(&render_pipeline_layout_rtt),
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
            texture_bind_group,
            texture_bind_group_layout,
            rtt_bind_group,
            dif_tex_1,
            dif_tex_2,
            rtt_tex,
            buffer_a,
            buffer_b,
            buffer_screen,
            toylike_uniforms,

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
            self.uniforms.iResolution[0] = new_size.width as f32;
            self.uniforms.iResolution[1] = new_size.height as f32;

            self.toylike_uniforms.uniforms.iResolution[0] = new_size.width as f32;
            self.toylike_uniforms.uniforms.iResolution[1] = new_size.height as f32;
            self.surface.configure(&self.device,&self.config);
        }
    }

    fn input(&mut self,event: &WindowEvent) ->bool {
        false
    }

    fn update(&mut self,delta_time: instant::Duration) {
        self.uniforms.iTime += delta_time.as_secs_f32();
        self.queue.write_buffer(&self.uniform_buffer,0,bytemuck::cast_slice(&[self.uniforms]));

        self.toylike_uniforms.uniforms.iTime += delta_time.as_secs_f32().max(f32::MIN_POSITIVE);
        self.toylike_uniforms.push_buffer_to_gfx_card(&self.queue);
    }

    fn render(&mut self) -> Result<(),wgpu::SurfaceError> {
        let output= self.surface.get_current_texture()?;
        let view_of_surface = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.buffer_a.render_to_own_buffer(self.buffer_b.get_target_rtt_bindgroup(),&self.toylike_uniforms,&mut encoder);
        self.buffer_b.render_to_own_buffer(self.buffer_a.get_target_rtt_bindgroup(),&self.toylike_uniforms,&mut encoder);
        self.buffer_screen.render_to_screen(&view_of_surface,self.buffer_a.get_target_rtt_bindgroup(),&self.toylike_uniforms,&mut encoder);

        //submit will accept anythingthatimplements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

