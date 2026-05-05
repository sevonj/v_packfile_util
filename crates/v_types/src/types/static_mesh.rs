use crate::Quaternion;
use crate::Vector;
use crate::VolitionError;
use crate::util::*;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StaticMesh {
    pub magic: i32,
    pub version: i16,
    pub mesh_flags: i16,

    pub unk_08: i32,
    pub num_textures: i16,
    /// Probably. number of "weapon_handle"s
    pub num_navpoints: i16,

    pub unk_10: i32,

    pub bounding_center: Vector,
    pub bounding_radius: f32,
}

impl StaticMesh {
    pub const SIGNATURE: i32 = 0x424BD00D;
    pub const VERSION: i16 = 33;

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let magic = read_i32_le(buf, 0);
        if magic != Self::SIGNATURE {
            return Err(VolitionError::UnexpectedValue {
                field: "StaticMesh::magic",
                expected: Self::SIGNATURE,
                got: magic,
            });
        }

        let version = read_i16_le(buf, 0x4);
        if version != Self::VERSION {
            return Err(VolitionError::UnknownStaticMeshVersion(version));
        }

        Ok(Self {
            magic,
            version,
            mesh_flags: read_i16_le(buf, 0x6),

            unk_08: read_i32_le(buf, 0x8),
            num_textures: read_i16_le(buf, 0xc),
            num_navpoints: read_i16_le(buf, 0xe),

            unk_10: read_i32_le(buf, 0x10),

            bounding_center: Vector::from_data(&buf[0x14..])?,
            bounding_radius: read_f32_le(buf, 0x20),
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

    // use super::*;

    // #[test]
    // fn test_static_mesh_size() {
    //     assert_eq!(size_of::<StaticMesh>(), 0x40);
    // }
}
