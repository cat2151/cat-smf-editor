use crate::{
    play::playback::{self, Mode},
    view::tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

fn fixture() -> crate::app::App {
    let events = [
        (120, 0x90, 60, 100),
        (120, 0x90, 64, 80),
        (600, 0x80, 60, 0),
        (600, 0x80, 64, 0),
        (2100, 0x90, 67, 90),
        (2500, 0x80, 67, 0),
        (4000, 0xff, 0, 0),
    ]
    .into_iter()
    .map(|(tick, status, pitch, velocity)| Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    })
    .collect();
    app(Music {
        format: 0,
        division: 480,
        tracks: vec![Track { events }],
    })
}
fn press(app: &mut crate::app::App, keys: &[(egui::Key, egui::Modifiers)]) {
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: keys
                .iter()
                .map(|&(key, modifiers)| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                })
                .collect(),
            ..Default::default()
        },
        |ctx| {
            playback::shortcuts(app, ctx);
            crate::cursor::moves::shortcuts(app, ctx);
        },
    );
}
#[test]
fn overlay_closes_on_play_and_space_reuses_each_method() {
    let mut app = fixture();
    let none = egui::Modifiers::NONE;
    for (key, mode) in [
        (egui::Key::A, Mode::AllTracks),
        (egui::Key::C, Mode::AllTracksFromMeasure),
        (egui::Key::S, Mode::MeasureStart),
        (egui::Key::M, Mode::Measure),
        (egui::Key::T, Mode::Track),
        (egui::Key::P, Mode::Position),
        (egui::Key::B, Mode::Beat),
        (egui::Key::I, Mode::BeatOnly),
        (egui::Key::E, Mode::Event),
        (egui::Key::N, Mode::Notes),
    ] {
        press(&mut app, &[(egui::Key::P, none)]);
        assert!(app.playback.open);
        press(&mut app, &[(key, none)]);
        assert!(!app.playback.open);
        assert_eq!(app.playback.last, mode);
        press(&mut app, &[(egui::Key::Space, none)]);
        assert!(!app.playback.open);
        assert_eq!(app.playback.last, mode);
    }
    press(&mut app, &[(egui::Key::P, none), (egui::Key::Escape, none)]);
    assert!(!app.playback.open);
    assert_eq!(app.playback.last, Mode::Notes);
}

#[test]
fn ps_starts_at_current_measure_and_continues_to_track_end() {
    let mut app = fixture();
    app.selected_event = Some(4);
    crate::cursor::moves::sync_mode(&mut app);
    let sequence = playback::sequence(&app, Mode::MeasureStart).unwrap();
    assert_eq!(sequence.notes.len(), 1);
    assert!((sequence.notes[0].start - 0.1875).abs() < 1e-9);
    assert!((sequence.end - (4000.0 - 1920.0) / 960.0).abs() < 1e-9);
}

#[test]
fn pa_merges_and_sorts_tracks_and_uses_each_format_two_tempo() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.format = 2;
    loaded.music.tracks.push(Track {
        events: vec![
            Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x51,
                data: vec![0x0f, 0x42, 0x40],
            },
            Event {
                tick: 48,
                delta: 48,
                status: 0x90,
                meta_type: 0,
                data: vec![72, 70],
            },
            Event {
                tick: 4800,
                delta: 4752,
                status: 0x80,
                meta_type: 0,
                data: vec![72, 0],
            },
        ],
    });
    loaded
        .note_views
        .push(cat_smf_editor::score::note_view::NoteView::new(
            &loaded.music.tracks[1].events,
        ));
    loaded
        .patches
        .push(cat_smf_editor::score::instrument::Patch::from(
            cat_smf_editor::chain_settings::Settings {
                instrument: Some(crate::test_support::referenced(
                    cat_smf_editor::chain_settings::PluginState {
                        reference: None,
                        plugin: cat_smf_editor::host::PluginKey {
                            format: "CLAP".into(),
                            id: "track-two".into(),
                            name: "Second tone".into(),
                            vendor: "fixture".into(),
                            bundle_path: "X:/plugins/second.clap".into(),
                        },
                        state: vec![1],
                        preset_name: None,
                        bypass: false,
                    },
                )),
                ..Default::default()
            },
        ));
    app.selected_event = Some(4);
    let sequence = playback::sequence(&app, Mode::AllTracks).unwrap();
    assert_eq!(sequence.notes.len(), 4);
    assert_eq!(sequence.notes[0].pitch, 72);
    assert!((sequence.notes[0].start - 0.1).abs() < 1e-9);
    assert!((sequence.notes[1].start - 0.125).abs() < 1e-9);
    assert!((sequence.end - 10.0).abs() < 1e-9);
    let parts = playback::parts(&app, Mode::AllTracks);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].0, 0);
    assert_eq!(parts[1].0, 1);
    assert_ne!(parts[0].1, parts[1].1);
    assert!((parts[0].2.notes[0].start - 0.125).abs() < 1e-9);
    assert!((parts[1].2.notes[0].start - 0.1).abs() < 1e-9);
    assert!((parts[1].2.end - 10.0).abs() < 1e-9);
    assert!(sequence.notes.windows(2).all(|n| n[0].start <= n[1].start));
}
#[test]
fn ranges_and_event_vs_chord_use_current_cursor() {
    let mut app = fixture();
    app.selected_event = Some(0);
    crate::cursor::moves::sync_mode(&mut app);
    assert_eq!(
        playback::sequence(&app, Mode::Event).unwrap().notes.len(),
        1
    );
    assert_eq!(
        playback::sequence(&app, Mode::Notes).unwrap().notes.len(),
        2
    );
    let measure = playback::sequence(&app, Mode::Measure).unwrap();
    assert_eq!(measure.notes.len(), 2);
    assert!((measure.end - 2.0).abs() < 1e-9);
    assert!((measure.notes[0].start - 0.125).abs() < 1e-9);
    let position = playback::sequence(&app, Mode::Position).unwrap();
    assert_eq!(position.notes[0].start, 0.0);
    let beat = playback::sequence(&app, Mode::Beat).unwrap();
    assert!((beat.notes[0].start - 0.125).abs() < 1e-9);
    app.selected_event = Some(4);
    crate::cursor::moves::sync_mode(&mut app);
    assert_eq!(
        playback::sequence(&app, Mode::Measure).unwrap().notes.len(),
        1
    );
    assert_eq!(
        playback::sequence(&app, Mode::Track).unwrap().notes.len(),
        3
    );
}
#[test]
fn measure_and_beat_moves_land_on_empty_boundaries() {
    let mut app = fixture();
    app.selected_event = Some(4);
    crate::cursor::moves::sync_mode(&mut app);
    let none = egui::Modifiers::NONE;
    press(&mut app, &[(egui::Key::H, none)]);
    assert_eq!(playback::location(&app).1.unwrap().to_string(), "1:1:0");
    press(&mut app, &[(egui::Key::ArrowRight, none)]);
    assert_eq!(playback::location(&app).1.unwrap().to_string(), "2:1:0");
    press(&mut app, &[(egui::Key::J, egui::Modifiers::SHIFT)]);
    assert_eq!(playback::location(&app).1.unwrap().to_string(), "2:2:0");
    press(&mut app, &[(egui::Key::K, egui::Modifiers::SHIFT)]);
    assert_eq!(playback::location(&app).1.unwrap().to_string(), "2:1:0");
}
#[test]
fn audio_scheduler_splits_at_onsets_and_end_without_restarting_other_pitches() {
    use crate::play::audio_sequence::{Cursor, Note, Sequence};
    let sequence = Sequence {
        notes: vec![
            Note {
                start: 0.1,
                duration: 0.5,
                pitch: 60,
                velocity: 100,
            },
            Note {
                start: 0.2,
                duration: 0.1,
                pitch: 64,
                velocity: 80,
            },
        ],
        end: 0.6,
    };
    let mut cursor = Cursor::default();
    let mut velocities = [0; 128];
    let mut durations = [0; 128];
    assert_eq!(
        cursor.block(&sequence, 1000, 1024, &mut velocities, &mut durations),
        (100, 0)
    );
    assert_eq!(
        cursor.block(&sequence, 1000, 1024, &mut velocities, &mut durations),
        (100, 2)
    );
    assert_eq!(durations[60], 500);
    assert_eq!(
        cursor.block(&sequence, 1000, 1024, &mut velocities, &mut durations),
        (400, 2)
    );
    assert_eq!((velocities[60], velocities[64]), (0, 80));
    assert_eq!(
        cursor.block(&sequence, 1000, 1024, &mut velocities, &mut durations),
        (1024, 3)
    );
}

#[test]
fn raw_navigation_preserves_empty_boundary_and_session() {
    let mut app = fixture();
    app.note_mode = false;
    app.selected_event = Some(4);
    crate::cursor::moves::sync_mode(&mut app);
    press(&mut app, &[(egui::Key::H, egui::Modifiers::NONE)]);
    assert_eq!(playback::location(&app).1.unwrap().to_string(), "1:1:0");
    assert_eq!(app.selected_event, None);
    let saved = app.session().unwrap();
    let mut restored = fixture();
    restored.restore_position(&saved);
    assert_eq!(
        playback::location(&restored).1.unwrap().to_string(),
        "1:1:0"
    );
    press(&mut restored, &[(egui::Key::L, egui::Modifiers::NONE)]);
    assert_eq!(
        playback::location(&restored).1.unwrap().to_string(),
        "2:1:0"
    );
}

#[test]
fn meter_changes_and_fractional_positions_round_trip() {
    use cat_smf_editor::score::musical_time::MusicalTime;
    let music = Music {
        format: 0,
        division: 3,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x58,
                    data: vec![4, 3, 24, 8],
                },
                Event {
                    tick: 5,
                    delta: 5,
                    status: 0xff,
                    meta_type: 0x58,
                    data: vec![3, 2, 24, 8],
                },
            ],
        }],
    };
    let time = MusicalTime::new(&music);
    let map = time.map(0).unwrap();
    for tick in 0..30 {
        assert_eq!(map.tick_at(&map.position(tick)), Some(tick));
    }
    let previous = map.adjacent_beat(&map.position(5), false, 30).unwrap();
    assert_eq!(previous.to_string(), "1:4:0");
    assert_eq!(map.tick_at(&previous), Some(5));
    let next = map.adjacent_beat(&previous, true, 30).unwrap();
    assert_eq!(next.to_string(), "2:1:0");
}

#[test]
fn full_ui_overlay_m_does_not_toggle_note_mode() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    for key in [egui::Key::P, egui::Key::M] {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(&mut app, ctx),
        );
    }
    assert!(app.note_mode);
    assert!(!app.playback.open);
    assert_eq!(app.playback.last, Mode::Measure);
}
