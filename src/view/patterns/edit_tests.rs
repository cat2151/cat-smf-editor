use super::*;
use crate::view::palette::tests::{frame, press};
use cat_smf_editor::{
    edit,
    model::{Event, Music, Track},
    score::{
        note_view::note_on,
        pattern::{self, operations},
    },
};
use eframe::egui::{self, Key};

pub(crate) fn fixture() -> (App, usize, u64) {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    tick: 480,
                    delta: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
            ],
        }],
    };
    let source = operations::create_source(&mut m).unwrap();
    let id = operations::create(&mut m, source, None).unwrap();
    for (tick, status) in [(120, 0x92), (600, 0x82)] {
        edit::insert(
            &mut m.tracks[source],
            Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![67, 83],
            },
        );
    }
    operations::create(&mut m, source, Some(id)).unwrap();
    (crate::view::tests::app(m), source, id)
}
/// The visible index of the edited body's Note On at `tick` on the selected track.
fn on_at(a: &App, tick: u64) -> usize {
    a.loaded.as_ref().unwrap().visible().tracks[a.track]
        .events
        .iter()
        .position(|e| note_on(e) && e.tick == tick)
        .unwrap()
}
fn on(a: &App) -> usize {
    on_at(a, 120)
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
/// The other pattern of the fixture: the duplicate of `id`.
fn other(a: &App, id: u64) -> u64 {
    pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .keys()
        .copied()
        .find(|&p| p != id)
        .unwrap()
}

#[test]
fn pattern_edit_common_boundary_maps_velocity_start_length_and_rejects_outside() {
    let (mut a, source, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let event = on(&a);
    a.edit_music(|m| edit::note_value::set_velocity(&mut m.tracks[source], event, 45))
        .unwrap();
    let event = on(&a);
    a.edit_music(|m| edit::note_start::move_start(&mut m.tracks[source], event, 180))
        .unwrap();
    let event = on_at(&a, 180);
    a.edit_music(|m| edit::note_length::set_duration(&mut m.tracks[source], event, 300))
        .unwrap();
    let p = &pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns[&id];
    assert_eq!(p.track, source);
    let l = a.loaded.as_ref().unwrap();
    let src = l.music.tracks[source]
        .events
        .iter()
        .find(|e| note_on(e) && e.tick < 1920)
        .unwrap();
    assert_eq!((src.tick, src.data[1]), (180, 45));
    let before = bytes(&a);
    assert!(a
        .edit_music(|m| edit::note_length::set_duration(&mut m.tracks[source], event, 3000))
        .is_err());
    assert_eq!(bytes(&a), before);
    assert!(a
        .edit_music(|m| edit::note_move::move_note(&mut m.tracks[source], event, -2000))
        .is_err());
    assert_eq!(bytes(&a), before);
    a.undo_music().unwrap();
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert!(a.loaded.as_ref().unwrap().display_music.is_some());
}

#[test]
fn pattern_edit_selection_delete_is_source_only_and_undoable() {
    let (mut a, source, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let before = bytes(&a);
    let row = a.loaded.as_ref().unwrap().display_rows[source]
        .event_row(on(&a))
        .unwrap();
    crate::cursor::moves::place(&mut a, row);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::D);
    assert!(a.loaded.as_ref().unwrap().music.tracks[source]
        .events
        .iter()
        .all(|e| !note_on(e) || e.tick >= 1920));
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[source]
            .events
            .iter()
            .filter(|e| note_on(e))
            .count(),
        1
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}

#[test]
fn pattern_edit_mml_preview_cancel_confirm_and_stale_take_are_safe() {
    let (mut a, source, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let before = bytes(&a);
    crate::view::mml_input::measure::begin(&mut a, &c);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert!(a.mml_input.open);
    frame(&mut a, &c, vec![egui::Event::Text("o5 l4 c".into())]);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[source]
            .events
            .iter()
            .find(|e| note_on(e))
            .unwrap()
            .data[0],
        60
    );
    press(&mut a, &c, Key::Escape);
    assert_eq!(bytes(&a), before);
    library::edit(&mut a, id).unwrap();
    crate::view::mml_input::measure::begin(&mut a, &c);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![egui::Event::Text("o5 l4 d".into())]);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    press(&mut a, &c, Key::Enter);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert!(!a.mml_input.open);
    assert_eq!(
        a.loaded.as_ref().unwrap().music.tracks[source]
            .events
            .iter()
            .find(|e| note_on(e))
            .unwrap()
            .data[0],
        62
    );
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
    library::edit(&mut a, id).unwrap();
    crate::view::mml_input::open(&mut a, &c, false);
    let other = other(&a, id);
    library::edit(&mut a, other).unwrap();
    let switched = bytes(&a);
    frame(&mut a, &c, vec![]);
    press(&mut a, &c, Key::Enter);
    assert!(!a.mml_input.open);
    assert_eq!(bytes(&a), switched);
}

#[test]
fn pattern_edit_source_direct_input_is_disabled_and_running_sequence_refreshes() {
    let (mut a, source, id) = fixture();
    let c = egui::Context::default();
    library::edit(&mut a, id).unwrap();
    crate::play::transport::repeat::start(&mut a, (0, 1920));
    let event = on(&a);
    a.edit_music(|m| edit::note_value::set_pitch(&mut m.tracks[source], event, 72))
        .unwrap();
    let p = a.playback.pending.as_ref().unwrap();
    assert_eq!(a.playback.repeat, Some((0, 1920)));
    assert_eq!(p.parts[0].0, source);
    assert_eq!(p.parts[0].2.notes[0].pitch, 72);
    a.patterns.editing = None;
    refresh(&mut a);
    a.select_track(source);
    let before = bytes(&a);
    assert!(a
        .edit_music(|m| {
            m.tracks[source].events.clear();
            Ok(())
        })
        .is_err());
    crate::view::mml_input::open(&mut a, &c, false);
    assert!(!a.mml_input.open);
    assert_eq!(bytes(&a), before);
}

#[test]
fn pattern_edit_old_note_input_is_rejected_and_long_mml_extends_the_current_pattern() {
    let (mut a, source, id) = fixture();
    let c = egui::Context::default();
    library::edit(&mut a, id).unwrap();
    let event = on(&a);
    crate::view::note_edit::begin(
        &mut a,
        crate::view::note_edit::Field::Pitch,
        source,
        event,
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 20.0)),
        "C5",
    );
    assert!(a.note_edit.is_some());
    let other = other(&a, id);
    library::edit(&mut a, other).unwrap();
    let before = bytes(&a);
    crate::view::note_edit::confirm(&mut a, &c);
    assert_eq!(bytes(&a), before);
    a.note_edit = None;
    crate::view::mml_input::measure::begin(&mut a, &c);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![egui::Event::Text("o5 c4 r1 r1".into())]);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    press(&mut a, &c, Key::Enter);
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert!(!a.mml_input.open);
    let catalog = pattern::Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(catalog.patterns[&other].meters.len(), 3);
    assert_eq!(catalog.patterns[&id].meters.len(), 1);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}

#[test]
fn pattern_single_note_playback_maps_visible_index_to_the_body() {
    let (mut a, source, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let index = on(&a);
    let row = a.loaded.as_ref().unwrap().display_rows[source]
        .event_row(index)
        .unwrap();
    crate::cursor::moves::place(&mut a, row);
    let seq = crate::play::playback::sequence(&a, crate::play::playback::Mode::Event).unwrap();
    assert_eq!(seq.notes.len(), 1);
    assert_eq!(seq.notes[0].pitch, 67);
    assert_eq!(seq.notes[0].start, 0.0);
    assert_eq!(seq.notes[0].duration, 0.5);
}
