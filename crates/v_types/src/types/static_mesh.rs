use crate::Matlib;
use crate::Quaternion;
use crate::Vector;
use crate::VolitionError;
use crate::types::mesh::Mesh;
use crate::util::*;

pub const MAX_TEXTURES: u16 = 100;
pub const MAX_NAVPOINTS: u16 = 100;
pub const MAX_BONES: u32 = 500;

/// Deserialized cmesh/smesh
#[derive(Debug, Clone)]
pub struct StaticMesh {
    pub header: StaticMeshHeader,
    /// Probably. At least matches texture count.
    pub texture_flags: Vec<i32>,
    pub texture_names: Vec<String>,
    pub navpoints: Vec<StaticMeshNavPoint>,
    /// Maybe.
    pub bone_indices: Vec<i32>,
    pub matlib: Matlib,
    pub mesh: Mesh,
}

impl StaticMesh {
    pub fn from_data(buf: &[u8], data_offset: &mut usize) -> Result<Self, VolitionError> {
        let header = StaticMeshHeader::from_data(buf)?;

        let num_textures = header.num_textures as usize;
        let num_navpoints = header.num_navpoints as usize;
        let num_bones = header.num_bones as usize;

        *data_offset += 0x40;

        let mut texture_flags = Vec::with_capacity(num_textures);
        for _ in 0..num_textures {
            texture_flags.push(read_i32_le(buf, *data_offset));
            *data_offset += 4;
        }

        align(data_offset, 16);
        *data_offset += 1; // for some there's an extra null byte at start 
        let mut texture_names = Vec::with_capacity(num_textures);
        for _ in 0..num_textures {
            let name = read_cstr(buf, *data_offset)?;
            *data_offset += name.len();
            *data_offset += 1; // nullterm
            texture_names.push(name.to_string());
        }

        let mut navpoints = Vec::with_capacity(num_navpoints);
        if num_navpoints > 0 {
            align(data_offset, 16);
            for _ in 0..num_navpoints {
                navpoints.push(StaticMeshNavPoint::from_data(&buf[*data_offset..])?);
                *data_offset += size_of::<StaticMeshNavPoint>();
            }
        }

        let mut bone_indices = Vec::with_capacity(num_bones);
        if num_bones > 0 {
            align(data_offset, 16);
            for _ in 0..num_bones {
                bone_indices.push(read_i32_le(buf, *data_offset));
                *data_offset += 4;
            }
        }

        align(data_offset, 4);
        let matlib = Matlib::from_data(buf, data_offset)?;

        let mesh = Mesh::from_data(buf, data_offset, header.unk_2c)?;

        Ok(Self {
            header,
            texture_flags,
            texture_names,
            navpoints,
            bone_indices,
            matlib,
            mesh,
        })
    }

    pub fn dump_wavefront_cpu(&self) -> String {
        let mut out = String::new();

        let mut next_index = 1;
        for (i, submesh) in self.mesh.submeshes.iter().enumerate() {
            let Some(cpu_submesh) = &submesh.cpu else {
                continue;
            };
            out += &format!("g submesh_{}\n", i);

            let vbuf = &submesh.cpu_vdata;
            let mut voff = 0;
            let mut added_vertices = 0;
            for vhead in &cpu_submesh.vertex_headers {
                assert_eq!(vhead.stride, 12);
                for _ in 0..vhead.num_vertices {
                    let v = Vector::from_data(&vbuf[voff..]).unwrap();
                    out += &format!("v {} {} {}\n", v.x, v.y, v.z);
                    voff += 12;
                }
                added_vertices += vhead.num_vertices as usize;
            }

            let ibuf = &submesh.cpu_idata;
            let mut ioff = 0;
            for _ in 0..cpu_submesh.index_header.num_indices - 2 {
                let a = next_index + read_u16_le(ibuf, ioff) as usize;
                let b = next_index + read_u16_le(ibuf, ioff + 2) as usize;
                let c = next_index + read_u16_le(ibuf, ioff + 4) as usize;
                out += &format!("f {a} {b} {c}\n");
                ioff += 2;
            }
            next_index += added_vertices;
        }

        out
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct StaticMeshHeader {
    pub magic: i32,
    pub version: i16,
    pub mesh_flags: i16,
    pub unk_08: i32,
    pub num_textures: u16,
    pub num_navpoints: u16,
    pub unk_10: i32,
    pub bounding_center: Vector,
    pub bounding_radius: f32,
    /// maybe?
    pub num_bones: u32,
    pub unk_28: i32,
    pub unk_2c: i32,
}

impl StaticMeshHeader {
    pub const SIGNATURE: i32 = 0x424BD00D;
    pub const VERSION: i16 = 33;

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let magic = read_i32_le(buf, 0);
        if magic != Self::SIGNATURE {
            return Err(VolitionError::ExpectedExactValue {
                field: "StaticMesh::magic",
                expected: Self::SIGNATURE,
                got: magic,
            });
        }

        let version = read_i16_le(buf, 0x4);
        if version != Self::VERSION {
            return Err(VolitionError::UnknownStaticMeshVersion(version));
        }

        let num_textures = read_u16_le(buf, 0xc);
        if num_textures > MAX_TEXTURES {
            return Err(VolitionError::ValueTooHigh {
                field: "StaticMeshHeader::num_textures",
                max: MAX_TEXTURES as usize,
                got: num_textures as usize,
            });
        }

        let num_navpoints = read_u16_le(buf, 0xe);
        if num_navpoints > MAX_NAVPOINTS {
            return Err(VolitionError::ValueTooHigh {
                field: "StaticMeshHeader::num_navpoints",
                max: MAX_NAVPOINTS as usize,
                got: num_navpoints as usize,
            });
        }

        let num_bones = read_u32_le(buf, 0x24);
        if num_bones > MAX_BONES {
            return Err(VolitionError::ValueTooHigh {
                field: "StaticMeshHeader::num_bones",
                max: MAX_BONES as usize,
                got: num_bones as usize,
            });
        }

        Ok(Self {
            magic,
            version,
            mesh_flags: read_i16_le(buf, 0x6),
            unk_08: read_i32_le(buf, 0x8),
            num_textures,
            num_navpoints,
            unk_10: read_i32_le(buf, 0x10),
            bounding_center: Vector::from_data(&buf[0x14..])?,
            bounding_radius: read_f32_le(buf, 0x20),
            num_bones,
            unk_28: read_i32_le(buf, 0x28),
            unk_2c: read_i32_le(buf, 0x2c),
        })
    }
}

/// Navigation reference point
/// Used for IK?
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StaticMeshNavPoint {
    /// name to reference nav point by
    pub name: [u8; Self::MAX_NAME_LENGTH],
    /// vid this navp is attached to.
    pub vid: i32,
    /// position of navpoint in object coords
    pub pos: Vector,
    /// quaternion representation of navpoint
    pub orient: Quaternion,
}

impl StaticMeshNavPoint {
    pub const MAX_NAME_LENGTH: usize = 64;

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            name: read_bytes(buf, 0x0),
            vid: read_i32_le(buf, Self::MAX_NAME_LENGTH),
            pos: Vector::from_data(&buf[Self::MAX_NAME_LENGTH + 4..])?,
            orient: Quaternion::from_data(&buf[Self::MAX_NAME_LENGTH + 16..])?,
        })
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_parse_every_smesh() {
        // Unpacked meshes.vpp_pc
        let samples_path = PathBuf::from("../../samples/meshes_extracted");

        let mut num_failed = 0;
        for entry in std::fs::read_dir(samples_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if !entry.metadata().unwrap().is_file() || path.extension().unwrap() != "smesh_pc" {
                continue;
            }

            let buf = std::fs::read(&path).unwrap();
            let mut offset = 0;
            if let Err(e) = StaticMesh::from_data(&buf, &mut offset) {
                num_failed += 1;
                println!("ERR: {path:?} {offset:#X?} {e}");
            }
        }
        println!("num_failed: {num_failed:?}");
        assert_eq!(num_failed, 0);
    }

    #[test]
    fn test_parse_every_smesh_reaches_end() {
        // Unpacked meshes.vpp_pc
        let samples_path = PathBuf::from("../../samples/meshes_extracted");

        let mut num_failed = 0;
        for entry in std::fs::read_dir(samples_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if !entry.metadata().unwrap().is_file() || path.extension().unwrap() != "smesh_pc" {
                continue;
            }

            let buf = std::fs::read(&path).unwrap();
            let mut offset = 0;
            StaticMesh::from_data(&buf, &mut offset).unwrap();

            align(&mut offset, 16);

            if offset != buf.len() {
                num_failed += 1;
                println!(
                    "Didn't reach end: {path:?} off: {offset:#X?}, end: {:#X?}",
                    buf.len()
                );
            }
        }
        println!("num_failed: {num_failed:?}");
        assert_eq!(num_failed, 0);
    }

    // #[test]
    // fn test_static_mesh_size() {
    //     assert_eq!(size_of::<StaticMesh>(), 0x40);
    // }
}
