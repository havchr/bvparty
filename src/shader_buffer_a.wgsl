
struct Uniforms {
iMouse: vec4<f32>,
iTime:f32,
iResolution: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) color: vec3<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) color: vec3<f32>,
	@location(1) uv: vec2<f32>,
};


@vertex
fn vs_main(
	model: VertexInput,
)-> VertexOutput {
	var out: VertexOutput;
	out.color = model.color;
	out.clip_position = vec4<f32>(model.position, 1.0);
	out.uv = vec2<f32>(model.position.x*0.5, model.position.y*0.5*-1.0)+ vec2<f32>(0.5,0.5);

	return out;
}

//Fragment Shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

     //reading with distortion
     let dist_factor = textureSample(t_diffuse,s_diffuse,in.uv).a;
     let uv:vec2f = in.uv + vec2f(sin(in.uv.x*uniforms.iTime*0.003 + in.uv.y*5.3 + uniforms.iTime*5.4)*(0.01 + dist_factor),
     sin(in.uv.y*uniforms.iTime*0.003 + in.uv.y*8.3 + uniforms.iTime*6.4)*(0.01 + dist_factor));
	let texSample : vec4<f32> = textureSample(t_diffuse,s_diffuse,uv);
	return texSample;
}


