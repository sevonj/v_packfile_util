// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::PathBuf;

use v_types::Packfile;
use v_types::VolitionError;

pub fn unpack(input_file: PathBuf, output_dir: Option<PathBuf>) -> Result<(), VolitionError> {
    let buf = std::fs::read(&input_file).unwrap();

    let out_dir = output_dir.unwrap_or_else(|| {
        let stem = input_file.file_stem().unwrap().to_str().unwrap();
        input_file
            .parent()
            .unwrap()
            .join(format!("{stem}_extracted"))
    });

    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    let packfile = Packfile::from_data(&buf).unwrap();

    let entries = packfile.read_entries(&buf).unwrap();
    let filenames = packfile.read_filenames(&buf, &entries).unwrap();

    assert_eq!(filenames.len(), entries.len());

    for (entry, filename) in entries.iter().zip(filenames) {
        std::fs::write(out_dir.join(filename), packfile.entry_data(&buf, entry)?).unwrap();
    }

    Ok(())
}
