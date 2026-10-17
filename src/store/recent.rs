//! Successfully opened files, separate from editor settings and document contents.
use cat_smf_editor::{model::Music, score::audition_timing::TempoMap};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    pub opened: u64,
    pub title: String,
    pub seconds: f64,
    pub tracks: usize,
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Default)]
pub struct History {
    pub entries: Vec<Entry>,
    path: Option<PathBuf>,
    pub error: Option<String>,
}

impl History {
    pub fn local() -> Self {
        match super::config::directory() {
            Some(root) => Self::open(root.join("recent-files.json")),
            None => Self {
                error: Some("Cannot save recent files: LOCALAPPDATA is unavailable".into()),
                ..Default::default()
            },
        }
    }

    pub fn open(path: PathBuf) -> Self {
        let mut history = Self {
            path: Some(path.clone()),
            ..Default::default()
        };
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Vec<Entry>>(&bytes) {
                Ok(entries) => history.entries = entries,
                Err(e) => history.error = Some(format!("{}: {e}", path.display())),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => history.error = Some(format!("{}: {e}", path.display())),
        }
        history
    }

    pub fn record(&mut self, path: &Path, music: &Music) {
        // Never replace unreadable history with an incomplete list.
        if self.error.is_some() {
            return;
        }
        let seconds = music
            .tracks
            .iter()
            .enumerate()
            .map(|(i, track)| {
                TempoMap::new(music, i).duration(0, track.events.last().map_or(0, |e| e.tick))
            })
            .fold(0.0, f64::max);
        self.entries.retain(|entry| entry.path != path);
        self.entries.insert(
            0,
            Entry {
                path: path.to_owned(),
                opened: now(),
                title: music
                    .tracks
                    .first()
                    .and_then(|t| t.name())
                    .unwrap_or_default(),
                seconds,
                tracks: music.tracks.len(),
            },
        );
        let Some(path) = &self.path else {
            return;
        };
        let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            use std::io::Write;
            std::fs::create_dir_all(path.parent().unwrap())?;
            let bytes = serde_json::to_vec_pretty(&self.entries)?;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        self.error = result.err().map(|e| format!("{}: {e}", path.display()));
    }
}
