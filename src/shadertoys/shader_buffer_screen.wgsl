//Vertex Shader


struct Uniforms {
iMouse: vec4<f32>,
iResolution: vec2<f32>,
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

     var uv:vec2f = in.uv;
    
    //this modification is  me trying to make the social media scroller effect
     uv.y*=0.25;
     uv.x*=1.5;
     uv.y+=pow(tan(uniforms.iTime*0.6)*0.5,3.0);
    
    var texCol :vec3f = textureSample(t_diffuse,s_diffuse,uv).rgb;
    texCol = clamp(texCol,vec3f(0.0),vec3f(1.0));
    return vec4(texCol,1.0);

}


