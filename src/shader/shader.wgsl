struct VertexOutput {
    [[location(0)]] tex_coord: vec2<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] norpos: vec3<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[block]]
struct Uniforms {
    mvp: mat4x4<f32>;
    transpose_inverse_modelview: mat3x3<f32>;
};
[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] position: vec4<f32>,
    [[location(1)]] normal: vec3<f32>,
    [[location(2)]] tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.normal = uniforms.transpose_inverse_modelview * normal;
    out.position = uniforms.mvp * position;
    out.norpos = out.position.xyz / out.position.w;
    out.tex_coord = tex_coord;
    return out;
}

[[group(0), binding(1)]]
var r_color: texture_2d<u32>;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let tex = textureLoad(r_color, vec2<i32>(in.tex_coord * 256.0), 0);
    let v = f32(tex.x) / 255.0;
    return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}

[[stage(fragment)]]
fn fs_wire() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.5, 0.0, 0.5);
}