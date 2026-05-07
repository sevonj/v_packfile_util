use crate::Vector;
use crate::VolitionError;
use crate::util::*;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AABB {
    pub min: Vector,
    pub max: Vector,
}

impl AABB {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            min: Vector::from_data(buf)?,
            max: Vector::from_data(&buf[12..])?,
        })
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.min.is_nan() || self.max.is_nan()
    }
}
