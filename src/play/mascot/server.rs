//! Own only the server process launched by this editor. Never stop an external server.
use mascot_render_protocol::PlaybackRequest;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Server {
    child: Option<Child>,
    last_start: Option<Instant>,
}

impl Server {
    pub(super) fn sync(&mut self, request: &PlaybackRequest) -> Result<(), String> {
        let address = mascot_render_client::mascot_render_server_address();
        let result = mascot_render_client::sync_playback_at(address, request);
        if result.is_ok() || !request.playing {
            return result.map_err(|error| format!("{error:#}"));
        }
        self.ensure_started(
            || mascot_render_client::mascot_render_server_healthcheck_at(address).is_ok(),
            spawn,
        )?;
        // Readiness is retried with the latest state on the next worker wake.
        result.map_err(|error| format!("{error:#}"))
    }

    fn ensure_started(
        &mut self,
        healthy: impl FnOnce() -> bool,
        launch: impl FnOnce() -> Result<Child, String>,
    ) -> Result<(), String> {
        if let Some(child) = &mut self.child {
            match child.try_wait().map_err(|error| error.to_string())? {
                None => return Ok(()), // Includes startup: never launch a second process.
                Some(_) => self.child = None,
            }
        }
        if healthy()
            || self
                .last_start
                .is_some_and(|at| at.elapsed() < Duration::from_secs(5))
        {
            return Ok(());
        }
        self.last_start = Some(Instant::now());
        self.child = Some(launch()?);
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            if let Err(error) = child.kill()
                && !matches!(child.try_wait(), Ok(Some(_)))
            {
                crate::store::error_log::write("mascot", &format!("server shutdown: {error}"));
                return;
            }
            let _ = child.wait();
        }
    }
}

fn candidates(editor: &Path, search_path: Option<&std::ffi::OsStr>) -> Vec<PathBuf> {
    let name = format!("mascot-render-server{}", std::env::consts::EXE_SUFFIX);
    let mut paths = Vec::new();
    if let Some(directory) = editor.parent() {
        paths.push(directory.join(&name));
        if matches!(
            directory.file_name().and_then(|s| s.to_str()),
            Some("debug" | "release")
        ) && directory.parent().is_some_and(|p| p.ends_with("target"))
            && let Some(projects) = directory.ancestors().nth(3)
        {
            paths.push(
                projects
                    .join("mascot-render-server/target/release")
                    .join(&name),
            );
        }
    }
    if let Some(search_path) = search_path {
        paths.extend(std::env::split_paths(search_path).map(|directory| directory.join(&name)));
    }
    paths
}

fn spawn() -> Result<Child, String> {
    let editor = std::env::current_exe().map_err(|error| error.to_string())?;
    let paths = candidates(&editor, std::env::var_os("PATH").as_deref());
    let path = paths.iter().find(|p| p.is_file()).ok_or_else(|| {
        format!(
            "mascot-render-server executable not found: {}",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let mut command = Command::new(path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
        .spawn()
        .map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
