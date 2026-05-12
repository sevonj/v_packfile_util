use crate::LodMeshData;
use crate::LodMeshHeader;
use crate::MaterialsData;
use crate::Quaternion;
use crate::Surface;
use crate::Vector;
use crate::VolitionError;
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
    pub navpoints: Vec<StaticMeshNavpoint>,
    /// Maybe.
    pub bone_indices: Vec<i32>,
    pub matlib: MaterialsData,
    pub mesh_header: LodMeshHeader,
    pub lod_meshes: Vec<LodMeshData>,
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
                navpoints.push(StaticMeshNavpoint::from_data(&buf[*data_offset..])?);
                *data_offset += size_of::<StaticMeshNavpoint>();
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
        let matlib = MaterialsData::from_data(buf, data_offset)?;

        let mesh_header = LodMeshHeader::from_data(&buf[*data_offset..])?;
        *data_offset += size_of::<LodMeshHeader>();

        let lod_meshes = mesh_header.read_data(buf, data_offset, header.unk_2c)?;

        Ok(Self {
            header,
            texture_flags,
            texture_names,
            navpoints,
            bone_indices,
            matlib,
            mesh_header,
            lod_meshes,
        })
    }

    pub fn dump_wavefront(&self, g_smesh: Option<&[u8]>, separate_surfaces: bool) -> String {
        let mut out = String::new();

        out += "# v_modelview StaticMesh dump\n";
        if self.mesh_header.has_cpu_geometry() {
            out += "# INFO: CPU file has geometry.\n";
        } else {
            out += "# INFO: CPU file doesn't have geometry.\n";
        }
        if separate_surfaces {
            out += "# INFO: separate_surfaces enabled. Every surface is a separate object.\n";
        }
        if g_smesh.is_none() {
            out += "# WARNING: GPU file not provided. Dumping only CPU file contents.\n";
        }

        fn write_vertices(out: &mut String, vhead: &super::VertexBuffer, vbuf: &[u8]) {
            let stride = vhead.stride as usize;
            for i in 0..vhead.num_vertices as usize {
                let v_off = i * stride;
                let pos = Vector::from_data(&vbuf[v_off..]).unwrap();
                let (u, v) = if vhead.num_uv_channels > 0 {
                    let uv_offset = v_off + vhead.off_uv();
                    (
                        read_i16_le(vbuf, uv_offset) as f32 / 1024.0,
                        read_i16_le(vbuf, uv_offset + 2) as f32 / 1024.0,
                    )
                } else {
                    (0.0, 0.0)
                };
                let normal_offset = v_off + vhead.off_normal();
                let (nx, ny, nz) = if vhead.has_normals() {
                    (
                        vbuf[normal_offset] as f32 / 128.0 - 0.5,
                        vbuf[normal_offset + 1] as f32 / 128.0 - 0.5,
                        vbuf[normal_offset + 2] as f32 / 128.0 - 0.5,
                    )
                } else {
                    (0.0, 0.0, 0.0)
                };
                *out += &format!("v {} {} {}\n", pos.x, pos.y, pos.z);
                *out += &format!("vt {u} {v}\n");
                *out += &format!("vn {nx} {ny} {nz}\n");
            }
        }

        fn write_indices(out: &mut String, base_index: usize, surf: &Surface, ibuf: &[u8]) {
            let start = surf.start_index as usize;
            let end = start + surf.num_indices as usize;

            for i in start..end - 2 {
                let a = base_index + read_u16_le(ibuf, i * 2) as usize;
                let b = base_index + read_u16_le(ibuf, i * 2 + 2) as usize;
                let c = base_index + read_u16_le(ibuf, i * 2 + 4) as usize;
                if (i - start).is_multiple_of(2) {
                    *out += &format!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n");
                } else {
                    *out += &format!("f {a}/{a}/{a} {c}/{c}/{c} {b}/{b}/{b}\n");
                }
            }
        }

        let gpu_buffers = g_smesh.map(|g_smesh| self.gpu_buffers(g_smesh).unwrap());
        if let Some(gpu_buffers) = &gpu_buffers {
            assert_eq!(gpu_buffers.len(), self.lod_meshes.len());
        }

        let mut base_index = 1;
        for (lod, mesh) in self.lod_meshes.iter().enumerate() {
            if let Some(gpu_buffers) = &gpu_buffers {
                let (vbufs, ibuf) = &gpu_buffers[lod];
                let geom = &mesh.gpu_geometry;

                if !separate_surfaces {
                    out += &format!("o lod{lod}_gpu\n");
                }

                for (i, surf) in geom.surfaces.iter().enumerate() {
                    if separate_surfaces {
                        out += &format!("o lod{lod}_gpu_surf{i}\n");
                    }
                    let mat_name = self.matlib.materials[surf.material as usize].material_hash;
                    out += &format!("usemtl mat_{mat_name:08X}\n");

                    let vhead = &geom.vertex_headers[surf.vbuf as usize];
                    let vbuf = vbufs[surf.vbuf as usize];
                    write_vertices(&mut out, vhead, vbuf);
                    write_indices(&mut out, base_index, surf, ibuf);
                    base_index += vhead.num_vertices as usize;
                }
            }

            if let Some(geom) = &mesh.cpu_geometry {
                if !separate_surfaces {
                    out += &format!("o lod{lod}_cpu\n");
                }

                let vbuf = &mesh.cpu_vdata;
                let ibuf = &mesh.cpu_idata;
                for (i, surf) in geom.surfaces.iter().enumerate() {
                    if separate_surfaces {
                        out += &format!("o lod{lod}_cpu_surf{i}\n");
                    }
                    let mat_name = self.matlib.materials[surf.material as usize].material_hash;
                    out += &format!("usemtl mat_{mat_name:08X}\n");

                    let vhead = &geom.vertex_headers[surf.vbuf as usize];
                    write_vertices(&mut out, vhead, vbuf);
                    write_indices(&mut out, base_index, surf, ibuf);
                    base_index += vhead.num_vertices as usize;
                }
            }
        }

        out
    }

    /// Return: vec<(vbufs, ibuf)>
    #[allow(clippy::type_complexity)]
    pub fn gpu_buffers<'a>(
        &self,
        buf: &'a [u8],
    ) -> Result<Vec<(Vec<&'a [u8]>, &'a [u8])>, VolitionError> {
        let mut offset = 0;
        let mut datas = vec![];
        for lod_mesh in &self.lod_meshes {
            align(&mut offset, 16);
            let mut vbufs = vec![];
            for vertex_head in &lod_mesh.gpu_geometry.vertex_headers {
                let len = vertex_head.num_vertices as usize * vertex_head.stride as usize;
                let end = offset + len;
                vbufs.push(&buf[offset..end]);
                offset += len;
            }

            align(&mut offset, 16);
            let len = lod_mesh.gpu_geometry.index_header.num_indices as usize * 2;
            let end = offset + len;
            datas.push((vbufs, &buf[offset..end]));
            offset += len;
        }
        Ok(datas)
    }
}

/// 1:1 from disk
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

/// 1:1 from disk
/// Navigation reference point
/// Used for IK?
#[derive(Debug, Clone)]
#[repr(C)]
pub struct StaticMeshNavpoint {
    /// name to reference nav point by
    pub name: [u8; Self::MAX_NAME_LENGTH],
    /// vid this navp is attached to.
    pub vid: i32,
    /// position of navpoint in object coords
    pub pos: Vector,
    /// quaternion representation of navpoint
    pub orient: Quaternion,
}

impl StaticMeshNavpoint {
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

    pub fn name(&self) -> Result<&str, VolitionError> {
        read_cstr(&self.name, 0)
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
        let mut num_success = 0;
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
            } else {
                num_success += 1
            }
        }
        println!("num_failed: {num_failed:?}");
        assert_eq!(num_failed, 0);
        assert_eq!(num_success, 659);
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

    #[test]
    fn test_parse_every_cmesh() {
        // Unpacked meshes.vpp_pc
        let samples_path = PathBuf::from("../../samples/meshes_extracted");

        let mut num_failed = 0;
        let mut num_success = 0;
        for entry in std::fs::read_dir(samples_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if !entry.metadata().unwrap().is_file() || path.extension().unwrap() != "cmesh_pc" {
                continue;
            }

            let buf = std::fs::read(&path).unwrap();
            let mut offset = 0;
            if let Err(e) = StaticMesh::from_data(&buf, &mut offset) {
                num_failed += 1;
                println!("ERR: {path:?} {offset:#X?} {e}");
            } else {
                num_success += 1
            }
        }
        println!("num_failed: {num_failed:?}");
        assert_eq!(num_failed, 0);
        assert_eq!(num_success, 3028);
    }

    // #[test]
    // fn test_static_mesh_size() {
    //     assert_eq!(size_of::<StaticMesh>(), 0x40);
    // }
}
