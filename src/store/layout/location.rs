//! Source checkout builds edit one file; distributed executables use a manually placed file.
use std::path::{Path, PathBuf};

pub(super) fn select(executable: &Path, project: &Path, local: &Path) -> PathBuf {
    if executable.starts_with(project.join("target")) {
        project.join("config/layout.toml")
    } else {
        local.join("layout.toml")
    }
}

pub(crate) fn path(local: &Path) -> PathBuf {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project = project.canonicalize().unwrap_or(project);
    match std::env::current_exe() {
        Ok(executable) => {
            let executable = executable.canonicalize().unwrap_or(executable);
            select(&executable, &project, local)
        }
        Err(_) => local.join("layout.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_release_in_the_checkout_use_source_independent_of_working_directory() {
        let project = Path::new("X:/project");
        let local = Path::new("X:/local");
        for profile in ["debug", "release"] {
            assert_eq!(
                select(
                    &project.join(format!("target/{profile}/app.exe")),
                    project,
                    local
                ),
                project.join("config/layout.toml")
            );
        }
        assert_eq!(
            select(Path::new("X:/distribution/app.exe"), project, local),
            local.join("layout.toml")
        );
        assert_eq!(
            select(Path::new("X:/project/target-other/app.exe"), project, local),
            local.join("layout.toml")
        );
    }
}
