use super::*;

#[test]
fn round_trip_routes_without_changing_gm_notes_or_other_metadata() {
    let mut track = Track {
        events: vec![Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 3,
            data: b"Drums".to_vec(),
        }],
    };
    let mut settings = Settings::default();
    settings.rows[1].output = 63;
    write(&mut track, Some(&settings)).unwrap();
    for row in 0..4 {
        toggle(&mut track, 480, row, 16 + row as u64).unwrap();
    }
    let music = crate::model::Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    };
    let bytes = crate::smf_io::encode(&music).unwrap();
    let restored = crate::ffi::parse(&bytes).unwrap();
    assert_eq!(read(&restored.tracks[0]).unwrap(), Some(settings.clone()));
    let pitches: Vec<_> = restored.tracks[0]
        .events
        .iter()
        .filter(|e| note_on(e))
        .map(|e| (e.status, e.data[0]))
        .collect();
    assert_eq!(pitches, NOTES.map(|p| (0x99, p)));
    assert!(restored.tracks[0]
        .events
        .iter()
        .any(|e| e.meta_type == 3 && e.data == b"Drums"));
    for (i, row) in settings.rows.iter().enumerate() {
        for note in 0..128 {
            assert_eq!(
                settings.pitch(row.layer, note),
                (note == NOTES[i]).then_some(row.output)
            );
        }
    }
}
#[test]
fn deleting_cell_removes_its_paired_off_and_keeps_other_steps() {
    let mut track = Track { events: vec![] };
    toggle(&mut track, 480, 0, 0).unwrap();
    edit::insert(
        &mut track,
        Event {
            tick: 240,
            delta: 0,
            status: 0x90,
            meta_type: 0,
            data: vec![36, 80],
        },
    );
    edit::insert(
        &mut track,
        Event {
            tick: 400,
            delta: 0,
            status: 0x80,
            meta_type: 0,
            data: vec![36, 0],
        },
    );
    toggle(&mut track, 480, 0, 0).unwrap();
    assert_eq!(track.events.len(), 3);
    assert_eq!(track.events[0].tick, 240);
    assert_eq!(track.events[1].tick, 400);
    assert_eq!(track.events[2].meta_type, 0x2f);
}

#[test]
fn filling_steps_backwards_keeps_note_off_before_the_next_note_on() {
    let mut track = Track { events: vec![] };
    for step in (0..32).rev() {
        toggle(&mut track, 101, 2, step).unwrap();
    }
    let notes = crate::score::note_view::NoteView::new(&track.events);
    for (i, event) in track.events.iter().enumerate().filter(|(_, e)| note_on(e)) {
        assert!(matches!(notes.durations[i], Some(25 | 26)));
        if event.tick > 0 {
            assert!(crate::score::note_view::note_off(&track.events[i - 1]));
        }
    }
}
#[test]
fn fractional_grid_and_corrupt_metadata_are_checked() {
    assert_eq!(tick(101, 16).unwrap(), 404);
    assert_eq!(tick(101, 3).unwrap(), 75);
    assert!(tick(0xe728, 1).is_err());
    let mut track = Track { events: vec![] };
    write(&mut track, Some(&Settings::default())).unwrap();
    track.events[0].tick = 1;
    let original = track.clone();
    assert!(write(&mut track, None).is_err());
    assert_eq!(track.events[0].data, original.events[0].data);
    let mut settings = Settings::default();
    settings.rows[1].layer = 1;
    assert!(settings.validate().is_err());
}
