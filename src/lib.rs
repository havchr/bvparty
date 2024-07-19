pub mod nocmp;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use cgmath::num_traits::real::Real;
use egui::FontDefinitions;
use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};
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



use winit::window::{Fullscreen, Window};

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
    fs_quad: nocmp::shadertoy_buffer::ShaderToylikeBuffer,
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
    meshes : HashMap<String, HashMap< String, nocmp::obj_parser::Mesh> >,
    textures: HashMap<String, wgpu::BindGroup>

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
                memory_hints: Default::default(),
            },
            None,
        ).await.unwrap();

        let dif_tex_1= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("../art/logo.png"),"testing").unwrap();
        let dif_tex_2= nocmp::texture::Texture::from_bytes(&device,&queue,include_bytes!("diffuse.png"),"testing imagetest").unwrap();

        let (texture_bind_group_layout,texture_bind_group) = nocmp::texture::setup_texture_stage(&device, &[&dif_tex_1], Some("Just one texture")).unwrap();
        let mut textures = HashMap::new();

        //Loading up all textures


        let mut message_frames = 11;
        while message_frames >= 0{

            let path : String = format!("art/message/frame_{message_frames}.png");
            let (_,txbg) = nocmp::texture::setup_texture_stage(&device, &[&nocmp::texture::Texture::from_path(&device,&queue,
                                                                                                               path.as_str(),"fingers crossed").unwrap()
            ], Some("Just one texture")).unwrap();
            textures.insert(format!("frame_{message_frames}").parse().unwrap(),txbg);
            message_frames-=1;
        }

        let mut greet_frames= 14;
        while greet_frames >= 0{

            let path : String = format!("art/greets/greets_{greet_frames}.png");
            let (_,txbg) = nocmp::texture::setup_texture_stage(&device, &[&nocmp::texture::Texture::from_path(&device,&queue,
                                                                                                              path.as_str(),"fingers crossed").unwrap()
            ], Some("Just one texture")).unwrap();
            textures.insert(format!("greet_{greet_frames}").parse().unwrap(),txbg);
            greet_frames-=1;
        }

        let mut refreng_frames= 3;
        while refreng_frames >= 0{

            let path : String = format!("art/refrence/refreng_{refreng_frames}.png");
            let (_,txbg) = nocmp::texture::setup_texture_stage(&device, &[&nocmp::texture::Texture::from_path(&device,&queue,
                                                                                                              path.as_str(),"fingers crossed").unwrap()
            ], Some("Just one texture")).unwrap();
            textures.insert(format!("refreng_{refreng_frames}").parse().unwrap(),txbg);
            refreng_frames-=1;
        }

        let mut cred_frames= 6;
        while cred_frames >= 0{

            let path : String = format!("art/creds/creds_{cred_frames}.png");
            let (_,txbg) = nocmp::texture::setup_texture_stage(&device, &[&nocmp::texture::Texture::from_path(&device,&queue,
                                                                                                              path.as_str(),"fingers crossed").unwrap()
            ], Some("Just one texture")).unwrap();
            textures.insert(format!("creds_{cred_frames}").parse().unwrap(),txbg);
            cred_frames-=1;
        }

        let (_,txbg) = nocmp::texture::setup_texture_stage(&device, &[&nocmp::texture::Texture::from_bytes(&device,&queue,
                                                                                                           include_bytes!("../art/logo.png"),"fingers crossed").unwrap()
        ], Some("Just one texture")).unwrap();
        textures.insert("logo".parse().unwrap(),txbg);

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

        let fs_quad = nocmp::shadertoy_buffer::ShaderToylikeBuffer::create(
            &device,
            &toylike_uniforms,
            &texture_bind_group_layout,
            &config,
            wgpu::include_wgsl!("shadertoys/fs_quad.wgsl")
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
            &(meshes.get_key_value("world").unwrap().1.get_key_value("World").unwrap().1)
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
            meshes,
            fs_quad,
            textures
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


        //self.camera_controller.update_camera(&mut self.camera);
        //self.camera_uniform.update_view_proj(&self.camera);

        let x_sin = f32::sin(self.toylike_uniforms.uniforms.iTime*0.1) * 5.0;
        //self.obj_mesh_test.model_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3 { x: x_sin, y: f32::sin(self.toylike_uniforms.uniforms.iTime*3.0)*0.25, z: 0.0 });
        self.obj_mesh_test.push_modelview(&self.queue);


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



        let time = self.toylike_uniforms.uniforms.iTime;
        let time_ms = (time * 1000.0) as i32;

        if(time > 0.0 && time < 7.0){


            let beat_decay = 350;
            let beat_decay_longer = 650;
            let is_beat = time_ms % 469 <= beat_decay;
            let is_beat_half = time_ms % (469*2) <= beat_decay;
            if(is_beat_half){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("logo").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
        }

        let number_beats = time_ms / 469;
        let number_beats_2x = time_ms / (469*2);
        let mut camera_change =  (number_beats / 32)%4;

        //camera_change = 3;
        if(camera_change == 0){

            self.camera.eye.x =  f32::sin(self.toylike_uniforms.uniforms.iTime*0.1 + 0.7) * 0.05;
            self.camera.eye.y = f32::sin(self.toylike_uniforms.uniforms.iTime*0.1) * 0.2;
            self.camera.eye.z =  f32::sin(self.toylike_uniforms.uniforms.iTime*0.1) * 0.05 + 2.0;

            self.camera_uniform.update_view_proj(&self.camera);

        }
        if(camera_change == 1){

            self.camera.eye.x = f32::sin(self.toylike_uniforms.uniforms.iTime*1.1) * 0.3;
            self.camera.eye.y =  f32::cos(self.toylike_uniforms.uniforms.iTime*2.1) * 0.2 + 0.3;
            self.camera.eye.z =  f32::sin(self.toylike_uniforms.uniforms.iTime*0.7) * 1.0 - 1.5;
            self.camera_uniform.update_view_proj(&self.camera);

        }
        if(camera_change == 2){

            self.camera.eye.x = f32::sin(self.toylike_uniforms.uniforms.iTime*5.1) * 0.3;
            self.camera.eye.y =  f32::cos(self.toylike_uniforms.uniforms.iTime*2.1) * 0.2 + 0.3;
            self.camera.eye.z =  f32::sin(self.toylike_uniforms.uniforms.iTime*5.7 + 0.3) * 0.5 + 2.5;
            self.camera_uniform.update_view_proj(&self.camera);

        }
        if(camera_change == 3){

            self.camera.eye.x = f32::sin(self.toylike_uniforms.uniforms.iTime*1.1) * 10.3;
            self.camera.eye.y =  f32::cos(self.toylike_uniforms.uniforms.iTime*2.1) * 0.2 + 0.3;
            self.camera.eye.z =  f32::cos(self.toylike_uniforms.uniforms.iTime*0.7) * 7.0 - 1.5;
            self.camera_uniform.update_view_proj(&self.camera);

        }


        if(number_beats >= 16*4 && number_beats < 20*4){

            let offset = 16*4;
            let greet = (number_beats-offset) ;
            if(greet < 15){
                let greet_frame = format!("greet_{greet}");
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&greet_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
            }
        }

        if(number_beats >= 30 *4 && number_beats < 38*4){

            let refreng= number_beats_2x % 4;
            let refrenge_frame= format!("refreng_{refreng}");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&refrenge_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }

        if(number_beats >= 40 *4 && number_beats < 41*4){

            let creds_frame= format!("creds_0");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 41 *4 && number_beats < 42*4){

            let creds_frame= format!("creds_1");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 42 *4 && number_beats < 43*4){

            let creds_frame= format!("creds_2");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 43 *4 && number_beats < 44*4){

            let creds_frame= format!("creds_3");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 44 *4 && number_beats < 45*4){

            let creds_frame= format!("creds_4");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 45 *4 && number_beats < 46*4){

            let creds_frame= format!("creds_5");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }
        if(number_beats >= 46 *4 ){

            let creds_frame= format!("creds_6");
            self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get(&creds_frame).unwrap(),&self.toylike_uniforms,&mut encoder);
        }


        if(time > 7.43 && time < 15.0){

            if(time > 7.582 && time < 7.869){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_0").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 7.869 && time < 8.777){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_1").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 8.777 && time < 9.339){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_2").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 9.339 && time < 10.293){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_3").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 10.293 && time < 10.670){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_4").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 10.670 && time < 12.161){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_5").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 12.161 && time < 12.763){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_6").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 12.763 && time < 13.349){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_7").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 13.349 && time < 13.634){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_8").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 13.634 && time < 13.832){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_9").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 13.832 && time < 14.150){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_10").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
            if(time > 14.150 && time < 114.150){
                self.fs_quad.render_to_screen_without_clear(&view_of_surface,&self.textures.get("frame_11").unwrap(),&self.toylike_uniforms,&mut encoder);
            }
        }




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
    let window_size = Size::from(PhysicalSize::new(1920,1080));
    let window_size = Size::from(PhysicalSize::new(1920/2,1080/2));
    let window = WindowBuilder::new()
        .with_min_inner_size(window_size)
       // .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .build(&event_loop).unwrap();

    window.set_cursor_visible(false);

    let mut inited : bool = false;

    let mut start_time = instant::Instant::now();
    let mut last_render_time = instant::Instant::now();



    let mut state = State::new(&window).await;


    //todo - make the egui work
    /*

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: window_size.to_physical(window.scale_factor()).width as u32,
        physical_height: window_size.to_physical(window.scale_factor()).height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&state.device, state.config.format, 1);
    // Display the demo application that ships with egui.
    let mut demo_app = egui_demo_lib::DemoWindows::default();

   //_________ end of todo
     */

    start_time = instant::Instant::now();
    last_render_time = instant::Instant::now();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open("art/nsts.ogg").unwrap());
    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();
    // Play the sound directly on the device

    stream_handle.play_raw(source.convert_samples());
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
