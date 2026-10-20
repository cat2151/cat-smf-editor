use std::{collections::BTreeMap, io::Write, path::Path};

fn append(directory: &Path, category: &str, message: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("error.log"))?;
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    writeln!(
        file,
        "[{}.{:03} UTC unix] [{category}] {message}",
        time.as_secs(),
        time.subsec_millis()
    )
}

pub fn write(category: &str, message: &str) {
    if let Some(directory) = crate::store::config::directory() {
        if let Err(error) = append(&directory, category, message) {
            eprintln!(
                "Cannot write {}: {error}",
                directory.join("error.log").display()
            );
        }
    } else {
        eprintln!("Cannot write error.log: LOCALAPPDATA is unavailable");
    }
}

#[derive(Default)]
pub struct Log {
    active: BTreeMap<&'static str, String>,
}

impl Log {
    fn observe(&mut self, category: &'static str, error: Option<&str>) {
        if let Some(error) = error {
            if self.active.get(category).map(String::as_str) != Some(error) {
                write(category, error);
                self.active.insert(category, error.to_owned());
            }
        } else {
            self.active.remove(category);
        }
    }
}

impl crate::app::App {
    pub fn log_errors(&mut self) {
        for (category, error) in [
            ("settings", self.settings_error.as_deref()),
            ("session", self.config.error.as_deref()),
            ("load", self.load_error.as_deref()),
            ("player", self.player_selection.error.as_deref()),
            ("audio", self.audition.error()),
            ("save", self.saved.error.as_deref()),
            ("voicevox", self.voicevox.error.as_deref()),
        ] {
            self.error_log.observe(category, error);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn errors_are_appended_without_truncating_previous_entries() {
        let root = std::env::temp_dir().join(format!("cat-smf-log-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("error.log"), "previous\n").unwrap();
        super::append(&root, "load", "X:/one/song.mid: invalid SMF").unwrap();
        super::append(&root, "save", "disk full").unwrap();
        let log = std::fs::read_to_string(root.join("error.log")).unwrap();
        assert!(log.starts_with("previous\n"));
        assert!(log.contains("[load] X:/one/song.mid: invalid SMF"));
        assert!(log.contains("[save] disk full"));
        std::fs::remove_file(root.join("error.log")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
