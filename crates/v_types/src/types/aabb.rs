use crate::Vector;
use crate::VolitionError;
use crate::util::*;

/// 1:1 from disk
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct AABB {
    pub min: Vector,
    pub max: Vector,
}

impl AABB {
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            min: Vector::from_le_bytes(buf)?,
            max: Vector::from_le_bytes(&buf[12..])?,
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x0..0xc].copy_from_slice(&self.min.to_le_bytes());
        bytes[0xc..0x18].copy_from_slice(&self.max.to_le_bytes());
        bytes
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.min.is_nan() || self.max.is_nan()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_aabb_size() {
        assert_eq!(size_of::<AABB>(), 0x18);
    }

    #[test]
    fn test_aabb_cycle_bytes() {
        let mut buf = vec![];
        buf.extend_from_slice(&3.0_f32.to_le_bytes());
        buf.extend_from_slice(&4.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        buf.extend_from_slice(&7.0_f32.to_le_bytes());
        buf.extend_from_slice(&8.0_f32.to_le_bytes());
        buf.extend_from_slice(&9.0_f32.to_le_bytes());
        let bbox = AABB::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, bbox.to_le_bytes());
    }
}
