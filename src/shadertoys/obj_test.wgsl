
struct Uniforms {
iMouse: vec4<f32>,
iResolution: vec2<f32>,
iTime:f32,
};

struct VertUniforms {
view_proj: mat4x4<f32>,
};

// https://toji.dev/webgpu-best-practices/bind-groups.html
//some info here - group(0) should contain variables that rareley changes per mesh
// for instance, camera and such
//group 1 can be materials and such that changes sometimes, but not as often as
//group 2 , model matrix, changes per model/mesh.

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
@group(0) @binding(1)
var<uniform> vert_uniforms: VertUniforms;
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) uv: vec2<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) normal: vec3<f32>,
	@location(1) uv: vec2<f32>,
};


@vertex
fn vs_main(
	model: VertexInput,
)-> VertexOutput {
	var out: VertexOutput;
	out.normal = model.normal;
	out.clip_position = vert_uniforms.view_proj * vec4<f32>(model.position, 1.0);
	out.uv = model.uv;

	return out;
}

//Fragment Shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

	var texSample : vec4<f32> = textureSample(t_diffuse,s_diffuse,in.uv);
	texSample.r = dot(in.normal,vec3<f32>(0.0,-1.0,0.0));
	texSample.g = sin(uniforms.iTime*5.0);
	texSample.b = uniforms.iMouse[2] * dot(in.normal,vec3<f32>(0.0,-1.0,0.0));
	texSample.a = 1.0;
	return texSample;
}


