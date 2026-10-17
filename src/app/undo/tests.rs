use crate::{
    app::App,
    view::note_edit::tests::{chord_song, render},
};
use cat_smf_editor::{edit::note_length, smf_io};
use eframe::egui;

fn fixture() -> App {
    crate::view::tests::app(chord_song())
}

fn bytes(app: &App) -> Vec<u8> {
    smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
}

fn duration(app: &mut App, ticks: u64) {
    app.edit_music(|music| {
        let track = &mut music.tracks[1];
        let on = track
            .events
            .iter()
            .position(cat_smf_editor::score::note_view::note_on)
            .unwrap();
        note_length::set_duration(track, on, ticks)
    })
    .unwrap();
}

fn key(app: &mut App, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    render(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect(),
    );
}

#[test]
fn snapshots_restore_exact_events_and_noops_and_failures_do_not_record() {
    let mut app = fixture();
    let original = bytes(&app);
    duration(&mut app, 720);
    let first = bytes(&app);
    duration(&mut app, 600);
    app.edit_music(|_| Ok(())).unwrap();
    assert!(app
        .edit_music(|music| {
            music.tracks.clear();
            Err("failed edit".into())
        })
        .is_err());
    app.undo_music().unwrap();
    assert_eq!(bytes(&app), first);
    app.undo_music().unwrap();
    assert_eq!(bytes(&app), original);
    assert!(!app.undo.available());
    app.undo_music().unwrap();
    assert_eq!(bytes(&app), original);
}

#[test]
fn failed_undo_keeps_the_document_and_entry() {
    let mut app = fixture();
    duration(&mut app, 720);
    let changed = bytes(&app);
    // A snapshot for a different open document must not replace the current one.
    let path =
        std::env::temp_dir().join(format!("cat-smf-undo-failure-{}.mid", std::process::id()));
    std::fs::write(&path, &changed).unwrap();
    app.saved.load(&path).unwrap();
    assert!(app.undo_music().is_err());
    assert_eq!(bytes(&app), changed);
    assert!(app.undo.available());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn save_undo_and_file_switch_have_correct_history_and_dirty_state() {
    let mut app = fixture();
    let root = std::env::temp_dir().join(format!("cat-smf-undo-save-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("song.mid");
    std::fs::write(&path, bytes(&app)).unwrap();
    app.saved.directory = Some(root.clone());
    app.saved.load(&path).unwrap();
    app.loaded.as_mut().unwrap().original = path.clone();
    duration(&mut app, 720);
    assert!(app.saved.dirty());
    app.undo_music().unwrap();
    assert!(!app.saved.dirty());
    duration(&mut app, 600);
    app.saved.save().unwrap();
    assert!(!app.saved.dirty());
    app.undo_music().unwrap();
    assert!(app.saved.dirty());
    duration(&mut app, 480);
    app.load_mid_unchecked(root.join("missing.mid"));
    assert!(app.undo.available());
    app.load_mid_unchecked(path);
    assert!(!app.undo.available());
    assert!(!app.saved.dirty());
    assert!(root
        .canonicalize()
        .unwrap()
        .starts_with(std::env::temp_dir().canonicalize().unwrap()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sound_and_volume_changes_are_undone_without_losing_score_history() {
    let mut app = fixture();
    duration(&mut app, 720);
    let mut patch = app.loaded.as_ref().unwrap().patches[1].clone();
    patch.layers[0].settings.octave = 1;
    crate::player::selection::store_track(&mut app, 1, &patch).unwrap();
    assert!(app.undo.available());
    crate::play::mixer::volume::set(&mut app, 1, 0.5).unwrap();
    duration(&mut app, 600);
    app.undo_music().unwrap();
    assert_eq!(app.audition.gain(1), 0.5);
    assert_eq!(app.loaded.as_ref().unwrap().patches[1], patch);
    app.undo_music().unwrap();
    assert_eq!(app.audition.gain(1), 1.0);
    app.undo_music().unwrap();
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .octave,
        0
    );
    app.undo_music().unwrap();
    assert_eq!(bytes(&app), smf_io::encode(&chord_song()).unwrap());
    assert!(!app.undo.available());
}

#[test]
fn history_retains_only_the_latest_hundred_edits() {
    let mut app = fixture();
    duration(&mut app, 700);
    let oldest_retained = bytes(&app);
    for ticks in 500..600 {
        duration(&mut app, ticks);
    }
    for _ in 0..100 {
        app.undo_music().unwrap();
    }
    assert!(!app.undo.available());
    assert_eq!(bytes(&app), oldest_retained);
}

#[test]
fn keyboard_undo_works_in_score_and_roll_and_menu_and_respects_overlays() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    let original = bytes(&app);
    duration(&mut app, 720);
    app.help_open = true;
    key(&mut app, &ctx, egui::Key::Z, egui::Modifiers::CTRL);
    assert_ne!(bytes(&app), original);
    app.help_open = false;
    key(
        &mut app,
        &ctx,
        egui::Key::Z,
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
    );
    assert_ne!(bytes(&app), original);
    key(&mut app, &ctx, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(bytes(&app), original);
    duration(&mut app, 600);
    app.track = 1;
    key(&mut app, &ctx, egui::Key::R, egui::Modifiers::NONE);
    assert!(app.piano_roll.open);
    key(&mut app, &ctx, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(bytes(&app), original);
    duration(&mut app, 480);
    key(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    key(&mut app, &ctx, egui::Key::E, egui::Modifiers::NONE);
    assert!(app.edit_menu.open);
    key(&mut app, &ctx, egui::Key::U, egui::Modifiers::NONE);
    assert!(!app.edit_menu.open);
    assert_eq!(bytes(&app), original);
}
