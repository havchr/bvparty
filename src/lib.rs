pub mod nocmp;

use std::mem::size_of;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::util::DeviceExt;
use instant::Duration;
use winit::dpi::{PhysicalSize, Size};


pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window_size = Size::from(PhysicalSize::new(1024,1024));
    let window = WindowBuilder::new().with_inner_size(window_size).build(&event_loop).unwrap();

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
    window: Window,
    dif_tex_1: nocmp::texture::Texture,
    dif_tex_2: nocmp::texture::Texture,
    rtt_tex: nocmp::texture::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    buffer_a: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_b: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_screen: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    spline_test: nocmp::spline_test::SplineTest,
    toylike_uniforms: nocmp::shadertoy_buffer::ShaderToyUniforms,
    camera: nocmp::camera::Camera,
    camera_controller: nocmp::camera::CameraController,
    camera_uniform : nocmp::camera::CameraUniform,
    camera_uniform_buffer : wgpu::Buffer,

}

impl State {

    pub fn update_mousexy(&mut self, mx: f64, my: f64) {
        self.toylike_uniforms.uniforms.iMouse[0] = mx as f32;
        self.toylike_uniforms.uniforms.iMouse[1] = my as f32;
    }

    pub fn update_mouse_event(&mut self, element_state:&ElementState , button: &MouseButton) {
        match(element_state,button)  {
            (ElementState::Pressed, MouseButton::Left) => {
                self.toylike_uniforms.uniforms.iMouse[2] = 1.0;
            }
            (ElementState::Released, MouseButton::Left) => {
                self.toylike_uniforms.uniforms.iMouse[2] = 0.0;
            }
            (ElementState::Pressed, MouseButton::Right) => {
                self.toylike_uniforms.uniforms.iMouse[3] = 1.0;
            }
            (ElementState::Released, MouseButton::Right) => {
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



        let camera = nocmp::camera::Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fov_y: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };
        let camera_controller = nocmp::camera::CameraController::new(5.0);
        let mut camera_uniform = nocmp::camera::CameraUniform::new();
        let camera_uniform_buffer = camera_uniform.create_buffer(&device).unwrap();

        let spline_test = nocmp::spline_test::SplineTest::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/test.wgsl"),
            &camera_uniform_buffer,
        ).unwrap();


        Self{
            surface,
            device,
            queue,
            config,
            size,
            window,
            texture_bind_group,
            texture_bind_group_layout,
            dif_tex_1,
            dif_tex_2,
            rtt_tex,
            buffer_a,
            buffer_b,
            buffer_screen,
            toylike_uniforms,
            spline_test,
            camera,
            camera_controller,
            camera_uniform,
            camera_uniform_buffer

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

            self.toylike_uniforms.uniforms.iResolution[0] = new_size.width as f32;
            self.toylike_uniforms.uniforms.iResolution[1] = new_size.height as f32;
            self.surface.configure(&self.device,&self.config);
        }
    }

    fn input(&mut self,event: &WindowEvent) ->bool {
        false
    }

    fn update(&mut self,delta_time: instant::Duration) {
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
        //self.spline_test.render_to_screen(&view_of_surface,self.buffer_a.get_target_rtt_bindgroup(),&self.toylike_uniforms,&mut encoder,&self.queue);

        //submit will accept anythingthatimplements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

