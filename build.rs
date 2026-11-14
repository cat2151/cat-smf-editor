#[path = "../cat-plugin-player/shared/build.rs"]
mod shared_build;
fn main() {
    println!("cargo:rerun-if-changed=shim/project_import.cpp");
    println!("cargo:rerun-if-changed=shim/project_export.cpp");
    shared_build::build();
}
