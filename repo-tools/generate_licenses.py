# SPDX-License-Identifier: CC0-1.0

# Dependency license text generator.
# This will use cargo-about to create generated_licenses.json,
# and then process it into a generated_licenses.txt.
#
# https://github.com/EmbarkStudios/cargo-about

import os
import subprocess
import json

ROOT_DIR = os.path.abspath(
    os.path.join(os.path.dirname(os.path.realpath(__file__)), "..")
)

if __name__ == "__main__":
    os.chdir(ROOT_DIR)
    subprocess.run(["cargo", "about", "generate", "--format=json", "-o=generated_licenses.json"])

    data = None
    with open("generated_licenses.json", "r") as f:
        data = json.load(f)

    with open("generated_licenses.txt", "w") as f:
        for license in data["licenses"]:
            f.write(f"\n{license["name"]}\n\nis used by:\n")

            for used_by in license["used_by"]:
                crate = used_by["crate"]
                f.write(f" - {crate["name"]} {crate["version"]}\n")

            f.write(f"\n{license["text"]}\n___\n")