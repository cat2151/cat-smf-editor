use super::{parts, Mode};
use crate::{play::transport, view::tests::app};
use cat_smf_editor::model::{Event, Music, Track};

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: data.to_vec(),
    }
}

fn music() -> Music {
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            notes(60),
            notes(72),
            // A non-conductor containing only Note Off and velocity-zero Note On.
            Track {
                events: vec![event(0, 0x90, &[60, 0]), event(4000, 0x80, &[60, 0])],
            },
        ],
    }
}

fn notes(pitch: u8) -> Track {
    Track {
        events: vec![
            event(120, 0x90, &[pitch, 100]),
            event(600, 0x80, &[pitch, 0]),
            event(2100, 0x90, &[pitch, 80]),
            event(2500, 0x80, &[pitch, 0]),
            event(4000, 0xff, &[]),
        ],
    }
}

#[test]
fn empty_tracks_expand_every_current_track_mode_without_changing_its_range() {
    for track in [0, 3] {
        let mut app = app(music());
        app.select_track(track);
        app.track_view.cursor_tick = Some(2100);
        // The selected row index must not select unrelated events on other tracks.
        app.selected_event = Some(0);
        for (mode, start, end, count, offset) in [
            (Mode::Track, 0, 4000, 2, 120),
            (Mode::Measure, 1920, 3840, 1, 180),
            (Mode::MeasureStart, 1920, 4000, 1, 180),
            (Mode::Position, 2100, 4000, 1, 0),
            (Mode::Beat, 1920, 4000, 1, 180),
            (Mode::Event, 2100, 2500, 1, 0),
            (Mode::Notes, 2100, 2500, 1, 0),
        ] {
            let prepared = transport::prepare(&app, mode).unwrap();
            assert!((prepared.active.start_seconds - start as f64 / 960.0).abs() < 1e-9);
            assert!((prepared.active.end - (end - start) as f64 / 960.0).abs() < 1e-9);
            let sounding: Vec<_> = prepared
                .parts
                .iter()
                .filter(|(_, _, s)| !s.notes.is_empty())
                .collect();
            assert_eq!(sounding.len(), 2, "{track}: {mode:?}");
            for (index, _, sequence) in sounding {
                assert!(matches!(index, 1 | 2));
                assert_eq!(sequence.notes.len(), count, "{mode:?}");
                assert!((sequence.notes[0].start - offset as f64 / 960.0).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn a_track_with_notes_stays_single_track_even_in_an_empty_measure() {
    let mut app = app(music());
    app.select_track(1);
    app.track_view.cursor_tick = Some(3840);
    let parts = parts(&app, Mode::Measure);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, 1);
    assert!(parts[0].2.notes.is_empty());
}
