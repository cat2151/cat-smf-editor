use super::*;
use crate::play::{
    audio_sequence::{Note, Sequence},
    automation::{OwnerPlan, RuntimePlan},
    playback::{self, Mode},
};
use cat_smf_editor::{
    model::{Event as SmfEvent, Music, Track},
    score::audition_timing::TempoMap,
};
use std::sync::Arc;

fn music(events: &[(u64, u8, u8, u8)]) -> Music {
    Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: events
                .iter()
                .map(|&(tick, status, pitch, velocity)| SmfEvent {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![pitch, velocity],
                })
                .collect(),
        }],
    }
}
fn sequence(music: Music) -> Sequence {
    playback::sequence(&crate::view::tests::app(music), Mode::Track).unwrap()
}
fn words(timeline: &Timeline, sample: u64) -> Vec<u32> {
    timeline.events_at(sample).iter().map(|e| e.word).collect()
}

#[test]
fn source_controls_keep_order_and_do_not_close_same_numbered_notes() {
    let seq = sequence(music(&[
        (0, 0x93, 7, 100),
        (0, 0xb3, 7, 64),
        (240, 0xe3, 0, 96),
        (480, 0x83, 7, 37),
        (480, 0xb3, 7, 100),
    ]));
    let timeline = Timeline::new(&seq, 0., 48000);
    assert_eq!(words(&timeline, 0), vec![0x2093_0764, 0x20b3_0740]);
    assert_eq!(words(&timeline, 12000), vec![0x20e3_0060]);
    assert_eq!(words(&timeline, 24000), vec![0x2083_0725, 0x20b3_0764]);
    assert_eq!(seq.events.len(), 5);
}

#[test]
fn source_off_on_order_release_velocity_and_channels_survive_playback() {
    let seq = sequence(music(&[
        (0, 0x92, 72, 100),
        (240, 0x82, 72, 45),
        (240, 0x92, 60, 90),
        (480, 0x92, 60, 0),
    ]));
    let timeline = Timeline::new(&seq, 0.0, 48000);
    assert_eq!(words(&timeline, 12000), vec![0x2082_482d, 0x2092_3c5a]);
    assert_eq!(words(&timeline, 24000), vec![0x2082_3c00]);
}

#[test]
fn zero_velocity_source_notes_become_explicit_offs_without_editing_smf() {
    let source = music(&[
        (0, 0x9f, 60, 100),
        (240, 0x9f, 60, 0),
        (240, 0x9f, 60, 90),
        (480, 0x8f, 60, 45),
    ]);
    let original = cat_smf_editor::smf_io::encode(&source).unwrap();
    let app = crate::view::tests::app(source);
    let seq = playback::sequence(&app, Mode::Track).unwrap();
    assert_eq!(
        words(&Timeline::new(&seq, 0.0, 48000), 12000),
        vec![0x208f_3c00, 0x209f_3c5a]
    );
    assert_eq!(
        words(&Timeline::new(&seq, 0.0, 48000), 24000),
        vec![0x208f_3c2d]
    );
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
}

#[test]
fn intentional_overlap_and_same_tick_source_order_are_not_rewritten() {
    let seq = sequence(music(&[
        (0, 0x90, 72, 100),
        (240, 0x90, 60, 90),
        (240, 0x80, 72, 0),
        (480, 0x80, 60, 0),
    ]));
    assert_eq!(
        words(&Timeline::new(&seq, 0.0, 48000), 12000),
        vec![0x2090_3c5a, 0x2080_4800]
    );
}

#[test]
fn adjacent_notes_share_a_sample_for_any_rate_seek_and_callback_size() {
    let seq = sequence(music(&[
        (0, 0x90, 72, 100),
        (7, 0x80, 72, 0),
        (7, 0x90, 60, 90),
        (13, 0x80, 60, 0),
        (13, 0x90, 62, 80),
        (24, 0x80, 62, 0),
    ]));
    for rate in [44100, 48000, 96000] {
        for origin in [0.0, 0.00143] {
            let mut seq = seq.clone();
            seq.skip(origin);
            let timeline = Timeline::new(&seq, seq.origin, rate);
            for size in [1, 7, 64, 127, 1024] {
                let mut sent = Vec::new();
                let mut at = 0;
                while at <= timeline.end {
                    sent.extend(timeline.events_at(at).iter().map(|e| (at, e.word)));
                    at += timeline.next(at, size) as u64;
                }
                assert_eq!(
                    sent,
                    timeline
                        .events
                        .iter()
                        .map(|e| (e.sample, e.word))
                        .collect::<Vec<_>>()
                );
                for pair in sent
                    .windows(2)
                    .filter(|pair| pair[0].1 >> 20 & 15 == 8 && pair[1].1 >> 20 & 15 == 9)
                {
                    assert_eq!(
                        pair[0].0, pair[1].0,
                        "rate={rate} origin={origin} block={size}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_zero_length_note_keeps_its_own_on_before_off_when_ticks_collapse() {
    let seq = sequence(music(&[
        (0, 0x90, 72, 100),
        (1, 0x80, 72, 0),
        (1, 0x90, 60, 90),
        (1, 0x80, 60, 0),
        (2, 0x90, 62, 80),
        (3, 0x80, 62, 0),
    ]));
    let timeline = Timeline::new(&seq, 0.0, 100);
    assert_eq!(
        words(&timeline, 0),
        vec![
            0x2090_4864,
            0x2080_4800,
            0x2090_3c5a,
            0x2080_3c00,
            0x2090_3e50,
            0x2080_3e00
        ]
    );
}

#[test]
fn seek_at_boundary_and_repeat_end_do_not_resurrect_or_leave_notes() {
    let mut seq = sequence(music(&[
        (0, 0x90, 72, 100),
        (240, 0x80, 72, 0),
        (240, 0x90, 60, 90),
        (960, 0x80, 60, 0),
    ]));
    seq.skip(0.25);
    seq.truncate(0.125);
    let timeline = Timeline::new(&seq, seq.origin, 48000);
    assert_eq!(words(&timeline, 0), vec![0x2080_4800, 0x2090_3c5a]);
    assert_eq!(words(&timeline, 6000), vec![0x2080_3c00]);
    assert_eq!(seq.notes.len(), 1);
}

#[test]
fn automation_uses_the_same_note_samples_and_preserves_order() {
    let song = music(&[
        (0, 0x90, 72, 100),
        (7, 0x80, 72, 0),
        (7, 0x90, 60, 90),
        (13, 0x80, 60, 0),
    ]);
    let owner = Arc::new(OwnerPlan {
        windows: Default::default(),
        midi: false,
        owner: 0,
        live: Default::default(),
        curves: vec![],
        tempo: Arc::new(TempoMap::new(&song, 0)),
        boundaries: vec![],
    });
    let seq = sequence(song);
    let timeline = Timeline::new(&seq, 0.0, 48000);
    let plan = RuntimePlan::new(owner, &seq, 0.0, 48000);
    for event in &timeline.events {
        assert_eq!(
            plan.events_at(event.sample)
                .iter()
                .filter_map(|e| e.midi)
                .collect::<Vec<_>>(),
            words(&timeline, event.sample)
        );
    }
}

#[test]
fn synthetic_auditions_round_endpoints_instead_of_duration() {
    let seq = Sequence::new(
        vec![
            Note {
                start: 0.00143,
                duration: 0.00143,
                pitch: 72,
                velocity: 100,
            },
            Note {
                start: 0.00286,
                duration: 0.00143,
                pitch: 60,
                velocity: 90,
            },
        ],
        0.01,
    );
    assert_eq!(
        words(&Timeline::new(&seq, 0.0, 48000), 137),
        vec![0x2080_4800, 0x2090_3c5a]
    );
}

#[test]
#[ignore = "read-only verification of an explicitly selected SMF; set NOTE_ORDER_SMF"]
fn actual_smf_preserves_every_note_event_at_three_sample_rates() {
    let path = std::path::PathBuf::from(std::env::var_os("NOTE_ORDER_SMF").unwrap());
    let before = std::fs::read(&path).unwrap();
    let song = cat_smf_editor::smf_io::load(&path).unwrap();
    for track in 0..song.tracks.len() {
        let tempo = TempoMap::new(&song, track);
        let source: Vec<_> = song.tracks[track]
            .events
            .iter()
            .filter(|e| matches!(e.status & 240, 128 | 144))
            .collect();
        if source.is_empty() {
            continue;
        }
        let events = &song.tracks[track].events;
        let view = cat_smf_editor::score::note_view::NoteView::new(events);
        let selected: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| cat_smf_editor::score::note_view::note_on(e))
            .map(|(i, _)| i)
            .collect();
        let notes = selected
            .iter()
            .map(|&i| Note {
                start: tempo.seconds_at(events[i].tick),
                duration: tempo.duration(events[i].tick, view.durations[i].unwrap()),
                pitch: events[i].data[0],
                velocity: events[i].data[1],
            })
            .collect();
        let seq = Sequence::from_source(
            notes,
            tempo.seconds_at(events.last().unwrap().tick),
            events,
            &selected,
            &tempo,
            0,
        );
        for rate in [44100, 48000, 96000] {
            let timeline = Timeline::new(&seq, 0.0, rate);
            let actual: Vec<_> = timeline.events.iter().map(|e| (e.sample, e.word)).collect();
            let expected: Vec<_> = source
                .iter()
                .map(|e| {
                    let status = if e.status & 240 == 144 && e.data[1] == 0 {
                        128 | (e.status & 15)
                    } else {
                        e.status
                    };
                    (
                        frame(tempo.seconds_at(e.tick), rate),
                        0x2000_0000
                            | (u32::from(status) << 16)
                            | (u32::from(e.data[0]) << 8)
                            | u32::from(e.data[1]),
                    )
                })
                .collect();
            assert_eq!(actual, expected, "track={track} rate={rate}");
        }
        println!(
            "Track {}: {} source note events preserved at 44100/48000/96000 Hz",
            track + 1,
            source.len()
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}
