// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
    pub fn from_le_unsized(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Self::from_le_bytes(buf[..size_of::<Self>()].try_into().unwrap())
    }

    pub fn from_le_bytes(buf: &[u8; size_of::<Self>()]) -> Result<Self, VolitionError> {
        Ok(Self {
            min: Vector::from_le_unsized(buf)?,
            max: Vector::from_le_unsized(&buf[12..])?,
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

    pub const fn union(&self, other: &Self) -> Self {
        Self {
            min: Vector {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
                z: self.min.z.min(other.min.z),
            },
            max: Vector {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
                z: self.max.z.max(other.max.z),
            },
        }
    }

    pub fn center(&self) -> Vector {
        self.min + self.size() / 2.0
    }

    pub fn size(&self) -> Vector {
        self.max - self.min
    }

    pub fn radius(&self) -> f32 {
        (self.size() / 2.0).length()
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
        let bbox = AABB::from_le_unsized(&buf).unwrap();
        assert_eq!(buf, bbox.to_le_bytes());
    }
}
