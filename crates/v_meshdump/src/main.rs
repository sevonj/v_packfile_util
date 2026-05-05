use std::path::PathBuf;

use v_types::StaticMesh;
use v_types::StaticMeshNavPoint;
use v_types::util::read_cstr;
use v_types::util::read_i16_le;

fn main() {
    let path = PathBuf::from("samples/meshes_extracted/acousticguitar.smesh_pc");
    let buf = std::fs::read(path).unwrap();

    let smesh = StaticMesh::from_data(&buf).unwrap();

    let num_textures = smesh.num_textures as usize;
    let num_navpoints = smesh.num_navpoints as usize;

    // probably tex flags. at least it's texture-something
    let off_tex_flags = 0x40;
    let len_tex_flags = (num_textures * 2).div_ceil(16) * 16;
    let mut tex_flags = Vec::with_capacity(num_textures);
    for i in 0..num_textures {
        let offset = off_tex_flags + i * 2;
        tex_flags.push(read_i16_le(&buf, offset));
    }

    let off_tex_names = off_tex_flags + len_tex_flags;
    let mut len_tex_names = 1; // for some there's an extra null byte at start 
    let mut tex_names = Vec::with_capacity(num_textures);
    for _ in 0..num_textures {
        let offset = off_tex_names + len_tex_names;
        let name = read_cstr(&buf, offset).unwrap();
        len_tex_names += name.len() + 1; // +1 for nullterm
        tex_names.push(name);
    }
    len_tex_names = (len_tex_names).div_ceil(16) * 16;

    let off_navpoints = off_tex_names + len_tex_names;
    let mut navpoints = Vec::with_capacity(num_navpoints);
    for i in 0..num_navpoints {
        let offset = off_navpoints + i * size_of::<StaticMeshNavPoint>();
        navpoints.push(StaticMeshNavPoint::from_data(&buf[offset..]));
    }

    println!("{smesh:#?}");
    println!("{off_tex_flags:#?}, {tex_flags:#?}");
    println!("{off_tex_names:#?}, {tex_names:#?}");
    println!("{off_navpoints:#?}, {navpoints:#?}");

    println!("meh")
}
