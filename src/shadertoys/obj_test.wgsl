
struct Uniforms {
iMouse: vec4<f32>,
iResolution: vec2<f32>,
iTime:f32,
};

struct VertUniforms{
view_proj: mat4x4<f32>,
};


struct MaterialUniforms{
color : vec4<f32>
};

//modelUniforms typically change per draw-call
struct ModelUniforms {
model_matrix: mat4x4<f32>,
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
var<uniform> material_uniforms: MaterialUniforms;
@group(1) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(2)
var s_diffuse: sampler;

@group(2) @binding(0)
var<uniform> model_uniforms: ModelUniforms;



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
	out.clip_position = vert_uniforms.view_proj * model_uniforms.model_matrix * vec4<f32>(model.position, 1.0);
	out.uv = model.uv;

	return out;
}

//Fragment Shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

	var texSample : vec4<f32> = textureSample(t_diffuse,s_diffuse,in.uv);
	let depth :f32 = in.clip_position.z / in.clip_position.w;

    let dutchColors: vec3<f32> = vec3<f32>(0.5) + vec3<f32>(0.5) * cos(uniforms.iTime + vec3<f32>(in.uv.x, in.uv.x, in.uv.x) + vec3<f32>(0.0, 2.0, 4.0));
	var sun : vec3<f32> = vec3<f32>(-0.57,-0.57,0.57);

	let first_sun : f32 = pow(dot(in.normal,sun),2.0)*1.8;
	let otherSun : f32 = dot(in.normal,vec3<f32>(-0.57,0.57,0.57))*0.5;

	let fog = 1.0 - pow(((depth*0.5 +0.5)) *0.1,2.1342);

	texSample.r = first_sun * dutchColors.r + dutchColors.r * otherSun;
	texSample.g = first_sun * dutchColors.g * uniforms.iMouse.z + dutchColors.g * otherSun;
	texSample.b = first_sun * dutchColors.b + dutchColors.b * otherSun;

	texSample.r *= fog;
	texSample.g *= fog;
	texSample.b *= fog;

    let fog_plus = pow(fog,1.71212)*0.15;
	texSample.r += fog_plus;
	texSample.g += fog_plus;
	texSample.b += fog_plus;
	texSample.a = 1.0;
	return texSample;
}


