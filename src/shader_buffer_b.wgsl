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

    let colorSpewingBallRadius = 0.1;
    let colorSpewingBallRadiusOuterEdge = 0.2;
    let timeTick = sin(uniforms.iTime);


    //true fucking mystery!!
    let mouse_x_in_01 = uniforms.iMouse.x/uniforms.iResolution.x;
    let mouse_y_in_01 = uniforms.iMouse.y/uniforms.iResolution.y;

    var smoo :f32 = smoothedCircle(vec2f(0.5,0.5),0.175,0.1,in.uv);
    //let smootheppa = smoothstep(colorSpewingBallRadius,colorSpewingBallRadiusOuterEdge,distance(in.uv.xy,uniforms.iMouse.xy/uniforms.iResolution.xy));
    //Something is wrong here,but what is it?!?
    let smootheppa = smoothstep(colorSpewingBallRadius,colorSpewingBallRadiusOuterEdge,distance(in.uv,uniforms.iMouse.xy/uniforms.iResolution.xy));
    let mouseInput = (1.0- smootheppa)*uniforms.iMouse.z;


    //float mouseInput = (1.0-smoothstep(colorSpewingBallRadius,colorSpewingBallRadiusOuterEdge,distance(uv.xy,iMouse.xy/iResolution.xy)))*iMouse.z;

    //reading previous frame and diminishing color by a factor : also clamping to keep values from going haywire.
    var texCol :vec3f = textureSample(t_diffuse,s_diffuse,in.uv).rgb;
    texCol = clamp(texCol,vec3f(0.0),vec3f(1.0));
    var adjustments :vec3f  = vec3f(smoo*0.01);
    //var dutchColors : vec3f = (0.5 + 0.5*cos(uniforms.iTime+in.uv.xyx+vec3f(0,2,4)));


    let dutchColors: vec3<f32> = vec3<f32>(0.5) + vec3<f32>(0.5) * cos(uniforms.iTime + vec3<f32>(in.uv.x, in.uv.x, in.uv.x) + vec3<f32>(0.0, 2.0, 4.0));
    if(mouseInput>0.01){

    adjustments+=dutchColors*mouseInput*0.01; //inputting mouse input..
    //adjustments+=mouseInput*0.01; //inputting mouse input..
    }
    //mouse input to alpha channel - used in buffer A to modify distortion strength.
    return vec4(adjustments,mouseInput*0.0001);

}


