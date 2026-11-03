//! The selection menu's Crop on an opened pattern.
use super::*;
use crate::view::palette::tests::press;
use cat_smf_editor::{
    edit,
    model::{Event, Music, Track},
    score::{
        note_view::{note_off, note_on},
        pattern::{self, operations, Meter},
    },
};
use eframe::egui::{self, Key};

const BAR: u64 = 1920;

fn note(tick: u64, on: bool, pitch: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: if on { 0x90 } else { 0x80 },
        meta_type: 0,
        data: vec![pitch, if on { 100 } else { 0 }],
    }
}

/// A 3-bar pattern of an early note, then a long silence, then two notes in its last bar.
fn fixture() -> (App, usize, u64) {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    };
    let source = operations::create_source(&mut m).unwrap();
    let id = operations::create(&mut m, source, None).unwrap();
    let bar = Meter {
        numerator: 4,
        exponent: 2,
        ticks: BAR,
    };
    operations::resize(&mut m, id, vec![bar; 3]).unwrap();
    let at = pattern::Catalog::read(&m).unwrap().patterns[&id].tick;
    for (tick, on, pitch) in [
        (100, true, 60),
        (400, false, 60),
        (2 * BAR + 600, true, 64),
        (2 * BAR + 1000, false, 64),
        (2 * BAR + 1200, true, 67),
        (2 * BAR + 1500, false, 67),
    ] {
        edit::insert(&mut m.tracks[source], note(at + tick, on, pitch));
    }
    edit::refresh_deltas(&mut m.tracks[source]).unwrap();
    (crate::view::tests::app(m), source, id)
}

fn row_of_on(a: &App, tick: u64) -> usize {
    let l = a.loaded.as_ref().unwrap();
    let i = l.visible().tracks[a.track]
        .events
        .iter()
        .position(|e| note_on(e) && e.tick == tick)
        .unwrap();
    l.display_rows[a.track].event_row(i).unwrap()
}

/// Notes of pattern `id` as (start, end, pitch) from the pattern start.
fn notes(a: &App, id: u64) -> (usize, Vec<(u64, u64, u8)>) {
    let m = &a.loaded.as_ref().unwrap().music;
    let p = pattern::Catalog::read(m).unwrap().patterns[&id].clone();
    let events = pattern::body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    let mut out = vec![];
    for e in events.iter().filter(|e| note_on(e)) {
        let off = events
            .iter()
            .find(|o| note_off(o) && o.data[0] == e.data[0] && o.tick > e.tick)
            .unwrap();
        out.push((e.tick - p.tick, off.tick - p.tick, e.data[0]));
    }
    (p.meters.len(), out)
}

#[test]
fn crop_keeps_the_selected_notes_from_the_pattern_start_and_undo_restores() {
    let (mut a, _, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let before = notes(&a, id);
    let first = row_of_on(&a, 2 * BAR + 600);
    let last = row_of_on(&a, 2 * BAR + 1200);
    crate::cursor::moves::place(&mut a, first);
    press(&mut a, &c, Key::S);
    crate::cursor::moves::place(&mut a, last);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::C);
    assert_eq!(notes(&a, id), (1, vec![(0, 400, 64), (600, 900, 67)]));
    // The editor keeps showing the cropped pattern from its start.
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert_eq!(row_of_on(&a, 0), a.cursor_row.unwrap());
    a.undo_music().unwrap();
    assert_eq!(notes(&a, id), before);
}

#[test]
fn a_selected_note_on_keeps_its_note_off() {
    let (mut a, _, id) = fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    // Only the first note is selected; its Note Off comes along.
    let row = row_of_on(&a, 100);
    crate::cursor::moves::place(&mut a, row);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::C);
    assert_eq!(notes(&a, id), (1, vec![(0, 300, 60)]));
}

#[test]
fn crop_is_not_offered_outside_an_opened_pattern() {
    let (mut a, source, _) = fixture();
    a.select_track(source);
    let c = egui::Context::default();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    crate::cursor::moves::place(&mut a, 0);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::S);
    press(&mut a, &c, Key::C);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
