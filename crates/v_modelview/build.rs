// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

const GENERATED_LICENSES_PLACEHOLDER: &str = "License information not generated.\nIf you plan to distribute this, you should look at tools/generate_licenses.py for license compliance.";

fn main() {
    if !std::fs::exists("../../generated_licenses.txt").unwrap() {
        std::fs::write(
            "../../generated_licenses.txt",
            GENERATED_LICENSES_PLACEHOLDER,
        )
        .unwrap();
    }
}
