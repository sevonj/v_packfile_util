// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2025 sevonj

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::VolitionError;
use crate::util::*;

/// 3D float vector
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Ok(Self {
            x: read_f32_le(buf, 0x0),
            y: read_f32_le(buf, 0x4),
            z: read_f32_le(buf, 0x8),
        })
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_vector_size() {
        assert_eq!(size_of::<Vector>(), 0x0c);
    }
}
