//! Normal save/load/startup paths, including saving during projected note editing.
use crate::store::{config::Store, saved::Document};
use cat_smf_editor::{
    edit,
    score::{
        note_view::note_on,
        pattern::{body, Catalog},
        same_measure,
    },
};

#[test]
fn pattern_saved_document_and_two_startups_keep_definitions_not_expanded_notes() {
    let root = tempfile::tempdir().unwrap();
    let mid = root.path().join("patterns.mid");
    let (mut app, source, id) = crate::view::patterns::edit_tests::fixture();
    app.config = Store::open(root.path().join("status.json"));
    app.edit_tracks(|music| {
        same_measure::insert(music, 0, 1, 1, 4)?;
        Ok(())
    })
    .unwrap();
    crate::view::patterns::library::edit(&mut app, id).unwrap();
    app.saved = Document::untitled(
        Some(root.path().to_owned()),
        &app.loaded.as_ref().unwrap().music,
    )
    .unwrap();
    app.saved.save_as(&mid).unwrap();
    app.sync_saved_path();
    let original = std::fs::read(&mid).unwrap();
    let on = app.loaded.as_ref().unwrap().visible().tracks[source]
        .events
        .iter()
        .position(|e| note_on(e) && e.tick == 120)
        .unwrap();
    app.edit_music(|music| edit::note_value::set_pitch(&mut music.tracks[source], on, 72))
        .unwrap();
    let expected = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    assert_ne!(expected, original);
    assert!(app.loaded.as_ref().unwrap().display_music.is_some());
    app.saved.save_confirmed().unwrap();
    app.save_session();
    app.config.persist();
    assert_eq!(app.session().unwrap().mbt.as_deref(), Some("1:1:0"));
    assert_eq!(
        Store::open(root.path().join("status.json"))
            .session
            .unwrap()
            .mbt
            .as_deref(),
        Some("1:1:0")
    );
    assert_eq!(std::fs::read(&mid).unwrap(), expected);
    let backups = std::fs::read_dir(root.path().join("backup"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    assert_eq!(std::fs::read(&backups[0]).unwrap(), original);
    let loaded = Document::new(None).load(&mid).unwrap();
    assert_eq!(
        loaded.tracks[0]
            .events
            .iter()
            .filter(|e| note_on(e))
            .count(),
        1
    );
    let catalog = Catalog::read(&loaded).unwrap();
    assert_eq!(catalog.patterns.len(), 2);
    let p = &catalog.patterns[&id];
    assert!(catalog.sources.contains(&catalog.tracks[&p.container]));
    assert_eq!(
        body::extract(&loaded.tracks[p.track].events, p.range().unwrap())
            .unwrap()
            .iter()
            .filter(|e| note_on(e))
            .map(|e| e.data[0])
            .collect::<Vec<_>>(),
        vec![72]
    );
    assert_eq!(
        loaded.tracks[0]
            .events
            .iter()
            .filter_map(|e| same_measure::reference(e).unwrap())
            .collect::<Vec<_>>(),
        vec![1]
    );
    for _ in 0..2 {
        let mut restored = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        restored.loaded = None;
        restored.config = Store::open(root.path().join("status.json"));
        restored.saved = Document::new(Some(root.path().to_owned()));
        restored.open_initial(None);
        assert!(restored.load_error.is_none(), "{:?}", restored.load_error);
        assert_eq!(
            cat_smf_editor::smf_io::encode(&restored.loaded.as_ref().unwrap().music).unwrap(),
            expected
        );
        assert!(restored.patterns.editing.is_none());
        assert_eq!(
            crate::play::playback::location(&restored)
                .1
                .unwrap()
                .measure,
            1
        );
        let prepared =
            crate::play::transport::prepare(&restored, crate::play::playback::Mode::AllTracks)
                .unwrap();
        assert_eq!(prepared.parts.len(), 1);
        assert_eq!(
            prepared.parts[0]
                .2
                .notes
                .iter()
                .map(|n| n.pitch)
                .collect::<Vec<_>>(),
            vec![60, 60]
        );
        assert_eq!(prepared.parts[0].2.end, 8.0);
        // The normal app saves the restored position at the end of its update/exit.
        restored.save_session();
        assert_eq!(
            Store::open(root.path().join("status.json"))
                .session
                .unwrap()
                .mbt
                .as_deref(),
            Some("1:1:0")
        );
    }
    assert_eq!(std::fs::read(&mid).unwrap(), expected);
}

#[test]
#[ignore = "opt-in external acceptance fixture via CAT_SMF_ACCEPTANCE_FIXTURE; no GUI/audio"]
fn acceptance_fixture_loads_real_tracks_and_prepares_only_song_placements() {
    let path = std::path::PathBuf::from(std::env::var_os("CAT_SMF_ACCEPTANCE_FIXTURE").unwrap());
    let before = std::fs::read(&path).unwrap();
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    app.loaded = None;
    app.open_initial(Some(path.clone()));
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let music = &app.loaded.as_ref().unwrap().music;
    same_measure::validate(music).unwrap();
    let c = Catalog::read(music).unwrap();
    assert_eq!(c.sources.iter().copied().collect::<Vec<_>>(), vec![2]);
    assert_eq!(music.tracks[2].name().as_deref(), Some("Patterns"));
    assert_eq!(c.patterns.len(), 3);
    let sequence =
        crate::play::transport::prepare(&app, crate::play::playback::Mode::AllTracks).unwrap();
    assert!(sequence
        .parts
        .iter()
        .all(|(track, _, _)| !c.sources.contains(track)));
    let melody = &sequence.parts.iter().find(|part| part.0 == 1).unwrap().2;
    assert_eq!(
        melody.notes.iter().map(|n| n.pitch).collect::<Vec<_>>(),
        vec![67, 60]
    );
    assert!(sequence
        .parts
        .iter()
        .filter(|part| part.0 != 1)
        .all(|part| part.2.notes.is_empty()));
    assert_eq!(std::fs::read(path).unwrap(), before);
}
