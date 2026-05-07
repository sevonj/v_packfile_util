use crate::VolitionError;
use crate::util::*;

#[derive(Debug, Clone)]
pub struct Matlib {
    pub materials: Vec<Material>,
    pub mat_unk1s: Vec<[u8; 16]>,
    pub mat_consts: Vec<f32>,
    pub mat_textures: Vec<MaterialTextureEntry>,
    pub mat_unknown3s: Vec<MaterialUnknown3>,
    pub mat_unknown4s: Vec<i32>,
}

impl Matlib {
    /// Return: (data, len)
    pub fn from_data(buf: &[u8]) -> Result<(Self, usize), VolitionError> {
        let material_block = MaterialBlock::from_data(buf)?;
        let mut data_offset = size_of::<MaterialBlock>();

        let num_materials = material_block.num_materials as usize;
        let num_mat_consts = material_block.num_shader_consts as usize;
        let num_mat_unknown3 = material_block.num_mat_unknown3 as usize;

        let mut materials = Vec::with_capacity(num_materials);
        for _ in 0..num_materials {
            materials.push(Material::from_data(&buf[data_offset..])?);
            data_offset += size_of::<Material>();
        }

        let mut mat_unk1s: Vec<[u8; 16]> = Vec::with_capacity(num_materials);
        for _ in 0..num_materials {
            mat_unk1s.push(read_bytes(buf, data_offset));
            data_offset += 16;
        }

        align_16(&mut data_offset);
        let mut mat_consts = Vec::with_capacity(num_mat_consts);
        for _ in 0..num_mat_consts {
            mat_consts.push(read_f32_le(buf, data_offset));
            data_offset += 4;
        }

        let mut mat_textures = Vec::with_capacity(num_materials);
        for _ in 0..(num_materials * 16) {
            mat_textures.push(MaterialTextureEntry::from_data(&buf[data_offset..])?);
            data_offset += size_of::<MaterialTextureEntry>();
        }

        let mut mat_unknown3s = Vec::with_capacity(num_mat_unknown3);
        for _ in 0..num_mat_unknown3 {
            mat_unknown3s.push(MaterialUnknown3::from_data(&buf[data_offset..])?);
            data_offset += size_of::<MaterialUnknown3>();
        }

        let mut mat_unknown4s = vec![];
        for unk3 in &mat_unknown3s {
            for _ in 0..unk3.num_mat_unk4 {
                mat_unknown4s.push(read_i32_le(buf, data_offset));
                data_offset += 4;
            }
        }
        Ok((
            Self {
                materials,
                mat_unk1s,
                mat_consts,
                mat_textures,
                mat_unknown3s,
                mat_unknown4s,
            },
            data_offset,
        ))
    }
}

/// Appears at least in city chunks and static meshes
#[derive(Debug, Clone)]
#[repr(C)]
pub struct MaterialBlock {
    /// Number of [MaterialData] immediately after this header.
    pub num_materials: i32,
    /// Always Zero.
    pub unknown_04: i32,
    /// Always Zero.
    pub unknown_08: i32,
    /// Always Zero.
    pub unknown_0c: i32,
    /// Shader constants are just standard floats
    pub num_shader_consts: i32,
    /// Always Zero.
    pub unknown_14: i32,
    /// Always Zero.
    pub unknown_18: i32,
    pub num_mat_unknown3: i32,
    /// Always Zero.
    pub unknown_20: i32,
}

impl MaterialBlock {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let unknown_04 = read_i32_le(buf, 0x4);
        if unknown_04 != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_04",
                expected: 0,
                got: unknown_04,
            });
        }

        let unknown_08 = read_i32_le(buf, 0x8);
        if unknown_08 != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_08",
                expected: 0,
                got: unknown_08,
            });
        }

        let unknown_0c = read_i32_le(buf, 0xc);
        if unknown_0c != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_0c",
                expected: 0,
                got: unknown_0c,
            });
        }

        let unknown_14 = read_i32_le(buf, 0x14);
        if unknown_14 != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_14",
                expected: 0,
                got: unknown_14,
            });
        }

        let unknown_18 = read_i32_le(buf, 0x18);
        if unknown_18 != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_18",
                expected: 0,
                got: unknown_18,
            });
        }

        let unknown_20 = read_i32_le(buf, 0x20);
        if unknown_20 != 0 {
            return Err(VolitionError::UnexpectedValue {
                field: "MaterialBlock::unknown_20",
                expected: 0,
                got: unknown_20,
            });
        }

        Ok(Self {
            num_materials: read_i32_le(buf, 0x0),
            unknown_04,
            unknown_08,
            unknown_0c,
            num_shader_consts: read_i32_le(buf, 0x10),
            unknown_14,
            unknown_18,
            num_mat_unknown3: read_i32_le(buf, 0x1c),
            unknown_20,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Material {
    /// name checksum?
    pub shader_hash: i32,
    /// name checksum?
    pub material_hash: i32,
    pub flags: i32,
    pub num_unknown: i16,
    pub num_textures: i16,
    pub unk_10: i16,
    pub unk_12: i16,
    pub runtime_14: i32,
    /* Could be:
     * - num_constants u8
     * - max_constants u8
     * - off_texture i32
     * - off_constant_checksums i32
     * - off_constants i32
     */
}

impl Material {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let runtime_14 = read_i32_le(buf, 0x14);
        // Usually, but not always?
        // if runtime_14 != -1 {
        //     return Err(VolitionError::UnexpectedValue {
        //         field: "Material::runtime_14",
        //         expected: -1,
        //         got: runtime_14,
        //     });
        // }

        Ok(Self {
            shader_hash: read_i32_le(buf, 0x0),
            material_hash: read_i32_le(buf, 0x4),
            flags: read_i32_le(buf, 0x8),
            num_unknown: read_i16_le(buf, 0xc),
            num_textures: read_i16_le(buf, 0xe),
            unk_10: read_i16_le(buf, 0x10),
            unk_12: read_i16_le(buf, 0x12),
            runtime_14,
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MaterialTextureEntry {
    /// Texture index. -1 if entry is unused
    pub index: i16,
    /// Texture flags? -1 if entry is unused
    pub flags: i16,
}

impl MaterialTextureEntry {
    pub const fn placeholder() -> Self {
        Self {
            index: -1,
            flags: -1,
        }
    }

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            index: read_i16_le(buf, 0x0),
            flags: read_i16_le(buf, 0x2),
        })
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MaterialUnknown3 {
    pub unk_00: i32,
    pub unk_04: i32,
    pub num_mat_unk4: i16,
    pub unk_06: i16,
    pub runtime_08: i32,
}

impl MaterialUnknown3 {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            unk_00: read_i32_le(buf, 0x0),
            unk_04: read_i32_le(buf, 0x4),
            num_mat_unk4: read_i16_le(buf, 0x8),
            unk_06: read_i16_le(buf, 0xa),
            runtime_08: read_i32_le(buf, 0xc),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_material_block_size() {
        assert_eq!(size_of::<MaterialBlock>(), 0x24);
    }

    #[test]
    fn test_material_size() {
        assert_eq!(size_of::<Material>(), 0x18);
    }

    #[test]
    fn test_material_unk3() {
        assert_eq!(size_of::<MaterialUnknown3>(), 0x10);
    }
}
