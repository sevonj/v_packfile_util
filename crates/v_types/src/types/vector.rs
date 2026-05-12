// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2025 sevonj

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::VolitionError;
use crate::util::*;

/// 1:1 from disk
/// 3D float vector
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector {
    pub fn from_le_bytes(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            x: read_f32_le(buf, 0x0),
            y: read_f32_le(buf, 0x4),
            z: read_f32_le(buf, 0x8),
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x0..0x4].copy_from_slice(&self.x.to_le_bytes());
        bytes[0x4..0x8].copy_from_slice(&self.y.to_le_bytes());
        bytes[0x8..0xc].copy_from_slice(&self.z.to_le_bytes());
        bytes
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }

    /// Magnitude
    pub fn length(&self) -> f32 {
        let sq = self.x * self.x + self.y * self.y + self.z * self.z;
        sq.sqrt()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_vector_size() {
        assert_eq!(size_of::<Vector>(), 0x0c);
    }

    #[test]
    fn test_aabb_cycle_bytes() {
        let mut buf = vec![];
        buf.extend_from_slice(&3.0_f32.to_le_bytes());
        buf.extend_from_slice(&4.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        let vec = Vector::from_le_bytes(&buf).unwrap();
        assert_eq!(buf, vec.to_le_bytes());
    }
}
