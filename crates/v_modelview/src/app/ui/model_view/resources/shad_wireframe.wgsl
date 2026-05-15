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

struct VIn  { 
    @location(0) pos: vec3<f32>,
    @location(1) col: vec4<f32>,
}
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) col: vec4<f32>,
}

@vertex
fn vs_main(v: VIn) -> VOut {
    var out: VOut;
    out.clip = u.view * vec4<f32>(v.pos, 1.0);
    out.col = v.col;
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    return in.col;
}
