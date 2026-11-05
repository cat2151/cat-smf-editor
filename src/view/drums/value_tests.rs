use super::*;
use values::Field;

#[test]
fn row_parameters_preserve_running_clock_scope_and_loop_without_a_pending_restart() {
    use crate::play::transport;
    for all_tracks in [false, true] {
        let mut app = tests::fixture();
        activate(&mut app).unwrap();
        app.drum_editor.field = 3;
        activate(&mut app).unwrap();
        app.drum_editor.all_tracks = all_tracks;
        app.drum_editor.loop_enabled = true;
        playback::restart(&mut app).unwrap();
        transport::advance(&mut app);
        app.audition.set_test_seconds(0.08);
        let tick = transport::tick(&app);
        let start = app.playback.active.as_ref().unwrap().start_seconds;
        let end = app.playback.active.as_ref().unwrap().end;
        let tracks: Vec<_> = app
            .playback
            .active
            .as_ref()
            .unwrap()
            .sources
            .iter()
            .map(|s| s.0)
            .collect();
        let range = app.playback.repeat;
        values::set_level(&mut app, 0, -6.0).unwrap();
        assert!(app.playback.pending.is_none());
        assert_eq!(transport::tick(&app), tick);
        assert_eq!(app.audition.seconds(), Some(0.08));
        for (field, value) in [(Field::Note, 73), (Field::Velocity, 81), (Field::Swing, 66)] {
            values::set(&mut app, 0, field, value).unwrap();
            assert!(app.playback.pending.is_none());
            assert_eq!(transport::tick(&app), tick);
            assert_eq!(app.audition.seconds(), Some(0.08));
        }
        values::toggle_swing(&mut app, 0).unwrap();
        assert!(app.playback.pending.is_none());
        assert_eq!(transport::tick(&app), tick);
        assert_eq!(app.playback.repeat, range);
        let active = app.playback.active.as_ref().unwrap();
        assert_eq!((active.start_seconds, active.end), (start, end));
        assert_eq!(
            active.sources.iter().map(|s| s.0).collect::<Vec<_>>(),
            tracks
        );
        let source = &active.sources.iter().find(|s| s.0 == 1).unwrap().1;
        assert!(source.notes.iter().all(|n| n.velocity == 81));
        assert!((source.notes[1].start - 1.0 / 6.0).abs() < 1e-9);
        app.audition.set_test_seconds(end + 0.01);
        transport::advance(&mut app);
        assert_eq!(
            app.playback.pending.as_ref().unwrap().active.start_seconds,
            start
        );
    }
}

#[test]
fn editing_parameters_while_stopped_does_not_start_a_loop() {
    let mut app = tests::fixture();
    app.drum_editor.loop_enabled = true;
    values::set(&mut app, 0, Field::Velocity, 81).unwrap();
    values::set_level(&mut app, 0, -6.0).unwrap();
    values::toggle_swing(&mut app, 0).unwrap();
    assert!(app.playback.active.is_none());
    assert!(app.playback.pending.is_none());
}

#[test]
fn parameter_change_during_single_hit_audition_preserves_selected_hits() {
    use crate::play::{playback as player, transport};
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    app.drum_editor.row = 1;
    activate(&mut app).unwrap();
    app.selected_event = app.loaded.as_ref().unwrap().visible().tracks[1]
        .events
        .iter()
        .position(|e| cat_smf_editor::score::note_view::note_on(e) && e.data[0] == 36);
    player::start(&mut app, player::Mode::Event);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.01);
    let pitches = |app: &App| {
        app.playback
            .active
            .as_ref()
            .unwrap()
            .sources
            .iter()
            .flat_map(|s| s.1.notes.iter().map(|n| n.pitch))
            .collect::<Vec<_>>()
    };
    let before = pitches(&app);
    assert_eq!(before, [36]);
    values::set_level(&mut app, 0, -6.0).unwrap();
    values::set(&mut app, 0, Field::Velocity, 81).unwrap();
    assert!(app.playback.pending.is_none());
    assert_eq!(pitches(&app), before);
    assert_eq!(app.audition.seconds(), Some(0.01));
}

#[test]
fn row_values_roundtrip_undo_validate_and_preserve_steps() {
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    let before = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .filter(|e| e.status != 0xff)
        .map(|e| (e.tick, e.status, e.data[0]))
        .collect::<Vec<_>>();
    for (field, value) in [(Field::Note, 73), (Field::Velocity, 81), (Field::Swing, 66)] {
        values::set(&mut app, 0, field, value).unwrap();
    }
    values::toggle_swing(&mut app, 0).unwrap();
    let music = &app.loaded.as_ref().unwrap().music;
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
    let settings = drums::read(&restored.tracks[1]).unwrap().unwrap();
    let r = &settings.rows[0];
    assert_eq!(
        (r.output, r.velocity, r.swing, r.swing_enabled),
        (73, 81, 66, true)
    );
    assert_eq!(settings.rows[1], drums::Settings::default().rows[1]);
    assert_eq!(
        before,
        music.tracks[1]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (e.tick, e.status, e.data[0]))
            .collect::<Vec<_>>()
    );
    assert!(values::set(&mut app, 0, Field::Velocity, 0).is_err());
    assert!(values::set(&mut app, 0, Field::Swing, 100).is_err());
    app.undo_music().unwrap();
    assert!(
        !app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .swing_enabled
    );
}

#[test]
fn new_numeric_fields_use_keyboard_input_and_escape_cancels() {
    let mut app = tests::fixture();
    let ctx = egui::Context::default();
    for _ in 0..16 {
        tests::press(&mut app, &ctx, Key::ArrowRight);
    }
    tests::press(&mut app, &ctx, Key::Enter);
    assert!(matches!(app.drum_editor.input, Some(Input::Level { .. })));
    app.drum_editor.input = Some(Input::Level {
        text: "-7.5".into(),
        focus: false,
    });
    tests::press(&mut app, &ctx, Key::Enter);
    assert_eq!(
        values::level(&app.loaded.as_ref().unwrap().patches[1], 0).unwrap(),
        -7.5
    );
    tests::press(&mut app, &ctx, Key::ArrowRight);
    tests::press(&mut app, &ctx, Key::Enter);
    tests::press(&mut app, &ctx, Key::ArrowRight);
    tests::press(&mut app, &ctx, Key::Enter);
    assert!(matches!(app.drum_editor.input, Some(Input::Swing { .. })));
    app.drum_editor.input = Some(Input::Swing {
        text: "66".into(),
        focus: false,
    });
    tests::press(&mut app, &ctx, Key::Escape);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .swing,
        50
    );
    tests::press(&mut app, &ctx, Key::ArrowRight);
    tests::press(&mut app, &ctx, Key::Enter);
    assert!(matches!(app.drum_editor.input, Some(Input::Note { .. })));
    tests::press(&mut app, &ctx, Key::Escape);
    tests::press(&mut app, &ctx, Key::ArrowRight);
    tests::press(&mut app, &ctx, Key::Enter);
    assert!(matches!(
        app.drum_editor.input,
        Some(Input::Velocity { .. })
    ));
    app.drum_editor.input = Some(Input::Velocity {
        text: "93".into(),
        focus: false,
    });
    tests::press(&mut app, &ctx, Key::Enter);
    tests::press(&mut app, &ctx, Key::ArrowLeft);
    assert_eq!(app.drum_editor.field, 1);
}

#[test]
fn wheel_snaps_velocity_and_respects_numeric_bounds_and_disabled_swing() {
    let mut app = tests::fixture();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    let draw = |app: &mut App| {
        let _ = ctx.run(input(), |ctx| show(app, ctx));
    };
    for _ in 0..3 {
        draw(&mut app);
    }
    let scroll = |app: &mut App, pos: egui::Pos2, delta| {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, delta),
                        modifiers: Default::default(),
                    },
                ],
                ..input()
            },
            |ctx| show(app, ctx),
        );
    };
    let bounds = |id| {
        ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((id, 0usize)))
                .unwrap()
        })
    };
    let velocity = bounds("drum-velocity");
    assert!(velocity.right() <= 800.0);
    scroll(&mut app, velocity.center(), 1.0);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .velocity,
        110
    );
    scroll(&mut app, velocity.center(), -1.0);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .velocity,
        100
    );
    for (value, delta, expected) in [
        (127, -1.0, 120),
        (93, -1.0, 90),
        (93, 1.0, 100),
        (120, 1.0, 127),
        (127, 1.0, 127),
        (10, -1.0, 1),
        (1, -1.0, 1),
        (1, 1.0, 10),
    ] {
        values::set(&mut app, 0, Field::Velocity, value).unwrap();
        draw(&mut app);
        scroll(&mut app, bounds("drum-velocity").center(), delta);
        assert_eq!(
            app.loaded.as_ref().unwrap().patches[1]
                .drums
                .as_ref()
                .unwrap()
                .rows[0]
                .velocity,
            expected
        );
    }
    scroll(&mut app, bounds("drum-level").center(), -1.0);
    assert_eq!(
        values::level(&app.loaded.as_ref().unwrap().patches[1], 0).unwrap(),
        -3.0
    );
    scroll(&mut app, bounds("drum-level").center(), 1.0);
    assert_eq!(
        values::level(&app.loaded.as_ref().unwrap().patches[1], 0).unwrap(),
        0.0
    );
    assert_eq!(
        values::level(&app.loaded.as_ref().unwrap().patches[1], 1).unwrap(),
        0.0
    );
    assert!(ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("drum-swing", 0usize))))
        .is_none());
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .swing,
        50
    );
    values::toggle_swing(&mut app, 0).unwrap();
    for (value, delta) in [(99, 1.0), (1, -1.0)] {
        values::set(&mut app, 0, Field::Swing, value).unwrap();
        draw(&mut app);
        scroll(&mut app, bounds("drum-swing").center(), delta);
        assert_eq!(
            app.loaded.as_ref().unwrap().patches[1]
                .drums
                .as_ref()
                .unwrap()
                .rows[0]
                .swing,
            value
        );
    }
    let cells = ctx
        .data(|d| {
            d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                "drum-grid-bounds",
            ))
        })
        .unwrap()
        .2;
    scroll(&mut app, cells[3 * 18 + 1].center(), 1.0);
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1]
            .drums
            .as_ref()
            .unwrap()
            .rows[0]
            .output,
        61
    );
}

#[test]
fn velocity_changes_existing_and_new_hits_only_in_the_selected_row() {
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    app.drum_editor.row = 1;
    activate(&mut app).unwrap();
    values::set(&mut app, 0, Field::Velocity, 81).unwrap();
    app.drum_editor.row = 0;
    app.drum_editor.field = 3;
    activate(&mut app).unwrap();
    let ons = app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| (e.data[0], e.data[1]))
        .collect::<Vec<_>>();
    assert_eq!(ons, [(36, 81), (38, 100), (36, 81)]);
}

#[test]
fn level_roundtrips_by_layer_id_preserves_notes_and_undoes() {
    let mut app = tests::fixture();
    activate(&mut app).unwrap();
    // The row refers to an ID; layer storage order is independent of row order.
    app.loaded.as_mut().unwrap().patches[1].layers.reverse();
    let before = app.loaded.as_ref().unwrap().music.clone();
    let original = app.loaded.as_ref().unwrap().patches[1].clone();
    values::set_level(&mut app, 0, -9.5).unwrap();
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.patches[1].drums, original.drums);
    assert_eq!(values::level(&loaded.patches[1], 0).unwrap(), -9.5);
    for row in 1..4 {
        assert_eq!(values::level(&loaded.patches[1], row).unwrap(), 0.0);
    }
    let notes = |m: &cat_smf_editor::model::Music| {
        m.tracks[1]
            .events
            .iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (e.tick, e.delta, e.status, e.meta_type, e.data.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(notes(&loaded.music), notes(&before));
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(&loaded.music).unwrap())
            .unwrap();
    let saved = cat_smf_editor::instrument_layers::parse(&restored.tracks[1]).unwrap();
    assert_eq!(
        saved.layers.iter().find(|l| l.id == 1).unwrap().volume_db,
        -9.5
    );
    let encode = |m: &cat_smf_editor::model::Music| cat_smf_editor::smf_io::encode(m).unwrap();
    let changed = encode(&loaded.music);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        assert!(values::set_level(&mut app, 0, invalid).is_err());
        assert_eq!(encode(&app.loaded.as_ref().unwrap().music), changed);
    }
    app.undo_music().unwrap();
    assert_eq!(encode(&app.loaded.as_ref().unwrap().music), encode(&before));
}
