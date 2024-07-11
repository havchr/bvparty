pub mod nocmp;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use wgpu::util::DeviceExt;
use instant::Duration;
use rodio::{Decoder, OutputStream, Source};
use wgpu::Gles3MinorVersion;
use winit::dpi::{PhysicalSize, Size};
use winit::keyboard::{KeyCode, PhysicalKey};



use winit::window::Window;

struct State<'demo_lifetime> {
    surface: wgpu::Surface<'demo_lifetime>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    //The window must be declared after the surface so
    //it gets dropped after it(the surface), because
    //the surface contains unsafe references to the windows resources
    window: &'demo_lifetime Window,
    dif_tex_1: nocmp::texture::Texture,
    dif_tex_2: nocmp::texture::Texture,
    rtt_tex: nocmp::texture::Texture,
    depth_texture : nocmp::texture::Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    buffer_a: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_b: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    buffer_screen: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
    obj_mesh_test: nocmp::obj_mesh_test::ObjMeshTest,
    spline_test: nocmp::spline_test::SplineTest,
    toylike_uniforms: nocmp::shadertoy_buffer::ShaderToyUniforms,
    camera: nocmp::camera::Camera,
    camera_controller: nocmp::camera::CameraController,
    camera_uniform : nocmp::camera::CameraUniform,
    camera_uniform_buffer : wgpu::Buffer,
    dancer : Vec<nocmp::obj_mesh_test::ObjMeshTest>,
    dancer_frame : usize,
    meshes : HashMap<String, HashMap< String, nocmp::obj_parser::Mesh> >

}

impl<'demo_lifetime> State<'demo_lifetime> {

    pub fn update_mousexy(&mut self, mx: f64, my: f64) {
        self.toylike_uniforms.uniforms.iMouse[0] = mx as f32;
        self.toylike_uniforms.uniforms.iMouse[1] = my as f32;
    }

    pub fn draw_sync_test(&mut self,song_time: &instant::Instant) {

        let elapsed = song_time.elapsed();
        let elapsed_millis = elapsed.as_millis();
        let is_beat = elapsed_millis % 469 <= 55;
        let is_beat_x2 = elapsed_millis % (469/4) <= 60;

        if is_beat_x2{
            self.dancer_frame += 1;
            if self.dancer_frame >= self.dancer.len() {
                self.dancer_frame = 0;
            }
        }
        if  is_beat {

            self.toylike_uniforms.uniforms.iMouse[0] = self.window.inner_size().width as f32 * 0.5_f32;
            self.toylike_uniforms.uniforms.iMouse[1] = self.window.inner_size().height as f32 * 0.5_f32;
            self.toylike_uniforms.uniforms.iMouse[2] = 1.0;
        }
        else {
            self.toylike_uniforms.uniforms.iMouse[2] = 0.0;
        }
        
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
    async fn new(window: &'demo_lifetime Window) -> State<'demo_lifetime> {
        let size = window.inner_size();


        //The instance is a handle to our GPU
        //Backends::all => Vulkan , Metal, DX12,Browser ,WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor{
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::debugging(),
            gles_minor_version: Gles3MinorVersion::Automatic
        });

        let surface =  instance.create_surface(window).unwrap();


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
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32"){
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None,
        ).await.unwrap();

        let dif_tex_1= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("../art/scroll_test.png"),"testing").unwrap();
        let dif_tex_2= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("diffuse.png"),"testing imagetest").unwrap();

        let (texture_bind_group_layout,texture_bind_group) = nocmp::texture::setup_texture_stage(&device, &[&dif_tex_1], Some("Just one texture")).unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Rgba8Unorm)
            .unwrap_or(surface_caps.formats[0]);

        let rtt_tex= nocmp::texture::Texture::create_rtt_texture(1024,1024,&device,surface_format,Some("rtt_nocmp_test")).unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency:2,
            view_formats: vec![],
        };
        surface.configure(&device,&config);

        let depth_texture = nocmp::texture::Texture::create_depth_texture(&device,&config,"nocmp depth texture");

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
            target: (0.0, 1.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fov_y: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };
        let camera_controller = nocmp::camera::CameraController::new(0.05);
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

        let mut meshes: HashMap<String, HashMap< String, nocmp::obj_parser::Mesh> > = HashMap::new();
        meshes.insert("world".parse().unwrap(), nocmp::obj_parser::Mesh::parse_from_file("art/world.obj").unwrap());
        meshes.insert("alphabet".parse().unwrap(), nocmp::obj_parser::Mesh::parse_from_file("art/alphabet/alphabet_.obj").unwrap());

        let obj_mesh_test = nocmp::obj_mesh_test::ObjMeshTest::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/obj_test.wgsl"),
            &camera_uniform_buffer,
            &queue,
            &(meshes.get_key_value("alphabet").unwrap().1.get_key_value("z").unwrap().1)
        ).unwrap();



        let mut dancer : Vec<nocmp::obj_mesh_test::ObjMeshTest> = Vec::new();

        let mut dancer_frames = 334;
        while(dancer_frames < 371){
            let path : String = format!("art/dance_frames/dance_frames0{dancer_frames}.obj");

            meshes.insert(path.parse().unwrap(), nocmp::obj_parser::Mesh::parse_from_file(path.as_str()).unwrap());
            let dance_frame_mesh= nocmp::obj_mesh_test::ObjMeshTest::create(
                &device,
                &toylike_uniforms,
                &texture_bind_group_layout,
                &config,
                wgpu::include_wgsl!("shadertoys/obj_test.wgsl"),
                &camera_uniform_buffer,
                &queue,
                &(meshes.get_key_value(&path).unwrap().1.get_key_value("Beta_Surface").unwrap().1)
            ).unwrap();
            dancer.push(dance_frame_mesh);
            dancer_frames+=1;
        }

        let mut dancer_frames = 157;
        while(dancer_frames <= 207){
            let path : String = format!("art/2dance_frames/2dancer0{dancer_frames}.obj");

            meshes.insert(path.parse().unwrap(), nocmp::obj_parser::Mesh::parse_from_file(path.as_str()).unwrap());
            let dance_frame_mesh= nocmp::obj_mesh_test::ObjMeshTest::create(
                &device,
                &toylike_uniforms,
                &texture_bind_group_layout,
                &config,
                wgpu::include_wgsl!("shadertoys/obj_test.wgsl"),
                &camera_uniform_buffer,
                &queue,
                &(meshes.get_key_value(&path).unwrap().1.get_key_value("Beta_Surface").unwrap().1)
            ).unwrap();
            dancer.push(dance_frame_mesh);
            dancer_frames+=1;
        }

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
            depth_texture,
            buffer_a,
            buffer_b,
            buffer_screen,
            toylike_uniforms,
            spline_test,
            obj_mesh_test,
            camera,
            camera_controller,
            camera_uniform,
            camera_uniform_buffer,
            dancer,
            dancer_frame : 0,
            meshes
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

            self.depth_texture = nocmp::texture::Texture::create_depth_texture(&self.device,&self.config,"depth texture");

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


        self.camera_controller.update_camera(&mut self.camera);

        self.camera_uniform.update_view_proj(&self.camera);


        self.queue.write_buffer(&self.camera_uniform_buffer,0,bytemuck::cast_slice(&[self.camera_uniform]));
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

        
        //This renders to screen with texture_bind_group , which is our POOC scroller texture which is y = 8k, and thus very squashed without a shader with texture coordinate hacks
        //self.buffer_screen.render_to_screen(&view_of_surface,&self.texture_bind_group,&self.toylike_uniforms,&mut encoder);

        self.obj_mesh_test.render_to_screen(&view_of_surface, &self.depth_texture.view, &self.texture_bind_group, &self.toylike_uniforms, &mut encoder);
        self.dancer.get_mut(self.dancer_frame).unwrap().render_to_screen_no_clear(&view_of_surface, &self.depth_texture.view, &self.texture_bind_group, &self.toylike_uniforms, &mut encoder);

        //self.spline_test.render_to_screen(&view_of_surface,self.buffer_a.get_target_rtt_bindgroup(),&self.toylike_uniforms,&mut encoder,&self.queue);

        //submit will accept anythingthatimplements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window_size = Size::from(PhysicalSize::new(1024,1024));
    let window = WindowBuilder::new().build(&event_loop).unwrap();



    //Try playing music..
    // Get an output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open("art/track.wav").unwrap());
    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();
    // Play the sound directly on the device

    //stream_handle.play_raw(source.convert_samples());

    let start_time = instant::Instant::now();
    let mut last_render_time = instant::Instant::now();

    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow|
        {

            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                }  if window_id == state.window.id() => {
                    state.camera_controller.process_events(&event);
                    state.input(&event);
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            log::info!("physical_size: {physical_size:?}") ;
                            surface_configured = true;
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            state.window().request_redraw();
                            if !surface_configured{
                                return;
                            }

                            let now = instant::Instant::now();
                            let delta_time = now - last_render_time;


                            state.draw_sync_test(&start_time);
                            state.update(delta_time);

                            last_render_time = now;
                            match state.render() {
                                Ok(_) => {}
                                //reconfigure the surface if it's lost or outdated
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    state.resize(state.size)
                                }
                                //System out of memory ooops.
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    log::error!("OutOfMemory");
                                    control_flow.exit();
                                },
                                Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                            }
                        }
                        _ => {}
                    }
                },

                _ => {}
            }
        }).unwrap();

}
