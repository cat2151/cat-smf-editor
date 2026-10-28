use super::*;
use crate::view::palette::tests::{frame, open, press, query};
use cat_smf_editor::model::{Event, Music, Track};

fn fixture(notes: bool) -> App {
    let mut a = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 1800,
                    delta: 1800,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 90],
                },
                Event {
                    tick: 1920,
                    delta: 120,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 2040,
                    delta: 120,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![64, 80],
                },
                Event {
                    tick: 2400,
                    delta: 360,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![64, 0],
                },
            ],
        }],
    });
    a.note_mode = notes;
    a
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn references(a: &App) -> Vec<(u64, u64)> {
    a.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .filter_map(|e| same_measure::reference(e).unwrap().map(|m| (e.tick, m)))
        .collect()
}
fn type_number(a: &mut App, c: &egui::Context, text: &str) {
    frame(a, c, vec![]);
    frame(a, c, vec![egui::Event::Text(text.into())]);
    press(a, c, Key::Enter);
}
fn select(a: &mut App, last: usize) {
    let first = if a.note_mode {
        a.loaded.as_ref().unwrap().display_rows[0]
            .event_row(0)
            .unwrap()
    } else {
        0
    };
    crate::cursor::moves::place(a, first);
    crate::view::event_selection::begin(a);
    let last = if a.note_mode {
        a.loaded.as_ref().unwrap().display_rows[0]
            .event_row(last)
            .unwrap()
    } else {
        last
    };
    crate::cursor::moves::place(a, last);
}

#[test]
fn same_measure_palette_normalizes_target_and_undoes_once() {
    let mut a = fixture(true);
    let c = egui::Context::default();
    let before = bytes(&a);
    a.track_view.cursor_tick = Some(4100);
    open(&mut a, &c);
    query(&mut a, &c, "insert same measure");
    press(&mut a, &c, Key::Enter);
    type_number(&mut a, &c, "1");
    assert!(!a.palette.is_open());
    assert_eq!(references(&a), vec![(3840, 1)]);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn same_measure_batch_palette_and_selection_menu_match_notes_and_raw() {
    for notes in [true, false] {
        for menu in [true, false] {
            let mut a = fixture(notes);
            let c = egui::Context::default();
            let before = bytes(&a);
            select(&mut a, 2);
            if menu {
                press(&mut a, &c, Key::S);
                press(&mut a, &c, Key::G);
            } else {
                open(&mut a, &c);
                query(&mut a, &c, "generate same measures");
                press(&mut a, &c, Key::Enter);
            }
            type_number(&mut a, &c, "3");
            assert!(!a.palette.is_open());
            assert_eq!(references(&a), vec![(3840, 1), (5760, 2)]);
            a.undo_music().unwrap();
            assert_eq!(bytes(&a), before);
            assert!(!a.undo.available());
        }
    }
}

#[test]
fn same_measure_selection_excludes_note_duration_and_handles_empty_beats() {
    for notes in [true, false] {
        let mut a = fixture(notes);
        select(&mut a, 0);
        assert_eq!(crate::view::event_selection::measures(&a), Some((1, 1)));
        let c = egui::Context::default();
        open(&mut a, &c);
        query(&mut a, &c, "generate same measures");
        press(&mut a, &c, Key::Enter);
        type_number(&mut a, &c, "3");
        assert_eq!(references(&a), vec![(3840, 1)]);
    }
    let mut a = fixture(true);
    let map = a.loaded.as_ref().unwrap().time.map(0).unwrap();
    let p = map.position(960);
    let rows = &a.loaded.as_ref().unwrap().display_rows[0];
    let row = (0..rows.len()).find(|&row| matches!(rows.get(row), cat_smf_editor::score::display_rows::DisplayRow::Beat(ref position) if *position == p)).unwrap();
    crate::cursor::moves::place(&mut a, row);
    crate::view::event_selection::begin(&mut a);
    assert_eq!(crate::view::event_selection::measures(&a), Some((1, 1)));
}

#[test]
fn same_measure_invalid_cancel_stale_and_collision_do_not_mutate_or_leak() {
    for value in ["0", "no", "1", "18446744073709551616"] {
        let mut a = fixture(true);
        let c = egui::Context::default();
        let before = bytes(&a);
        a.track_view.cursor_tick = Some(1920);
        open(&mut a, &c);
        query(&mut a, &c, "insert same measure");
        press(&mut a, &c, Key::Enter);
        type_number(&mut a, &c, value);
        assert!(a.palette.is_open());
        assert!(a.palette.same_measure.as_ref().unwrap().error.is_some());
        assert_eq!(bytes(&a), before);
        assert!(!a.undo.available());
        assert!(!a.playback.open);
        press(&mut a, &c, Key::Escape);
        assert!(!a.palette.is_open());
    }
    let mut a = fixture(true);
    let c = egui::Context::default();
    let before = bytes(&a);
    a.track_view.cursor_tick = Some(3840);
    open(&mut a, &c);
    query(&mut a, &c, "insert same measure");
    press(&mut a, &c, Key::Enter);
    a.song_id += 1;
    type_number(&mut a, &c, "1");
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
    assert!(!a.palette.is_open());
}
