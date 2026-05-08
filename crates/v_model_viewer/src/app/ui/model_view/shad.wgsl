struct Uniforms {
    view: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad: f32,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VIn  { @location(0) pos: vec3<f32> }
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
}

@vertex
fn vs_main(v: VIn) -> VOut {
    var out: VOut;
    out.clip = u.view * vec4<f32>(v.pos, 1.0);
    out.world_pos = v.pos;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let n = normalize(cross(dpdx(in.world_pos), dpdy(in.world_pos)));
    let l = normalize(u.light_dir);
    let dif = 0.85 * max(dot(n, l), 0.0);
    let lit = 0.15 + 0.85 * dif;
    return vec4<f32>(lit, lit, lit, 1.0);
}