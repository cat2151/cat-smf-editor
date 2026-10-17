#[path = "../cat-plugin-player/shared/build.rs"]
mod shared_build;
fn main() {
    shared_build::build();
}
