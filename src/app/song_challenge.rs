use super::App;
use crate::store::{saved::Document, timestamp};
use cat_smf_editor::{edit::tracks, message};
use std::{
    io,
    path::{Path, PathBuf},
};

impl App {
    pub(crate) fn new_challenge_song(&mut self) -> Result<(), String> {
        let music = tracks::new_song();
        let sound = self.resolve_sound(&music)?;
        let mut document = Document::untitled(self.saved.directory.clone(), &music)?;
        let directory = document
            .directory
            .as_ref()
            .ok_or(message::BACKUP_DIRECTORY_IS_UNAVAILABLE)?
            .join("song-challenges");
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        let path = save_new(&mut document, &directory, &timestamp())?;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        self.install_resolved(path, name, music, document, sound);
        self.file_open_requested = false;
        self.song_challenge.ask_duration();
        Ok(())
    }
}

fn save_new(document: &mut Document, directory: &Path, timestamp: &str) -> Result<PathBuf, String> {
    for sequence in 1..=u64::MAX {
        let name = if sequence == 1 {
            format!("song-challenge-{timestamp}.mid")
        } else {
            format!("song-challenge-{timestamp}-{sequence}.mid")
        };
        let path = directory.join(name);
        match document.save_new(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(message::cannot_save(path.display(), error)),
        }
    }
    Err("チャレンジのファイル名を確保できません".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colliding_song_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("song-challenge-20261123123456.mid");
        std::fs::write(&original, b"existing song").unwrap();
        let mut document = Document::untitled(None, &tracks::new_song()).unwrap();
        let saved = save_new(&mut document, root.path(), "20261123123456").unwrap();
        assert_eq!(
            saved,
            root.path().join("song-challenge-20261123123456-2.mid")
        );
        assert_eq!(document.path(), Some(saved.as_path()));
        assert_eq!(std::fs::read(original).unwrap(), b"existing song");
    }
}
