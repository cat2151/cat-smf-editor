use super::*;
use crate::view::palette::tests::app;
use cat_smf_editor::{
    model::Event,
    score::{
        pattern,
        scene::arrangement::{self as grid, PatternOrScene},
    },
};
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
#[test]
fn midi_baseline_domains_and_compound_controls_are_explicit() {
    assert_eq!(midi_baseline("2 64 0").unwrap(), (1, Some(64), 0));
    assert_eq!(midi_baseline("16 bend 8192").unwrap(), (15, None, 8192));
    for invalid in [
        "0 64 0",
        "17 bend 8192",
        "1 7 128",
        "1 bend 16384",
        "1 0 1",
        "1 32 1",
        "1 6 1",
        "1 101 0",
        "1 123 0",
        "1 64",
    ] {
        assert!(midi_baseline(invalid).is_err(), "{invalid}");
    }
}
#[test]
fn container_midi_baseline_palette_is_atomic_keyboard_input_and_undoable() {
    use crate::view::patterns::workspace_tests::{command, key, text, verification_song};
    let (mut a, c, id, _, _) = verification_song();
    let root = egui::ViewportId::ROOT;
    let before = bytes(&a);
    command(&mut a, &c, root, "Container MIDI baseline");
    text(&mut a, &c, root, "Pattern 1");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    text(&mut a, &c, root, "2 64 0");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(!crate::view::modal::is_open(&a));
    let m = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(m).unwrap();
    assert!(
        pattern::automation::baselines(m, catalog.patterns[&id].track)
            .unwrap()
            .iter()
            .any(|b| matches!(
                b,
                pattern::automation::Baseline::Midi {
                    channel: 1,
                    controller: Some(64),
                    value: 0
                }
            ))
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}
#[test]
fn pattern_library_duplicate_opens_an_independent_copy_and_undoes() {
    use crate::view::patterns::workspace_tests::{command, key, text, verification_song};
    let (mut a, c, first, _, _) = verification_song();
    let root = egui::ViewportId::ROOT;
    let before = bytes(&a);
    command(&mut a, &c, root, "duplicate pattern");
    text(&mut a, &c, root, "Pattern 1");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert_ne!(a.patterns.selected.unwrap(), first);
    assert!(a.patterns.editing.as_ref().unwrap().standalone);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}
#[test]
fn pattern_sound_picker_cancel_and_stale_identity_do_not_mutate_music() {
    use crate::view::patterns::workspace_tests::{command, key, text, verification_song};
    let (mut a, c, _, _, _) = verification_song();
    let root = egui::ViewportId::ROOT;
    let before = bytes(&a);
    command(&mut a, &c, root, "move pattern to container");
    text(&mut a, &c, root, "nihongo 日本語");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert!(!crate::view::modal::is_open(&a));
    command(&mut a, &c, root, "move pattern to container");
    a.song_id += 1;
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(!crate::view::modal::is_open(&a));
    assert_eq!(bytes(&a), before);
}
#[test]
fn pattern_source_is_silent_in_full_partial_repeat_and_track_delete_is_atomic() {
    let mut music = app().loaded.unwrap().music;
    let source = pattern::operations::create_source(&mut music).unwrap();
    let id = pattern::operations::create(&mut music, source, None).unwrap();
    for (tick, status) in [(120, 0x90), (600, 0x80)] {
        cat_smf_editor::edit::insert(
            &mut music.tracks[source],
            Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![71, 90],
            },
        );
    }
    let mut placed = music.clone();
    let lane = grid::add_lane(&mut placed, None).unwrap();
    grid::place(
        &mut placed,
        None,
        lane,
        2,
        Some(PatternOrScene::Pattern(id)),
    )
    .unwrap();
    let mut a = crate::view::tests::app(music);
    a.track_view.cursor_tick = Some(0);
    for mode in [
        crate::play::playback::Mode::AllTracks,
        crate::play::playback::Mode::AllTracksFromMeasure,
    ] {
        let p = crate::play::transport::prepare(&a, mode).unwrap();
        assert!(!p.parts.iter().any(|p| p.0 == source));
    }
    crate::play::transport::repeat::start(&mut a, (0, 1920));
    assert!(!a
        .playback
        .pending
        .as_ref()
        .unwrap()
        .parts
        .iter()
        .any(|p| p.0 == source));
    // The Song still uses the pattern, so its container cannot go.
    let mut a = crate::view::tests::app(placed);
    let before = bytes(&a);
    assert!(a
        .edit_tracks(|m| {
            m.tracks.remove(source);
            Ok(())
        })
        .is_err());
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
    a.edit_tracks(|m| {
        m.tracks.remove(1);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns[&id]
            .track,
        source - 1
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}

#[test]
fn structure_tempo_and_song_length_inputs_are_keyboard_owned_and_atomic() {
    use crate::view::patterns::workspace_tests::{command, key, text, verification_song};
    use cat_smf_editor::score::pattern::Catalog;
    let (mut a, c, _, _, _) = verification_song();
    let root = egui::ViewportId::ROOT;
    crate::view::patterns::library::create(&mut a, None).unwrap();
    let id = a.patterns.selected.unwrap();
    let name = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns[&id]
        .name
        .clone();
    command(&mut a, &c, root, "Pattern measures / meter");
    text(&mut a, &c, root, &name);
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    key(&mut a, &c, root, Key::A, Modifiers::CTRL);
    text(&mut a, &c, root, "4/4 3/4");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(!crate::view::modal::is_open(&a));
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns[&id]
            .length()
            .unwrap(),
        3360
    );
    let before = bytes(&a);
    command(&mut a, &c, root, "Audition tempo");
    key(&mut a, &c, root, Key::A, Modifiers::CTRL);
    text(&mut a, &c, root, "90");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert_eq!(a.patterns.tempo.0, 90.0);
    assert_eq!(bytes(&a), before);
    let before = bytes(&a);
    command(&mut a, &c, root, "Scene measures / meter");
    key(&mut a, &c, root, Key::A, Modifiers::CTRL);
    text(&mut a, &c, root, "4/4 4/4 4/4");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(a.patterns.workspace.input_open());
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
}
#[test]
fn pattern_song_without_patterns_starts_the_first_pattern_in_mml_input() {
    use crate::view::palette::tests::{self as palette, frame, press};
    let mut a = app();
    a.edit_tracks(|m| pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let c = egui::Context::default();
    palette::open(&mut a, &c);
    palette::query(&mut a, &c, "pattern");
    assert_eq!(palette::listed(&mut a)[0], "Event list: Pattern / Song");
    press(&mut a, &c, Key::Enter);
    assert!(a.patterns.confirming_creation());
    press(&mut a, &c, Key::Enter);
    for _ in 0..3 {
        frame(&mut a, &c, vec![]);
    }
    assert!(a.mml_input.open, "{:?}", a.patterns.error);
    let id = a.patterns.selected.unwrap();
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    press(&mut a, &c, Key::Escape);
    let before = bytes(&a);
    palette::open(&mut a, &c);
    palette::query(&mut a, &c, "pattern");
    press(&mut a, &c, Key::Enter);
    frame(&mut a, &c, vec![]);
    assert!(!a.patterns.confirming_creation());
    assert!(!a.mml_input.open);
    assert!(crate::view::patterns::mode::workspace_open(&a));
    assert_eq!(bytes(&a), before);
}
