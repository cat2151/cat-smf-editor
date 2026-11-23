use super::App;
use crate::store::{saved::Document, timestamp};
use cat_smf_editor::{
    edit::{self, tracks},
    message,
    model::Music,
    score::{
        drum_kit, instrument,
        note_view::{note_off, note_on},
        pattern::operations,
        scene::arrangement::{self, PatternOrScene},
    },
};
use std::{
    io,
    path::{Path, PathBuf},
};

impl App {
    pub(crate) fn new_challenge_song(&mut self) -> Result<(), String> {
        let (music, containers) = starter_song(self)?;
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
        crate::view::patterns::workspace::enable(self);
        self.song_challenge.ask_duration(containers);
        Ok(())
    }
}

fn starter_song(app: &mut App) -> Result<(Music, [usize; 4]), String> {
    let patch = crate::player::default_kit::load(
        &mut app.state_cache,
        app.player_selection.executable.clone(),
    )
    .map_err(|error| format!("既定のドラムキットを読み込めません: {error}"))?;
    let kick = patch
        .kit
        .as_ref()
        .and_then(drum_kit::kick_pitch)
        .ok_or("既定のドラムキットにキックが見つかりません")?;
    let mut music = tracks::new_song();
    // The regular new file's example note is replaced by the arranged drum pattern.
    music.tracks[0]
        .events
        .retain(|e| !note_on(e) && !note_off(e));
    edit::refresh_deltas(&mut music.tracks[0])?;
    let mut containers = [0; 4];
    for (track, name) in containers
        .iter_mut()
        .zip(["Drum kit", "Bass", "Chord", "Lead"])
    {
        *track = operations::create_source(&mut music)?;
        tracks::rename(&mut music.tracks[*track], name)?;
    }
    let drum = containers[0];
    instrument::write(
        &mut music.tracks[drum],
        &patch,
        &mut app
            .patch_roots
            .lookup(app.player_selection.executable.clone()),
    )?;
    let pattern = operations::create(&mut music, drum, None)?;
    drum_kit::toggle(&mut music, drum, kick, 1, 0)?;
    edit::regenerate(&mut music)?;
    let lane = arrangement::add_lane(&mut music, None)?;
    arrangement::place(
        &mut music,
        None,
        lane,
        1,
        Some(PatternOrScene::Pattern(pattern)),
    )?;
    Ok((music, containers))
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
    #[ignore = "requires RESIDENT_PLAYER and installed default kit; no GUI or device output"]
    fn installed_default_kit_survives_reopening_the_challenge() {
        let directory = tempfile::tempdir().unwrap();
        let player = PathBuf::from(std::env::var_os("RESIDENT_PLAYER").unwrap());
        let output = crate::player::state_request::headless(Some(player.clone()), "--patch-roots")
            .unwrap()
            .output()
            .unwrap();
        assert!(output.status.success());
        let _roots = crate::player::patch_roots::Assumed::with_table(
            serde_json::from_slice(&output.stdout).unwrap(),
        );
        let make_app = || {
            let mut app = crate::view::tests::app(tracks::new_song());
            app.player_selection.executable = Some(player.clone());
            app.saved.directory = Some(directory.path().to_owned());
            app.state_cache =
                cat_smf_editor::score::state_cache::Cache::new(app.saved.directory.clone());
            app
        };
        let mut app = make_app();
        app.new_challenge_song().unwrap();
        let loaded = app.loaded.as_ref().unwrap();
        let patch = &loaded.patches[1];
        assert!(patch.label().ends_with("/DSF/Mind Game.sfz"));
        assert!(loaded.music.tracks[1]
            .events
            .iter()
            .any(|e| { note_on(e) && e.tick == 0 && e.data[0] == drum_kit::GM_KICK }));
        app.state_cache.save().unwrap();
        let mut reopened = make_app();
        reopened.load_mid_unchecked(loaded.original.clone());
        assert!(reopened.load_error.is_none(), "{:?}", reopened.load_error);
        assert_eq!(&reopened.loaded.as_ref().unwrap().patches[1], patch);

        let _library = cat_smf_editor::native_library::load_at(directory.path()).unwrap();
        let prepared = crate::play::arrangement::prepare(&reopened, None).unwrap();
        let parts = prepared.parts;
        let instruments =
            crate::play::audition::offline::Instruments::open(48_000, &parts, &Default::default())
                .unwrap();
        let gains = vec![1.0; loaded.music.tracks.len()];
        let job = unsafe { instruments.job(48_000, parts, &gains, true, vec![]) };
        let audio = job
            .run(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!(audio.iter().all(|v| v.is_finite()));
        assert!(audio.iter().any(|v| v.abs() > 0.00001), "kick is silent");
    }

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
