use cat_smf_editor::{
    model::Event,
    score::note_view::{note_name, NoteView},
};

fn event(tick: u64, status: u8, note: u8, velocity: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![note, velocity],
    }
}

#[test]
fn durations_pair_by_channel_and_pitch_in_fifo_order() {
    let events = vec![
        event(0, 0x90, 60, 100),
        event(10, 0x90, 60, 90),
        event(20, 0x91, 60, 80),
        event(30, 0x90, 64, 70),
        event(40, 0x80, 60, 50),
        event(50, 0x91, 60, 0),
        event(60, 0x90, 60, 0),
        event(70, 0x80, 64, 0),
    ];
    let view = NoteView::new(&events);
    assert_eq!(view.rows, vec![0, 1, 2, 3]);
    assert_eq!(
        view.durations,
        vec![
            Some(40),
            Some(50),
            Some(30),
            Some(40),
            None,
            None,
            None,
            None
        ]
    );
    assert_eq!(events.len(), 8);
}

#[test]
fn preserves_unmatched_releases_other_events_and_zero_length_notes() {
    let events = vec![
        event(0, 0x80, 60, 0),
        event(0, 0x90, 60, 100),
        event(0, 0x80, 60, 0),
        event(10, 0xb0, 64, 127),
        event(20, 0x90, 67, 90),
        event(30, 0x91, 67, 0),
    ];
    let view = NoteView::new(&events);
    assert_eq!(view.rows, vec![0, 1, 3, 4, 5]);
    assert_eq!(view.durations[1], Some(0));
    assert_eq!(view.durations[4], None);
    assert_eq!(note_name(0), "C-1");
    assert_eq!(note_name(60), "C4");
    assert_eq!(note_name(61), "C#4");
    assert_eq!(note_name(69), "A4");
    assert_eq!(note_name(127), "G9");
}

#[test]
fn groups_simultaneous_notes_on_last_visible_event_without_crossing_ticks() {
    let events = vec![
        event(0, 0x90, 60, 100),
        event(0, 0x91, 64, 90),
        event(0, 0xb0, 1, 70),
        event(0, 0x80, 60, 0),
        event(10, 0x90, 67, 100),
        event(10, 0x90, 67, 0),
        event(20, 0xb0, 1, 0),
    ];
    let view = NoteView::new(&events);
    assert_eq!(view.rows, vec![0, 1, 2, 4, 6]);
    assert_eq!(view.keyboard_notes[2], vec![0, 1]);
    assert_eq!(view.keyboard_notes[4], vec![4]);
    for index in [0, 1, 3, 5, 6] {
        assert!(view.keyboard_notes[index].is_empty());
    }
    assert_eq!(view.durations[0], Some(0));
    assert_eq!(view.durations[4], Some(0));
    assert!(NoteView::new(&[]).keyboard_notes.is_empty());
}

#[test]
fn overlaps_compare_with_the_next_later_note_on() {
    let events = vec![
        event(0, 0x90, 60, 90),
        event(0, 0x90, 64, 90),
        event(10, 0x80, 64, 0),
        event(10, 0x90, 67, 90),
        event(15, 0x80, 60, 0),
        event(20, 0x80, 67, 0),
        event(20, 0x90, 69, 90),
        event(30, 0x80, 69, 0),
    ];
    let view = NoteView::new(&events);
    assert_eq!(
        [0, 1, 3, 6].map(|i| view.overlaps[i]),
        [true, false, false, false]
    );
    assert_eq!(view.uniform_velocity_head, Some(0));
}

#[test]
fn differing_velocities_have_no_uniform_head() {
    let events = vec![
        event(0, 0xb0, 7, 100),
        event(0, 0x90, 60, 90),
        event(10, 0x90, 62, 80),
    ];
    assert_eq!(NoteView::new(&events).uniform_velocity_head, None);
    assert_eq!(NoteView::new(&events[..2]).uniform_velocity_head, Some(1));
}
