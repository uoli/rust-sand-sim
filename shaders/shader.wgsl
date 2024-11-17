
struct VertexOutput {
    @location(0) fragColor: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};


@group(0)
@binding(0)
var<uniform> projection: mat4x4<f32>;
@group(1)
@binding(0)
var<uniform> view: mat4x4<f32>;
@group(2)
@binding(0)
var<uniform> model: mat4x4<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = projection * view * model * vec4(position, 1.0);
    result.fragColor = color;
    result.tex_coord = tex_coord;
    return result;
}

@group(3)
@binding(0)
var r_color: texture_2d<f32>;

@group(3) 
@binding(1) 
var s_sampler: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(r_color, s_sampler, vertex.tex_coord);
    return tex;
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.5, 0.0, 0.5);
}
