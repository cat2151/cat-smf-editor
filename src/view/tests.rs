use crate::app::{App, Loaded};
use cat_smf_editor::{
    model::{Event, Music, Track},
    score::musical_time::MusicalTime,
    score::note_view::NoteView,
};
use eframe::egui;

pub(crate) fn app(music: Music) -> App {
    let time = MusicalTime::new(&music);
    let track_titles = music
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| format!("Track {} · {} events", i + 1, t.events.len()))
        .collect();
    App {
        mascot: Default::default(),
        startup: Default::default(),
        opening: Default::default(),
        palette: Default::default(),
        selection_history: Default::default(),
        undo: Default::default(),
        file_open_requested: false,
        recent: Default::default(),
        song_id: 0,
        layer_editor: Default::default(),
        effect_editor: Default::default(),
        state_cache: cat_smf_editor::score::state_cache::Cache::new(None),
        patch_roots: crate::player::patch_roots::Cache::new(None),
        error_log: crate::store::error_log::Log::default(),
        player_selection: crate::player::selection::State {
            executable: Some("X:/missing/player.exe".into()),
            ..Default::default()
        },
        saved: crate::store::saved::Document::default(),
        settings_error: None,
        config: crate::store::config::Store::default(),
        comma_menu: crate::view::comma_menu::State::default(),
        voicevox: Default::default(),
        analysis: crate::view::analysis::State::default(),
        event_selection: Default::default(),
        mml_input: Default::default(),
        busy: Default::default(),
        note_edit: None,
        trill_edit: None,
        note_edit_request: None,
        edit_menu: crate::view::edit_menu::State::default(),
        piano_roll: crate::view::piano_roll::State::default(),
        staff: crate::view::staff::State::default(),
        tempo_curve: crate::view::tempo_curve::State::default(),
        macro_editor: Default::default(),
        parameter_editor: Default::default(),
        last_command: None,
        loaded: Some(Loaded {
            patches: music
                .tracks
                .iter()
                .map(|track| {
                    cat_smf_editor::score::instrument::read(
                        track,
                        &mut crate::player::patch_roots::Assumed,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            original: "fixture.mid".into(),
            display_rows: music
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    cat_smf_editor::score::display_rows::DisplayRows::new(
                        &t.events,
                        &NoteView::new(&t.events).rows,
                        time.map(i),
                    )
                })
                .collect(),
            name: "fixture.mid".into(),
            keys: cat_smf_editor::score::key_signature::KeySignatures::new(&music),
            time_columns: music
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| crate::view::time_columns::TimeColumns::new(&t.events, time.map(i)))
                .collect(),
            note_views: music
                .tracks
                .iter()
                .map(|t| NoteView::new(&t.events))
                .collect(),
            music,
            time,
            track_titles,
        }),
        load_error: None,
        invalid_smf_name: None,
        missing_last_mid: None,
        track: 0,
        track_view: crate::cursor::split::State::default(),
        selected_event: None,
        reset_scroll: true,
        generation: 0,
        note_mode: true,
        help_open: false,
        cursor_row: None,
        cursor_position: None,
        reveal_cursor: false,
        audition: crate::play::audition::Audition::disabled(),
        playback: crate::play::playback::Playback::default(),
    }
}

#[test]
fn headless_render_virtualizes_large_track_and_handles_smpte() {
    let events = (0..100_000)
        .map(|tick| Event {
            delta: 1,
            tick,
            status: 0x90,
            meta_type: 0,
            data: vec![60, 0],
        })
        .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    let ctx = egui::Context::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            ..Default::default()
        },
        |ctx| crate::view::show(&mut app, ctx),
    );
    assert!(!output.shapes.is_empty());
    assert!(
        output.shapes.len() < 5000,
        "table rendered an excessive number of rows"
    );
    assert!(!app.reset_scroll);
    app.loaded.as_mut().unwrap().music.division = 0xe728;
    app.loaded.as_mut().unwrap().time = MusicalTime::new(&app.loaded.as_ref().unwrap().music);
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
}

#[test]
fn selecting_file_resets_track_and_failed_load_preserves_document() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    });
    let root = std::env::temp_dir().join(format!("cat-smf-file-selection-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let source = root.join("selection.mid");
    std::fs::write(
        &source,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
    )
    .unwrap();
    app.track = 99;
    app.selected_event = Some(999);
    app.loaded = None;
    app.open_mid(source.clone());
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    assert_eq!(app.track, 0);
    assert!(app.selected_event.is_none() && app.reset_scroll);
    assert!(app.loaded.is_some() && app.load_error.is_none());
    app.open_mid(std::env::temp_dir().join(format!("missing-smf-{}.mid", std::process::id())));
    assert!(app.loaded.is_some() && app.load_error.is_some());
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn v_toggles_modes_without_repeating_or_changing_selected_source_event() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    delta: 480,
                    tick: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
            ],
        }],
    });
    assert!(app.note_mode);
    assert_eq!(app.loaded.as_ref().unwrap().note_views[0].rows, vec![0]);
    app.selected_event = Some(0);
    let ctx = egui::Context::default();
    for (pressed, repeat, expected) in [
        (true, false, false),
        (true, true, false),
        (false, false, false),
        (true, false, true),
    ] {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::V,
                    physical_key: Some(egui::Key::V),
                    pressed,
                    repeat,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(&mut app, ctx),
        );
        assert_eq!(app.note_mode, expected);
        assert_eq!(app.selected_event, Some(0));
    }
}

#[test]
fn readable_events_start_at_note_column_and_raw_keeps_source_bytes() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0xc0,
                    meta_type: 0,
                    data: vec![19],
                },
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0xb0,
                    meta_type: 0,
                    data: vec![64, 127],
                },
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    delta: 480,
                    tick: 480,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
            ],
        }],
    });
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1800.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::epaint::Shape::Text(text) = &s.shape {
                Some((text.galley.text().to_owned(), text.pos))
            } else {
                None
            }
        })
        .collect();
    let position = |label: &str| texts.iter().find(|(text, _)| text == label).unwrap().1;
    let note = position("C4");
    assert!((position("Program Church Organ").x - note.x).abs() < 1.0);
    assert!((position("Sustain on").x - note.x).abs() < 1.0);
    assert!(texts.iter().all(|(text, _)| {
        ![
            "Channel",
            "Type",
            "Contents",
            "Selected event — full payload",
        ]
        .contains(&text.as_str())
    }));
    app.note_mode = false;
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::epaint::Shape::Text(text) = &s.shape {
                Some(text.galley.text())
            } else {
                None
            }
        })
        .collect();
    for bytes in ["C0 13", "B0 40 7F", "90 3C 64", "80 3C 00"] {
        assert!(texts.contains(&bytes), "missing raw source event: {bytes}");
    }
}

#[test]
fn mbt_color_marks_beat_first_row_its_tick_and_later_rows() {
    let events = [0, 0, 10, 490, 490, 500, 960, 1440, 1920]
        .into_iter()
        .map(|tick| Event {
            delta: 0,
            tick,
            status: 0x90,
            meta_type: 0,
            data: vec![60, 100],
        })
        .collect();
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    });
    app.note_mode = false;
    app.cursor_row = Some(4);
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1800.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let output = ctx.run(input(), |ctx| crate::view::show(&mut app, ctx));
    let colors = |label: &str| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == label => Some(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .map(|s| s.format.color)
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let rgb = egui::Color32::from_rgb;
    let normal = ctx.style().visuals.text_color();
    // Measure, beat, tick sections; only the beat is colored.
    let beat = |label: &str| {
        colors(label)
            .into_iter()
            .map(|sections| {
                assert_eq!((sections[0], sections[2]), (normal, normal));
                sections[1]
            })
            .collect::<Vec<_>>()
    };
    let (cyan, pink) = (rgb(0x66, 0xd9, 0xef), rgb(0xf9, 0x26, 0x72));
    // Beat's first row, the rest of its tick dimmed, later ticks in normal text.
    assert_eq!(beat("1:1: 0"), [cyan, rgb(0x38, 0x77, 0x83)]);
    assert_eq!(beat("1:1:10"), [normal]);
    // The first row need not sit on the beat; the selected row keeps its MBT color.
    assert_eq!(colors(" 490")[1], [egui::Color32::LIGHT_BLUE]);
    assert_eq!(beat("1:2:10"), [pink, rgb(0x88, 0x14, 0x3e)]);
    assert_eq!(beat("1:2:20"), [normal]);
    assert_eq!(beat("1:3: 0"), [rgb(0xfd, 0x97, 0x1f)]);
    assert_eq!(beat("1:4: 0"), [rgb(0xa6, 0xe2, 0x2e)]);
    assert_eq!(beat("2:1: 0"), [cyan]);
}

#[test]
fn tab_does_not_take_focus_from_view_shortcuts() {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: Vec::new() }],
    });
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
    for _ in 0..3 {
        let mut input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: Some(egui::Key::Tab),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        crate::app::drop_tab(&mut input);
        let _ = ctx.run(input, |ctx| crate::view::show(&mut app, ctx));
        assert!(!ctx.wants_keyboard_input());
    }
}
