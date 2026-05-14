use std::collections::HashMap;
use std::path::PathBuf;

use v_types::AABB;
use v_types::StaticMesh;
use v_types::Vector;
use v_types::VertexBuffer;
use v_types::util::*;

pub struct ModelData {
    pub smesh: StaticMesh,
    pub g_smesh: Option<Vec<u8>>,
    pub file_path: PathBuf,
}

impl ModelData {
    pub fn try_load_g_smesh(&mut self) {
        let Some(cpu_ext) = self.file_path.extension().and_then(|e| e.to_str()) else {
            return;
        };
        let file_path = self.file_path.with_extension(format!("g_{cpu_ext}"));
        self.g_smesh = std::fs::read(file_path).ok();
    }

    pub fn replace_with_gltf(&mut self, document: &gltf::Document, buffers: &[gltf::buffer::Data]) {
        let mut new_smesh = self.smesh.clone();

        let mut lod_nodes_map = HashMap::new();
        let mut shadow_lod_nodes_map = HashMap::new();

        for scene in document.scenes() {
            for node in scene.nodes() {
                let Some(name) = node.name() else {
                    println!("WARNING: ignoring unnamed node");
                    continue;
                };
                let mut split = name.split("_");
                if split.next().is_none_or(|s| s != "lod") {
                    println!("WARNING: ignoring unrecognized node: {name:?}");
                    continue;
                };
                let Some(lod_level) = split.next().and_then(|s| s.parse::<usize>().ok()) else {
                    println!("ERROR: couldn't parse lod level from {name:?}");
                    return;
                };
                match split.next() {
                    None => {
                        if lod_nodes_map.insert(lod_level, node).is_some() {
                            println!("ERROR: repeat lod: {name:?}");
                            return;
                        }
                    }
                    Some("shadow") => {
                        if split.next().is_some() {
                            println!("ERROR: couldn't parse node type {name:?}");
                            return;
                        } else if shadow_lod_nodes_map.insert(lod_level, node).is_some() {
                            println!("ERROR: repeat lod: {name:?}");
                            return;
                        }
                    }
                    _ => {
                        println!("ERROR: couldn't parse node type {name:?}");
                        return;
                    }
                }
            }
        }
        let has_shadow_lods = !shadow_lod_nodes_map.is_empty();

        if has_shadow_lods && shadow_lod_nodes_map.len() != lod_nodes_map.len() {
            println!("ERROR: number of shadow lods must match lods");
            return;
        }
        let mut lod_nodes = vec![];
        for i in 0..lod_nodes_map.len() {
            let Some(node) = lod_nodes_map.get(&i) else {
                println!("ERROR: failed to get node for lod {i:?}. Did you number them correctly?");
                return;
            };
            lod_nodes.push(node);
        }
        let mut shadow_lod_nodes = vec![];
        for i in 0..shadow_lod_nodes_map.len() {
            let Some(node) = shadow_lod_nodes_map.get(&i) else {
                println!(
                    "ERROR: failed to get node for shadow lod {i:?}. Did you number them correctly?"
                );
                return;
            };
            shadow_lod_nodes.push(node);
        }

        if lod_nodes.len() != new_smesh.lod_meshes.len() {
            println!(
                "ERROR: GLTF has different number of lods than target: {} vs {}",
                lod_nodes.len(),
                new_smesh.lod_meshes.len()
            );
            return;
        }

        if !has_shadow_lods && new_smesh.mesh_header.has_cpu_geometry() {
            println!("ERROR: GLTF doesn't have shadow lods, but target does.");
            return;
        } else if has_shadow_lods && !new_smesh.mesh_header.has_cpu_geometry() {
            println!("ERROR: GLTF has shadow lods, but target doesn't.");
            return;
        }

        let mut bounds = AABB {
            min: Vector {
                x: f32::INFINITY,
                y: f32::INFINITY,
                z: f32::INFINITY,
            },
            max: Vector {
                x: f32::NEG_INFINITY,
                y: f32::NEG_INFINITY,
                z: f32::NEG_INFINITY,
            },
        };

        let mut new_g_smesh = vec![];
        for (i, lod_mesh) in new_smesh.lod_meshes.iter_mut().enumerate() {
            {
                let tgt_mesh = &mut lod_mesh.gpu_geometry;
                if tgt_mesh.surfaces.len() != 1 {
                    println!(
                        "ERROR: Target mesh has {} surfs. Only 1 is supported.",
                        tgt_mesh.surfaces.len()
                    );
                    return;
                }
                let tgt_surf = &mut tgt_mesh.surfaces[0];
                if tgt_mesh.vertex_headers.len() != 1 {
                    println!(
                        "ERROR: Target mesh has {} vertex buffers. Only 1 is supported.",
                        tgt_mesh.vertex_headers.len()
                    );
                    return;
                }
                let tgt_vertex_header = &mut tgt_mesh.vertex_headers[0];

                let Some(src_mesh) = lod_nodes[i].mesh() else {
                    println!("ERROR: Source lod node has no mesh",);
                    return;
                };

                if src_mesh.primitives().len() != 1 {
                    println!(
                        "ERROR: Source mesh has {} surfs. Only 1 is supported.",
                        tgt_mesh.surfaces.len()
                    );
                    return;
                }
                let src_prim = src_mesh.primitives().nth(0).unwrap();
                let src_data = gltf_geom(src_prim, buffers);
                tgt_vertex_header.num_vertices = src_data.positions.len() as u32;
                tgt_mesh.index_header.num_indices = src_data.indices.len() as u32;
                tgt_surf.num_indices = src_data.indices.len() as u16;
                bounds = bounds.union(&calc_bbox(&src_data.positions));

                let (mut vbuf, mut ibuf) = generate_buffers(src_data, tgt_vertex_header);
                new_g_smesh.append(&mut vbuf);
                while !new_g_smesh.len().is_multiple_of(16) {
                    new_g_smesh.push(0);
                }
                new_g_smesh.append(&mut ibuf);
                while !new_g_smesh.len().is_multiple_of(16) {
                    new_g_smesh.push(0);
                }
            }

            if has_shadow_lods {
                let tgt_mesh = lod_mesh.cpu_geometry.as_mut().unwrap();

                if tgt_mesh.surfaces.len() != 1 {
                    println!(
                        "ERROR: Target mesh has {} surfs. Only 1 is supported.",
                        tgt_mesh.surfaces.len()
                    );
                    return;
                }
                let tgt_surf = &mut tgt_mesh.surfaces[0];
                if tgt_mesh.vertex_headers.len() != 1 {
                    println!(
                        "ERROR: Target mesh has {} vertex buffers. Only 1 is supported.",
                        tgt_mesh.vertex_headers.len()
                    );
                    return;
                }
                let tgt_vertex_header = &mut tgt_mesh.vertex_headers[0];

                let Some(src_mesh) = shadow_lod_nodes[i].mesh() else {
                    println!("ERROR: Source lod node has no mesh",);
                    return;
                };

                if src_mesh.primitives().len() != 1 {
                    println!(
                        "ERROR: Source mesh has {} surfs. Only 1 is supported.",
                        tgt_mesh.surfaces.len()
                    );
                    return;
                }
                let src_prim = src_mesh.primitives().nth(0).unwrap();
                let src_data = gltf_geom(src_prim, buffers);
                tgt_vertex_header.num_vertices = src_data.positions.len() as u32;
                tgt_mesh.index_header.num_indices = src_data.indices.len() as u32;
                tgt_surf.num_indices = src_data.indices.len() as u16;
                bounds = bounds.union(&calc_bbox(&src_data.positions));

                let (vbuf, ibuf) = generate_buffers(src_data, tgt_vertex_header);
                lod_mesh.cpu_vdata = vbuf;
                lod_mesh.cpu_idata = ibuf;
            }
        }

        new_smesh.header.bounding_center = bounds.center();
        new_smesh.header.bounding_radius = bounds.radius();
        new_smesh.mesh_header.bbox = bounds;

        self.smesh = new_smesh;
        self.g_smesh = Some(new_g_smesh);
    }
}

fn generate_buffers(src_data: GltfUnpacked, vertex_header: &VertexBuffer) -> (Vec<u8>, Vec<u8>) {
    let stride = 32;
    let mut vbuf = Vec::with_capacity(aligned(src_data.positions.len() * stride, 16));
    for ((p, uv), n) in src_data
        .positions
        .iter()
        .zip(src_data.uvs)
        .zip(src_data.normals)
    {
        vbuf.extend_from_slice(&p[0].to_le_bytes());
        vbuf.extend_from_slice(&p[1].to_le_bytes());
        vbuf.extend_from_slice(&p[2].to_le_bytes());
        if vertex_header.has_bones() {
            vbuf.extend_from_slice(&[0; 8]);
        }
        if vertex_header.has_normals() {
            vbuf.push(((n[0] + 0.5) * 128.0) as u8);
            vbuf.push(((n[1] + 0.5) * 128.0) as u8);
            vbuf.push(((n[2] + 0.5) * 128.0) as u8);
            vbuf.push(0);
        }
        if vertex_header.has_unk_attr() {
            vbuf.extend_from_slice(&[0; 8]);
        }
        for _ in 0..vertex_header.num_uv_channels {
            vbuf.extend_from_slice(&((uv[0] * 1024.0) as i16).to_le_bytes());
            vbuf.extend_from_slice(&((uv[1] * 1024.0) as i16).to_le_bytes());
        }
    }

    let mut ibuf = Vec::with_capacity(aligned(src_data.indices.len() * 2, 16));
    for i in src_data.indices {
        ibuf.extend_from_slice(&(i as u16).to_le_bytes());
    }

    (vbuf, ibuf)
}

#[allow(dead_code)]
struct GltfUnpacked {
    pub indices: Vec<u32>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
}

fn gltf_geom(prim: gltf::Primitive<'_>, buffers: &[gltf::buffer::Data]) -> GltfUnpacked {
    let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));

    let (indices, positions, normals, uvs) = match prim.mode() {
        gltf::mesh::Mode::TriangleStrip => {
            let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
            let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
            let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0) // set 0
                .unwrap()
                .into_f32()
                .collect();
            (indices, positions, normals, uvs)
        }
        gltf::mesh::Mode::Triangles => {
            let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
            let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
            let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0) // set 0
                .unwrap()
                .into_f32()
                .collect();

            (triangle_stripper(&indices), positions, normals, uvs)
        }
        _ => unimplemented!("{:?}", prim.mode()),
    };

    assert_eq!(positions.len(), normals.len());
    assert_eq!(positions.len(), uvs.len());

    GltfUnpacked {
        indices,
        positions,
        normals,
        uvs,
    }
}

pub fn triangle_stripper(indices: &[u32]) -> Vec<u32> {
    use std::collections::HashMap;

    assert!(indices.len().is_multiple_of(3));

    let mut edge_to_tris: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    let num_tris = indices.len() / 3;

    for tri_id in 0..num_tris {
        let base = tri_id * 3;
        let verts = [indices[base], indices[base + 1], indices[base + 2]];
        for e in 0..3 {
            let a = verts[e];
            let b = verts[(e + 1) % 3];
            let edge = (a.min(b), a.max(b));
            edge_to_tris.entry(edge).or_default().push(tri_id);
        }
    }

    let mut visited = vec![false; num_tris];
    let mut result: Vec<u32> = Vec::new();

    for start_tri in 0..num_tris {
        if visited[start_tri] {
            continue;
        }

        visited[start_tri] = true;
        let base = start_tri * 3;
        let [a, b, c] = [indices[base], indices[base + 1], indices[base + 2]];

        if !result.is_empty() {
            // degen
            result.push(*result.last().unwrap());
            result.push(a);
            if !result.len().is_multiple_of(2) {
                result.push(a);
            }
        }

        result.push(a);
        result.push(b);
        result.push(c);

        let mut last_edge = (b, c);
        loop {
            let edge_key = (last_edge.0.min(last_edge.1), last_edge.0.max(last_edge.1));
            let neighbor = edge_to_tris
                .get(&edge_key)
                .and_then(|tris| tris.iter().find(|&&t| !visited[t]))
                .copied();

            match neighbor {
                None => break,
                Some(next_tri) => {
                    visited[next_tri] = true;
                    let base = next_tri * 3;
                    let verts = [indices[base], indices[base + 1], indices[base + 2]];

                    let new_vert = verts
                        .iter()
                        .copied()
                        .find(|&v| v != last_edge.0 && v != last_edge.1)
                        .unwrap();

                    result.push(new_vert);
                    last_edge = (last_edge.1, new_vert);
                }
            }
        }
    }

    result
}

pub fn calc_bbox(positions: &[[f32; 3]]) -> AABB {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for pos in positions {
        min_x = min_x.min(pos[0]);
        min_y = min_y.min(pos[1]);
        min_z = min_z.min(pos[2]);
        max_x = max_x.max(pos[0]);
        max_y = max_y.max(pos[1]);
        max_z = max_z.max(pos[2]);
    }

    AABB {
        min: Vector {
            x: min_x,
            y: min_y,
            z: min_z,
        },
        max: Vector {
            x: max_x,
            y: max_y,
            z: max_z,
        },
    }
}
