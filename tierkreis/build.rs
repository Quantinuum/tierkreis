use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unset");
    let dist = Path::new(&manifest_dir).join("frontend").join("dist");
    std::fs::create_dir_all(&dist)
        .unwrap_or_else(|err| panic!("Failed to create {}: {err}", dist.display()));

    println!("cargo:rerun-if-changed=frontend/dist");
}
