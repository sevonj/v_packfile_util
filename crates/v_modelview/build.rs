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
