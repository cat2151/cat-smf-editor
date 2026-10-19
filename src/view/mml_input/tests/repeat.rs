use super::*;
use crate::play::transport;
use cat_smf_editor::model::Event;

const RANGE: (u64, u64) = (480, 1920);

fn playing() -> (App, egui::Context) {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: [48, 55]
            .into_iter()
            .map(|pitch| Track {
                events: [(0, 0x90, 80), (3840, 0x80, 0)]
                    .into_iter()
                    .map(|(tick, status, velocity)| Event {
                        tick,
                        delta: 0,
                        status,
                        meta_type: 0,
                        data: vec![pitch, velocity],
                    })
                    .collect(),
            })
            .collect(),
    });
    app.track_view.cursor_tick = Some(480);
    app.playback.follow = true;
    transport::repeat::start(&mut app, RANGE);
    transport::advance(&mut app);
    app.audition.set_test_seconds(0.25);
    (app, egui::Context::default())
}

fn assert_loop(app: &App) {
    assert_eq!(app.playback.repeat, Some(RANGE));
    let active = app
        .playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(app.playback.active.as_ref())
        .expect("repeat remains running");
    assert_eq!(active.sources.len(), 2);
    assert!((active.start_seconds + active.end - 2.0).abs() < 1e-9);
    assert!(app.audition.auditioned.is_empty(), "no cursor audition");
    assert!(app.playback.follow);
}

fn next_loop(app: &mut App, ctx: &egui::Context) {
    frame(app, ctx, vec![]);
    let end = app.playback.active.as_ref().unwrap().end;
    app.audition.set_test_seconds(end + 0.01);
    frame(app, ctx, vec![]);
    assert_eq!(
        app.playback.pending.as_ref().unwrap().active.start_seconds,
        0.5
    );
    frame(app, ctx, vec![]);
    assert_loop(app);
}

#[test]
fn i_keeps_clock_and_range_and_typing_refreshes_both_mml_and_chords_in_the_loop() {
    for input in ["o5 c1", "C F G"] {
        let (mut app, ctx) = playing();
        let before = transport::tick(&app);
        open_input(&mut app, &ctx);
        assert_eq!(transport::tick(&app), before);
        assert_eq!(app.mml_input.tick, 480);
        assert_loop(&app);
        next_loop(&mut app, &ctx);
        app.audition.set_test_seconds(0.25);
        frame(&mut app, &ctx, vec![egui::Event::Text(input.into())]);
        assert_loop(&app);
        let prepared = app.playback.pending.as_ref().unwrap();
        assert!((prepared.active.start_seconds - 0.75).abs() < 1e-9);
        assert!(prepared.parts[0].2.notes.iter().any(|n| n.pitch >= 60));
        assert!(prepared.parts[1].2.notes.iter().any(|n| n.pitch == 55));
        frame(&mut app, &ctx, vec![key(egui::Key::ArrowLeft)]);
        assert_loop(&app);
        next_loop(&mut app, &ctx);
        frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.mml_input.open);
        assert!(app.undo.available());
        assert_loop(&app);
        next_loop(&mut app, &ctx);
    }
}

#[test]
fn cancel_restores_song_and_cursor_and_keeps_looping() {
    let (mut app, ctx) = playing();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c1".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(!app.mml_input.open);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert_eq!(crate::play::playback::location(&app).0, 480);
    assert!(!app.undo.available());
    assert_loop(&app);
    next_loop(&mut app, &ctx);
}

#[test]
fn empty_input_and_failed_confirmation_keep_the_original_loop() {
    let (mut app, ctx) = playing();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
    assert!(app.mml_input.buffer.is_empty());
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![]);
    assert!(app.mml_input.open && app.mml_input.error.is_some());
    assert_loop(&app);
    next_loop(&mut app, &ctx);
}

#[test]
fn shift_space_stops_inside_editor_and_confirm_or_cancel_never_restarts_it() {
    for close in [egui::Key::Enter, egui::Key::Escape] {
        let (mut app, ctx) = playing();
        open_input(&mut app, &ctx);
        let mut stop = key(egui::Key::Space);
        if let egui::Event::Key { modifiers, .. } = &mut stop {
            *modifiers = egui::Modifiers::SHIFT;
        }
        frame(&mut app, &ctx, vec![stop]);
        assert!(app.mml_input.open);
        assert!(app.playback.repeat.is_none());
        assert!(!transport::timeline::running(&app));
        frame(&mut app, &ctx, vec![egui::Event::Text("c1".into())]);
        frame(&mut app, &ctx, vec![key(close)]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.mml_input.open);
        assert!(!transport::timeline::running(&app));
        assert!(app.playback.repeat.is_none());
    }
}
