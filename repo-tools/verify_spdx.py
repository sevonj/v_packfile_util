# SPDX-License-Identifier: CC0-1.0

# CI check

import os

SRC_DIR = os.path.abspath(
    os.path.join(os.path.dirname(os.path.realpath(__file__)), "..", "crates")
)
LICENSE_ID_STR = "// SPDX-License-Identifier: MPL-2.0\n"
LICENSE_ID_LEN = len(LICENSE_ID_STR)
LICENSE_NOTICE_STR = """
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
"""

if __name__ == "__main__":
    print("Verify SPDX ids")
    num_failed = 0
    for root, _, files in os.walk(SRC_DIR):
        for file in files:
            if file.endswith(".rs") or file.endswith(".wgsl"):
                file_path = os.path.join(root, file)
                with open(file_path, "r") as f:
                    starts_with = f.read(LICENSE_ID_LEN)
                    failed = False
                    if starts_with != LICENSE_ID_STR:
                        failed = True
                        print(
                            f"Missing SPDX ID in ile: {file_path}"
                        )
                    if LICENSE_NOTICE_STR not in f.read():
                        failed = True
                        print(
                            f"Missing license notice in ile: {file_path}"
                        )
                    if failed:
                        num_failed += 1
    print(f"num_failed: {num_failed}")
    exit(num_failed)