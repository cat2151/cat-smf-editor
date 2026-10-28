use super::*;
use crate::view::palette::tests::{frame, press};
use cat_smf_editor::{
    edit,
    score::{
        note_view::note_on,
        pattern::Catalog,
        scene::arrangement::{self as grid, PatternOrScene},
    },
};
use eframe::egui::{self, Key, Modifiers};
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn raw_note(a: &App) -> usize {
    a.loaded.as_ref().unwrap().music.tracks[a.track]
        .events
        .iter()
        .position(note_on)
        .unwrap()
}
#[test]
fn raw_source_real_note_edit_undo_and_notes_return_share_body_without_projection() {
    let (mut a, source, id) = edit_tests::fixture();
    a.edit_tracks(|m| {
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 2, Some(PatternOrScene::Pattern(id)))?;
        grid::place(m, None, lane, 3, Some(PatternOrScene::Pattern(id)))?;
        cat_smf_editor::score::scene::sizing::song(m, 3)
    })
    .unwrap();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let original = bytes(&a);
    let index = a.loaded.as_ref().unwrap().visible().tracks[source]
        .events
        .iter()
        .position(note_on)
        .unwrap();
    let row = a.loaded.as_ref().unwrap().display_rows[source]
        .event_row(index)
        .unwrap();
    crate::cursor::moves::place(&mut a, row);
    press(&mut a, &c, Key::V);
    assert!(!a.note_mode);
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    assert!(a.patterns.editing.is_none());
    assert_eq!(bytes(&a), original);
    let selected = a.selected_event.unwrap();
    assert_eq!(selected, raw_note(&a));
    assert_eq!(
        a.loaded.as_ref().unwrap().visible().tracks[source]
            .events
            .len(),
        a.loaded.as_ref().unwrap().music.tracks[source].events.len()
    );
    assert!(
        selected > index,
        "source metadata changes the actual event index"
    );
    press(&mut a, &c, Key::E);
    press(&mut a, &c, Key::V);
    frame(&mut a, &c, vec![]);
    assert!(a.note_edit.is_some());
    frame(
        &mut a,
        &c,
        vec![egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::COMMAND,
        }],
    );
    frame(&mut a, &c, vec![egui::Event::Text("51".into())]);
    press(&mut a, &c, Key::Enter);
    assert!(a.note_edit.is_none());
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[source].events[selected].data[1],
        51
    );
    let song = cat_smf_editor::score::performance::song(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        song.tracks
            .iter()
            .flat_map(|t| &t.events)
            .filter(|e| note_on(e))
            .map(|e| e.data[1])
            .collect::<Vec<_>>(),
        vec![51, 51]
    );
    let other = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .values()
        .find(|p| p.id != id)
        .unwrap()
        .id;
    assert_eq!(
        preview::prepare(&a, other).unwrap().parts[0].2.notes[0].velocity,
        83
    );
    frame(
        &mut a,
        &c,
        vec![egui::Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::COMMAND,
        }],
    );
    assert_eq!(bytes(&a), original);
    press(&mut a, &c, Key::V);
    assert!(a.note_mode);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert_eq!(a.track, source);
    assert_eq!(
        a.loaded.as_ref().unwrap().visible().tracks[source]
            .events
            .iter()
            .filter(|e| note_on(e))
            .count(),
        1
    );
}
#[test]
fn raw_track_selection_leaves_song_alone_and_keeps_the_container_uneditable() {
    let (mut a, source, id) = edit_tests::fixture();
    let c = egui::Context::default();
    let original = bytes(&a);
    press(&mut a, &c, Key::V);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    crate::cursor::split::go(&mut a, source, 120);
    frame(&mut a, &c, vec![]);
    assert!(a.patterns.raw_context.is_none());
    let note = raw_note(&a);
    let before = bytes(&a);
    assert!(a
        .edit_music(|m| edit::note_length::set_duration(&mut m.tracks[source], note, 2200))
        .is_err());
    assert_eq!(bytes(&a), before);
    let p = crate::play::transport::prepare(&a, crate::play::playback::Mode::AllTracks).unwrap();
    assert!(p.parts.iter().all(|part| part.0 != source));
    press(&mut a, &c, Key::V);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    assert_eq!(bytes(&a), original);
    track_selected(&mut a);
    library::edit(&mut a, id).unwrap();
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
}
#[test]
fn raw_uncommitted_mml_cancel_and_mode_roundtrip_do_not_change_document_or_undo() {
    let (mut a, _, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let before = bytes(&a);
    press(&mut a, &c, Key::V);
    press(&mut a, &c, Key::I);
    frame(&mut a, &c, vec![]);
    assert!(a.mml_input.open);
    press(&mut a, &c, Key::V);
    assert!(!a.note_mode);
    press(&mut a, &c, Key::Escape);
    assert!(!a.mml_input.open);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    press(&mut a, &c, Key::V);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn mode_roundtrip_keeps_the_selected_occurrence_of_identical_events() {
    let (mut a, source, id) = edit_tests::fixture();
    a.edit_tracks(|m| {
        let on = m.tracks[source]
            .events
            .iter()
            .find(|e| note_on(e))
            .unwrap()
            .clone();
        let mut off = on.clone();
        off.tick = 720;
        off.status = 0x82;
        off.data[1] = 0;
        edit::insert(&mut m.tracks[source], on);
        edit::insert(&mut m.tracks[source], off);
        edit::refresh_deltas(&mut m.tracks[source])
    })
    .unwrap();
    library::edit(&mut a, id).unwrap();
    let events = &a.loaded.as_ref().unwrap().visible().tracks[source].events;
    let second = events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .nth(1)
        .unwrap()
        .0;
    let row = a.loaded.as_ref().unwrap().display_rows[source]
        .event_row(second)
        .unwrap();
    crate::cursor::moves::place(&mut a, row);
    mode::toggle(&mut a);
    let raw = a.loaded.as_ref().unwrap().music.tracks[source]
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .nth(1)
        .unwrap()
        .0;
    assert_eq!(a.selected_event, Some(raw));
    mode::toggle(&mut a);
    assert_eq!(a.selected_event, Some(second));
}
