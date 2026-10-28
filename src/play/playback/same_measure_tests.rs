use crate::{
    play::{
        playback::{self, Mode},
        transport,
    },
    view::tests::app,
};
use cat_smf_editor::{
    model::{Event, Music, Track},
    score::same_measure,
};

fn fixture() -> crate::app::App {
    app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 120,
                    delta: 0,
                    status: 0x92,
                    meta_type: 0,
                    data: vec![60, 91],
                },
                Event {
                    tick: 600,
                    delta: 0,
                    status: 0x82,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 1920,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x51,
                    data: vec![15, 66, 64],
                },
                same_measure::event(1920, 1),
            ],
        }],
    })
}

#[test]
fn same_measure_sequence_uses_destination_tempo_and_full_end() {
    let mut a = fixture();
    let seq = playback::sequence(&a, Mode::Track).unwrap();
    assert_eq!(seq.notes.len(), 2);
    assert_eq!(seq.end, 6.0);
    assert_eq!(
        (
            seq.notes[1].start,
            seq.notes[1].duration,
            seq.notes[1].velocity
        ),
        (2.25, 1.0, 91)
    );
    assert_eq!(
        seq.events
            .iter()
            .map(|e| (e.time, e.word))
            .collect::<Vec<_>>(),
        vec![
            (0.125, 0x20923c5b),
            (0.625, 0x20823c00),
            (2.25, 0x20923c5b),
            (3.25, 0x20823c00)
        ]
    );
    a.track_view.cursor_tick = Some(1920);
    assert_eq!(playback::sequence(&a, Mode::Measure).unwrap().end, 4.0);
    transport::repeat::start(&mut a, (1920, 3840));
    let pending = a.playback.pending.as_ref().unwrap();
    assert_eq!(pending.active.end, 4.0);
    assert_eq!(pending.parts[0].2.notes.len(), 1);
    let saved = transport::timeline::capture(&a).unwrap();
    a.loaded.as_mut().unwrap().music.tracks[0].events[0].data[0] = 65;
    a.loaded.as_mut().unwrap().music.tracks[0].events[1].data[0] = 65;
    transport::timeline::resume(&mut a, saved);
    assert_eq!(
        a.playback.pending.as_ref().unwrap().parts[0].2.notes[0].pitch,
        65
    );
}

#[test]
fn same_measure_invalid_meta_reports_error_without_partial_playback() {
    let mut a = fixture();
    a.loaded.as_mut().unwrap().music.tracks[0]
        .events
        .push(same_measure::event(3840, 2));
    playback::start(&mut a, Mode::AllTracks);
    assert!(a.playback.pending.is_none());
    assert!(a.load_error.as_ref().unwrap().contains("Chained"));
}

#[test]
fn same_measure_empty_reference_does_not_borrow_other_tracks() {
    let mut a = fixture();
    let mut music = a.loaded.as_ref().unwrap().music.clone();
    music.tracks.push(Track {
        events: vec![same_measure::event(1920, 50)],
    });
    a = app(music);
    a.track = 1;
    let parts = playback::parts(&a, Mode::Track);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, 1);
    assert!(parts[0].2.notes.is_empty());
    assert_eq!(parts[0].2.end, 6.0);
}
