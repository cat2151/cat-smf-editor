//! Independent preference: registering an editor never rewrites hand-edited TOML files.
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct Editor {
    pub(crate) executable: Option<PathBuf>,
    path: Option<PathBuf>,
    checked: bool,
    pending: Option<(PathBuf, u64)>,
}

impl Editor {
    pub(crate) fn at(directory: &Path) -> Self {
        Self {
            executable: None,
            path: Some(directory.join("external-editor.json")),
            checked: true,
            pending: None,
        }
    }

    pub(crate) fn open(directory: &Path) -> Result<Self, String> {
        let path = directory.join("external-editor.json");
        let executable = match std::fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        Ok(Self {
            executable,
            path: Some(path),
            checked: true,
            pending: None,
        })
    }

    /// Called on the first palette opening, never during application startup.
    pub(crate) fn load_once(&mut self, directory: Option<&Path>) -> Result<(), String> {
        if self.checked {
            return Ok(());
        }
        self.checked = true;
        let Some(directory) = directory else {
            return Ok(());
        };
        *self = Self::at(directory);
        *self = Self::open(directory)?;
        Ok(())
    }

    pub(crate) fn request(&mut self, target: PathBuf, frame: u64) {
        self.pending = Some((target, frame));
    }

    /// Native viewport focus commands from the closing palette must be applied first.
    pub(crate) fn launch_pending(&mut self, frame: u64) -> Result<(), String> {
        if self
            .pending
            .as_ref()
            .is_none_or(|(_, requested)| *requested == frame)
        {
            return Ok(());
        }
        let (target, _) = self.pending.take().unwrap();
        self.launch(&target)
    }

    pub(crate) fn save(&mut self, input: &str) -> Result<(), String> {
        let input = input.trim();
        let executable = if input.is_empty() {
            None
        } else {
            let path = PathBuf::from(input);
            if !path.is_absolute() || !path.is_file() {
                return Err("外部エディタの実行ファイルをフルパスで指定してください".into());
            }
            Some(path)
        };
        let path = self.path.as_ref().ok_or("LOCALAPPDATAを使えません")?;
        let directory = path.parent().ok_or("保存先がありません")?;
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, &executable).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        self.executable = executable;
        Ok(())
    }

    pub(crate) fn launch(&self, target: &Path) -> Result<(), String> {
        let executable = self.executable.as_ref().ok_or("外部エディタが未登録です")?;
        if !target.is_file() {
            return Err(format!("{}: ファイルがありません", target.display()));
        }
        #[cfg(windows)]
        unsafe {
            // An editor may forward the open request to an already running process.
            // Give that process permission too, before sending the request.
            AllowSetForegroundWindow(u32::MAX);
        }
        std::process::Command::new(executable)
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", executable.display()))
    }
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn AllowSetForegroundWindow(process: u32) -> i32;
}

#[cfg(test)]
mod tests;
