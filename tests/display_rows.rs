use cat_smf_editor::{
    model::{Event, Music, Track},
    score::display_rows::{DisplayRow, DisplayRows},
    score::musical_time::MusicalTime,
    score::note_view::NoteView,
};
fn event(tick: u64, status: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: if status == 0xff { 0x58 } else { 0 },
        data,
    }
}
fn rows(events: Vec<Event>, division: u16) -> (Music, DisplayRows) {
    let music = Music {
        format: 1,
        division,
        tracks: vec![Track { events }],
    };
    let time = MusicalTime::new(&music);
    let rows = DisplayRows::new(
        &music.tracks[0].events,
        &NoteView::new(&music.tracks[0].events).rows,
        time.map(0),
    );
    (music, rows)
}
fn label(rows: &DisplayRows, i: usize) -> String {
    match rows.get(i) {
        DisplayRow::Event(i) => format!("event{i}"),
        DisplayRow::Beat(p) => p.to_string(),
    }
}
#[test]
fn fills_rests_and_hidden_release_tail_without_duplicating_occupied_beats() {
    let (_, r) = rows(
        vec![
            event(480, 0x90, vec![60, 100]),
            event(480, 0x90, vec![64, 100]),
            event(1920, 0x80, vec![60, 0]),
            event(1920, 0x80, vec![64, 0]),
        ],
        480,
    );
    assert_eq!(
        (0..r.len()).map(|i| label(&r, i)).collect::<Vec<_>>(),
        ["1:1:0", "event0", "event1", "1:3:0", "1:4:0", "2:1:0"]
    );
    assert_eq!(r.event_row(1), Some(2));
    assert_eq!(r.event_row(2), None);
}
#[test]
fn fractional_beats_and_mid_measure_signature_changes_are_exact() {
    let (_, r) = rows(
        vec![
            event(0, 0xff, vec![4, 3, 24, 8]),
            event(2, 0x90, vec![60, 100]),
            event(4, 0xff, vec![3, 2, 24, 8]),
            event(7, 0xb0, vec![1, 10]),
        ],
        3,
    );
    assert_eq!(
        (0..r.len()).map(|i| label(&r, i)).collect::<Vec<_>>(),
        ["event0", "1:2:0", "event1", "1:3:0", "event2", "event3"]
    );
}
#[test]
fn huge_silent_spans_are_indexed_without_expanding_beats() {
    let (_, r) = rows(vec![event(4_800_000_000, 0x90, vec![60, 100])], 480);
    assert_eq!(r.len(), 2_500_001);
    assert_eq!(label(&r, 2_499_999), "2500000:1:0");
    assert!(r.is_measure(2_499_999));
    assert_eq!(label(&r, 2_500_000), "event0");
    let (_, r) = rows(vec![event(900, 0x90, vec![60, 100])], 0xe728);
    assert_eq!(r.len(), 1);
}
#[test]
fn empty_measures_collapse_to_one_row_and_occupied_measures_keep_their_beats() {
    let (_, r) = rows(
        vec![
            event(480, 0x90, vec![60, 100]),
            event(9600, 0x90, vec![62, 100]),
            event(10560, 0x80, vec![60, 0]),
            event(10560, 0x80, vec![62, 0]),
        ],
        480,
    );
    assert_eq!(
        (0..r.len()).map(|i| label(&r, i)).collect::<Vec<_>>(),
        [
            "1:1:0", "event0", "1:3:0", "1:4:0", "2:1:0", "3:1:0", "4:1:0", "5:1:0", "event1",
            "6:2:0", "6:3:0"
        ]
    );
    assert_eq!(
        (0..r.len())
            .filter(|&i| r.is_measure(i))
            .collect::<Vec<_>>(),
        [4, 5, 6, 7]
    );
}

#[test]
fn same_measure_keeps_real_events_without_synthetic_beats() {
    let (_, r) = rows(
        vec![
            cat_smf_editor::score::same_measure::event(0, 3),
            event(960, 0xb0, vec![1, 20]),
            event(1920, 0x90, vec![60, 90]),
        ],
        480,
    );
    assert_eq!(
        (0..r.len()).map(|i| label(&r, i)).collect::<Vec<_>>(),
        ["event0", "event1", "event2"]
    );
}
