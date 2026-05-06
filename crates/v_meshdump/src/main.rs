use std::path::PathBuf;

use v_types::Material;
use v_types::MaterialBlock;
use v_types::MaterialTextureEntry;
use v_types::MaterialUnknown3;
use v_types::StaticMesh;
use v_types::StaticMeshNavPoint;
use v_types::util::*;

fn main() {
    let path = PathBuf::from("samples/meshes_extracted/aisha.cmesh_pc");
    // let path = PathBuf::from("samples/meshes_extracted/acousticguitar.smesh_pc");
    //let path = PathBuf::from("samples/meshes_extracted/box.smesh_pc");
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
    data_offset += size_of::<MaterialBlock>();

    let num_materials = material_block.num_materials as usize;
    let num_mat_consts = material_block.num_shader_consts as usize;
    let num_mat_unknown3 = material_block.num_mat_unknown3 as usize;

    let mut materials = Vec::with_capacity(num_materials);
    for _ in 0..num_materials {
        materials.push(Material::from_data(&buf[data_offset..]).unwrap());
        data_offset += size_of::<Material>();
    }

    let mut mat_unk16b: Vec<[u8; 16]> = Vec::with_capacity(num_materials);
    for _ in 0..num_materials {
        mat_unk16b.push(read_bytes(&buf, data_offset));
        data_offset += 16;
    }

    align_16(&mut data_offset);
    let mut mat_consts = Vec::with_capacity(num_mat_consts);
    for _ in 0..num_mat_consts {
        mat_consts.push(read_f32_le(&buf, data_offset));
        data_offset += 4;
    }

    let mut mat_tex_entries = Vec::with_capacity(num_materials);
    for _ in 0..(num_materials * 16) {
        mat_tex_entries.push(MaterialTextureEntry::from_data(&buf[data_offset..]).unwrap());
        data_offset += size_of::<MaterialTextureEntry>();
    }

    let mut mat_unknown3 = Vec::with_capacity(num_mat_unknown3);
    for _ in 0..num_mat_unknown3 {
        mat_unknown3.push(MaterialUnknown3::from_data(&buf[data_offset..]).unwrap());
        data_offset += size_of::<MaterialUnknown3>();
    }

    let mut mat_unknown4 = vec![];
    for unk3 in &mat_unknown3 {
        for _ in 0..unk3.num_mat_unk4 {
            mat_unknown4.push(read_i32_le(&buf, data_offset));
            data_offset += 4;
        }
    }

    println!("{smesh:#?}");
    println!("tex flags(?):     {tex_flags:#?}");
    println!("tex names:        {tex_names:#?}");
    println!("navpoints:        {num_navpoints:#?}");
    println!("bones(?):         {unk_24:#?}");
    println!("material_block:   {material_block:#?}");
    println!("materials:        {materials:#?}");
    println!("mat_unk16b:       {mat_unk16b:#?}");
    println!("mat_consts:       {mat_consts:#?}");
    println!("mat_tex_entries:  {mat_tex_entries:#?}");
    println!("mat_unknown3:     {mat_unknown3:#?}");
    println!("mat_unknown4:     {mat_unknown4:#?}");

    println!("meh {data_offset:#X?}");
}
