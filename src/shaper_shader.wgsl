//Vertex Shader


struct Uniforms {
iMouse: vec4<f32>,
iTime:f32,
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

fn smoothedCircle(pos : vec2<f32> ,size:f32,smoothFactor:f32,uv:vec2<f32>) -> f32{

    let distanceToCircleOrigo:f32 = distance(pos,uv);
    
    //why do we need 1.0
    //because smoothStep returns if 1 if x is bigger than a and b in a,b,x
    //when we define a circle , distances smaller than size should be 1, 
    //so we need to invert it.
    
    //rule to remember , smoothstep a b x , x larger than ab then 1..
    return (1.0-smoothstep(size,size+smoothFactor,distanceToCircleOrigo));

}

//Fragment Shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let smoo = smoothedCircle(vec2<f32>(uniforms.iMouse.x ,uniforms.iMouse.y + sin(uniforms.iTime*0.3)*0.66),0.2,0.1,in.uv) + 0.25;
	let texSample : vec4<f32> = textureSample(t_diffuse,s_diffuse,in.uv);
	return vec4<f32>(in.color*smoo,1.0) + texSample;
}


