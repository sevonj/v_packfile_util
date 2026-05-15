// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

struct Uniforms {
    view: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

struct VIn  {
    @location(0) pos: vec3<f32>,
    @location(1) uv0: vec2<i32>,
}
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv0: vec2<f32>,
}

const UV_SCALE: f32 = 1.0 / 1024.0;

@vertex
fn vs_main(v: VIn) -> VOut {
    var out: VOut;
    out.clip = u.view * vec4<f32>(v.pos, 1.0);
    out.world_pos = v.pos;
    out.uv0 = vec2<f32>(v.uv0) * UV_SCALE;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let n = normalize(cross(dpdx(in.world_pos), dpdy(in.world_pos)));
    let l = normalize(u.light_dir);
    let dif = 0.85 * max(dot(n, l), 0.0);
    let lit = 0.15 + 0.85 * dif;
    let color = textureSample(t_diffuse, s_diffuse, in.uv0).rgb;
    return vec4<f32>(color * lit, 1.0);
}
