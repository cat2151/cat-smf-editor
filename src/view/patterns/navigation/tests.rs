use super::*;
use crate::view::{
    commands::{self, Screen},
    drums,
    palette::tests::press,
};
use cat_smf_editor::score::{
    instrument::{Patch, ResolvedLayer},
    pattern::operations,
    scene::arrangement::{self, PatternOrScene},
};
use eframe::egui::{self, Key};

fn fixture() -> (App, usize, [u64; 2]) {
    let (a, track, first) = super::super::edit_tests::fixture();
    let mut music = a.loaded.unwrap().music;
    let patch = Patch {
        drums: Some(Default::default()),
        layers: (1..=4)
            .map(|id| ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        ..Default::default()
    };
    cat_smf_editor::score::instrument::write(
        &mut music.tracks[track],
        &patch,
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    let second = Catalog::read(&music)
        .unwrap()
        .patterns
        .keys()
        .copied()
        .find(|&id| id != first)
        .unwrap();
    (crate::view::tests::app(music), track, [first, second])
}

fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn assert_body(a: &App, c: &egui::Context, id: u64) {
    assert!(drums::is_open(a), "{:?}", a.load_error);
    assert_eq!(mode::editing(a, a.track).unwrap().pattern, id);
    assert_eq!(commands::owner(a, c), Screen::Drums);
    assert!(!a.mml_input.open && !a.piano_roll.open);
    assert!(a.playback.pending.is_none() && a.playback.active.is_none());
    assert!(a.playback.repeat.is_none());
}

#[test]
fn every_body_entry_resolves_the_same_editor_without_writes_or_playback() {
    for entry in 0..7 {
        let (mut a, track, ids) = fixture();
        let c = egui::Context::default();
        a.edit_tracks(|m| {
            let lane = arrangement::add_lane(m, None)?;
            arrangement::place(m, None, lane, 1, Some(PatternOrScene::Pattern(ids[0])))
        })
        .unwrap();
        let before = bytes(&a);
        let p = workspace::Pane::Song;
        a.patterns.workspace.song_cell.choice = Some(PatternOrScene::Pattern(ids[0]));
        match entry {
            0 => a.select_track(track),
            1 => crate::cursor::split::go(&mut a, track, 120),
            2 => library::edit(&mut a, ids[0]).unwrap(),
            3 => workspace::edit_choice(&mut a, &c, p),
            4 => workspace::edit(&mut a, &c, p),
            5 => workspace::click_body(&mut a, p),
            _ => {
                let mut session = a.session().unwrap();
                session.track = track + 1;
                session.note_mode = true;
                a.restore_position(&session);
            }
        }
        assert_body(&a, &c, ids[0]);
        assert_eq!(bytes(&a), before, "entry {entry} changed the SMF");
    }
}

#[test]
fn track_return_remembers_each_containers_last_body_and_deleted_ids_fall_back() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    a.select_track(track);
    assert_body(&a, &c, ids[0]);
    library::edit(&mut a, ids[1]).unwrap();
    a.select_track(0);
    a.select_track(track);
    assert_body(&a, &c, ids[1]);
    a.edit_tracks(|m| operations::delete(m, ids[1])).unwrap();
    assert!(!drums::is_open(&a));
    assert!(a.patterns.editing.is_none());
    a.select_track(track);
    assert_body(&a, &c, ids[0]);
}

#[test]
fn escape_returns_to_library_and_stays_closed_until_explicit_reselection() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    a.select_track(track);
    let before = bytes(&a);
    press(&mut a, &c, Key::Escape);
    assert!(!drums::is_open(&a));
    assert!(a.patterns.editing.is_none());
    assert!(mode::workspace_open(&a));
    assert_eq!(
        a.patterns.workspace.song_cell.choice,
        Some(PatternOrScene::Pattern(ids[0]))
    );
    refresh(&mut a);
    super::super::validate(&mut a);
    assert!(!drums::is_open(&a));
    a.select_track(track);
    assert_body(&a, &c, ids[0]);
    drums::dismiss(&mut a); // Native close button uses the same transition.
    assert!(!drums::is_open(&a));
    library::edit(&mut a, ids[0]).unwrap();
    assert_body(&a, &c, ids[0]);
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn pattern_switch_drops_numeric_input_and_an_old_command_cannot_edit_the_new_body() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    a.select_track(track);
    press(&mut a, &c, Key::ArrowLeft);
    press(&mut a, &c, Key::Enter);
    assert_eq!(commands::owner(&a, &c), Screen::DrumInput);
    let old = commands::definitions(&a, Screen::DrumInput)
        .into_iter()
        .find(|c| c.id == "drum-confirm")
        .unwrap();
    library::edit(&mut a, ids[1]).unwrap();
    assert_body(&a, &c, ids[1]);
    let before = bytes(&a);
    assert!(!commands::execute(&mut a, &c, &old));
    assert_eq!(bytes(&a), before);
}

#[test]
fn manual_loop_survives_edits_and_undo_but_not_target_changes() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    a.select_track(track);
    assert_body(&a, &c, ids[0]);
    press(&mut a, &c, Key::R);
    assert!(a.playback.repeat.is_some());
    press(&mut a, &c, Key::Enter);
    assert!(a.playback.repeat.is_some());
    a.undo_music().unwrap();
    assert!(drums::is_open(&a));
    assert_eq!(mode::editing(&a, track).unwrap().pattern, ids[0]);
    library::edit(&mut a, ids[1]).unwrap();
    assert_body(&a, &c, ids[1]);
}

#[test]
fn raw_and_restart_restore_valid_targets_without_persisting_last_pattern() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    library::edit(&mut a, ids[1]).unwrap();
    mode::toggle(&mut a);
    assert!(!drums::is_open(&a));
    let raw = a.session().unwrap();
    mode::toggle(&mut a);
    assert_body(&a, &c, ids[1]);
    let notes = a.session().unwrap();
    let music = a.loaded.as_ref().unwrap().music.clone();
    let mut restarted = crate::view::tests::app(music);
    restarted.restore_position(&raw);
    assert!(!drums::is_open(&restarted));
    restarted.restore_position(&notes);
    assert_body(&restarted, &c, ids[0]);
    assert_eq!(restarted.track, track);

    // Opening a track in Raw has no prior edit context to restore.
    let mut raw_entry = crate::view::tests::app(a.loaded.as_ref().unwrap().music.clone());
    raw_entry.note_mode = false;
    raw_entry.select_track(track);
    assert!(!drums::is_open(&raw_entry));
    press(&mut raw_entry, &c, Key::V);
    assert_body(&raw_entry, &c, ids[0]);
}

#[test]
fn missing_destination_is_atomic_and_empty_container_navigation_does_not_create() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    a.select_track(track);
    let before = bytes(&a);
    assert!(library::edit(&mut a, u64::MAX).is_err());
    assert_body(&a, &c, ids[0]);
    assert_eq!(bytes(&a), before);
    a.edit_tracks(|m| {
        operations::delete(m, ids[0])?;
        operations::delete(m, ids[1])
    })
    .unwrap();
    let before = bytes(&a);
    a.select_track(0);
    a.select_track(track);
    assert!(!drums::is_open(&a));
    assert!(mode::workspace_open(&a));
    assert_eq!(bytes(&a), before);
}

#[test]
fn keyboard_track_navigation_raw_and_reentry_keep_the_body_and_key_owner_in_sync() {
    let (mut a, track, ids) = fixture();
    let c = super::super::workspace_tests::context();
    let root = egui::ViewportId::ROOT;
    let key = |a: &mut App, key, modifiers| {
        super::super::workspace_tests::key(a, &c, root, key, modifiers);
    };
    key(&mut a, Key::ArrowRight, egui::Modifiers::CTRL);
    assert_eq!(a.track, track);
    assert_body(&a, &c, ids[0]);
    key(&mut a, Key::V, egui::Modifiers::NONE);
    assert!(!a.note_mode && !drums::is_open(&a));
    key(&mut a, Key::V, egui::Modifiers::NONE);
    assert_body(&a, &c, ids[0]);
    key(&mut a, Key::ArrowLeft, egui::Modifiers::CTRL);
    assert_eq!(a.track, 0);
    assert!(!drums::is_open(&a));
    key(&mut a, Key::ArrowRight, egui::Modifiers::CTRL);
    assert_body(&a, &c, ids[0]);
    key(&mut a, Key::Escape, egui::Modifiers::NONE);
    assert!(!drums::is_open(&a));
    assert!(a.patterns.editing.is_none());
    let song = workspace::Pane::Song;
    a.patterns.workspace.selection_mut(song).choice = Some(PatternOrScene::Pattern(ids[0]));
    super::super::workspace_tests::key(&mut a, &c, song.viewport(), Key::E, egui::Modifiers::SHIFT);
    assert_body(&a, &c, ids[0]);
}

#[test]
fn track_insertion_and_undo_rebind_the_visible_editor_by_pattern_identity() {
    let (mut a, track, ids) = fixture();
    let c = egui::Context::default();
    library::edit(&mut a, ids[1]).unwrap();
    a.edit_tracks(|m| {
        m.tracks
            .insert(track, cat_smf_editor::model::Track { events: vec![] });
        Ok(())
    })
    .unwrap();
    assert_eq!(a.track, track + 1);
    assert_body(&a, &c, ids[1]);
    a.undo_music().unwrap();
    assert_eq!(a.track, track);
    assert_body(&a, &c, ids[1]);
}
