use super::*;
use crate::view::palette::tests::{app, frame, open, press, query};
use eframe::egui::{self, Key};

fn command(a: &mut App, c: &egui::Context, name: &str) {
    open(a, c);
    query(a, c, name);
    press(a, c, Key::Enter);
    frame(a, c, vec![]);
}

fn replace(a: &mut App, c: &egui::Context, text: &str) {
    frame(
        a,
        c,
        vec![egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL | egui::Modifiers::COMMAND,
        }],
    );
    frame(a, c, vec![egui::Event::Text(text.into())]);
}

#[test]
fn chord_keyboard_insert_edit_move_cancel_delete_and_undo() {
    let mut a = app();
    let c = egui::Context::default();
    command(&mut a, &c, "Add chord guide track");
    command(&mut a, &c, "Insert chord");
    frame(&mut a, &c, vec![egui::Event::Text("Isus4".into())]);
    press(&mut a, &c, Key::Enter);
    assert!(!crate::view::modal::is_open(&a));
    assert!(!a.playback.open);
    let chord = |a: &App| {
        chord_guide::chords(&a.loaded.as_ref().unwrap().music)
            .map(|e| (e.tick, chord_guide::chord(e).unwrap().to_owned()))
            .collect::<Vec<_>>()
    };
    assert_eq!(chord(&a), [(0, "Isus4".into())]);
    command(&mut a, &c, "Edit guide event");
    replace(&mut a, &c, "V7");
    press(&mut a, &c, Key::Escape);
    assert_eq!(chord(&a), [(0, "Isus4".into())]);
    command(&mut a, &c, "Edit guide event");
    replace(&mut a, &c, "I");
    press(&mut a, &c, Key::Enter);
    assert_eq!(chord(&a), [(0, "I".into())]);
    command(&mut a, &c, "Move guide event");
    replace(&mut a, &c, "1920");
    press(&mut a, &c, Key::Enter);
    assert_eq!(chord(&a), [(1920, "I".into())]);
    command(&mut a, &c, "Delete guide event");
    assert!(chord(&a).is_empty());
    a.undo_music().unwrap();
    assert_eq!(chord(&a), [(1920, "I".into())]);
    a.undo_music().unwrap();
    assert_eq!(chord(&a), [(0, "I".into())]);
    a.undo_music().unwrap();
    assert_eq!(chord(&a), [(0, "Isus4".into())]);
}

#[test]
fn guide_creation_key_edit_delete_undo_and_disk_startup_restore() {
    let mut a = app();
    let c = egui::Context::default();
    let original = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    command(&mut a, &c, "Add chord guide track");
    assert_eq!(a.track, 3);
    let music = &a.loaded.as_ref().unwrap().music;
    assert_eq!(chord_guide::track(music), Some(3));
    assert!(!music.tracks[3]
        .events
        .iter()
        .any(cat_smf_editor::score::note_view::note_on));
    a.edit_tracks(|music| cat_smf_editor::edit::tracks::rename(&mut music.tracks[3], "Harmony"))
        .unwrap();
    assert_eq!(
        chord_guide::track(&a.loaded.as_ref().unwrap().music),
        Some(3)
    );
    a.track_view.cursor_tick = Some(1920);
    super::super::insert_event::apply(&mut a, Kind::KeySignature, "G major").unwrap();
    assert_eq!(
        a.loaded
            .as_ref()
            .unwrap()
            .keys
            .map(1)
            .unwrap()
            .at(1920)
            .unwrap()
            .name(),
        "G major"
    );
    let index = a.selected_event.unwrap();
    apply(&mut a, index, Kind::KeySignature, "D minor", false).unwrap();
    assert_eq!(
        a.loaded
            .as_ref()
            .unwrap()
            .keys
            .map(1)
            .unwrap()
            .at(1920)
            .unwrap()
            .name(),
        "D minor"
    );
    a.note_mode = false;
    a.cursor_row = a.selected_event;
    delete(&mut a).unwrap();
    assert!(a
        .loaded
        .as_ref()
        .unwrap()
        .keys
        .map(1)
        .unwrap()
        .at(1920)
        .is_none());
    a.undo_music().unwrap();
    assert_eq!(
        a.loaded
            .as_ref()
            .unwrap()
            .keys
            .map(1)
            .unwrap()
            .at(1920)
            .unwrap()
            .name(),
        "D minor"
    );
    for (tick, text) in [(0, "Isus4"), (1920, "I")] {
        a.track_view.cursor_tick = Some(tick);
        super::super::insert_event::apply(&mut a, Kind::Chord, text).unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let path = work.path().join("guide.mid");
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    for deferred in [false, true] {
        let mut document = crate::store::saved::Document::default();
        let restored = document.load_for_startup(&path, deferred).unwrap();
        assert_eq!(cat_smf_editor::smf_io::encode(&restored).unwrap(), bytes);
        assert_eq!(chord_guide::track(&restored), Some(3));
        assert_eq!(
            chord_guide::chords(&restored)
                .map(|e| (e.tick, chord_guide::chord(e).unwrap()))
                .collect::<Vec<_>>(),
            [(0, "Isus4"), (1920, "I")]
        );
    }
    let status = work.path().join("status.json");
    let mut config = crate::store::config::Store::open(status.clone());
    config.navigation.last_mid = Some(path.clone());
    config.session = a.session();
    config.persist();
    for _ in 0..2 {
        let mut restarted = app();
        restarted.loaded = None;
        restarted.config = crate::store::config::Store::open(status.clone());
        restarted.open_initial(None);
        assert!(restarted.load_error.is_none(), "{:?}", restarted.load_error);
        assert_eq!(
            cat_smf_editor::smf_io::encode(&restarted.loaded.as_ref().unwrap().music).unwrap(),
            bytes
        );
        assert_eq!(
            restarted
                .loaded
                .as_ref()
                .unwrap()
                .keys
                .map(1)
                .unwrap()
                .at(1920)
                .unwrap()
                .name(),
            "D minor"
        );
    }
    while a.undo.available() {
        a.undo_music().unwrap();
    }
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
}
