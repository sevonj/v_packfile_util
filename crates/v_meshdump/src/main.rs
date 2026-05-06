use std::path::PathBuf;

use v_types::MaterialBlock;
use v_types::Mesh;
use v_types::MeshData;
use v_types::StaticMesh;
use v_types::StaticMeshNavPoint;
use v_types::VertexBuffer;
use v_types::util::*;

fn main() {
    let path = PathBuf::from("samples/meshes_extracted/aisha.cmesh_pc");
    // let path = PathBuf::from("samples/meshes_extracted/acousticguitar.smesh_pc");
    // let path = PathBuf::from("samples/meshes_extracted/box.smesh_pc");
    let buf = std::fs::read(path).unwrap();

    let smesh = StaticMesh::from_data(&buf).unwrap();

    let num_textures = smesh.num_textures as usize;
    let num_navpoints = smesh.num_navpoints as usize;
    let num_unk_24 = smesh.unk_num_24 as usize;

    let mut data_offset = 0x40;

    // probably tex flags. at least it's texture-something
    let mut tex_flags = Vec::with_capacity(num_textures);
    for _ in 0..num_textures {
        tex_flags.push(read_i32_le(&buf, data_offset));
        data_offset += 4;
    }

    align_16(&mut data_offset);
    data_offset += 1; // for some there's an extra null byte at start 
    let mut tex_names = Vec::with_capacity(num_textures);
    for _ in 0..num_textures {
        let name = read_cstr(&buf, data_offset).unwrap();
        data_offset += name.len();
        data_offset += 1; // nullterm
        tex_names.push(name);
    }

    let mut navpoints = Vec::with_capacity(num_navpoints);
    if num_navpoints > 0 {
        align_16(&mut data_offset);
        for _ in 0..num_navpoints {
            navpoints.push(StaticMeshNavPoint::from_data(&buf[data_offset..]));
            data_offset += size_of::<StaticMeshNavPoint>();
        }
    }

    let mut unk_24 = Vec::with_capacity(num_unk_24);
    if num_unk_24 > 0 {
        align_16(&mut data_offset);
        for _ in 0..num_unk_24 {
            unk_24.push(read_i32_le(&buf, data_offset));
            data_offset += 4;
        }
    }

    let material_block = MaterialBlock::from_data(&buf[data_offset..]).unwrap();
    let _materials_data = {
        let (data, len) = material_block.read_materials(&buf[data_offset..]).unwrap();
        data_offset += len;
        data
    };

    println!("{smesh:#?}");

    let mesh = Mesh::from_data(&buf[data_offset..]).unwrap();
    data_offset += size_of::<Mesh>();
    if smesh.unk_2c != 0 {
        data_offset += 20;
    }
    align_16(&mut data_offset);
    let (_sm_a, _sm_b, sm_len) = mesh.read_submeshes(&buf[data_offset..]).unwrap();
    data_offset += sm_len;

    let mesh_buf = MeshData::from_data(&buf[data_offset..]).unwrap();
    data_offset += size_of::<MeshData>();

    let num_vertex_buffers = mesh_buf.num_vertex_buffers as usize;
    let mut vertex_buffers = Vec::with_capacity(num_vertex_buffers);
    for _ in 0..num_vertex_buffers {
        vertex_buffers.push(VertexBuffer::from_data(&buf[data_offset..]).unwrap());
        data_offset += size_of::<VertexBuffer>();
    }

    println!("{mesh_buf:#?}");
    println!("{vertex_buffers:#X?}");

    println!("meh {data_offset:#X?}");
}
