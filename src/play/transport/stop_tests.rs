use super::{tests::fixture, *};
use eframe::egui;

pub(super) fn press(
    app: &mut App,
    ctx: &egui::Context,
    key: egui::Key,
    modifiers: egui::Modifiers,
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events: vec![false, true]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                })
                .collect(),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

pub(super) fn stop_with_shift_space(app: &mut App, ctx: &egui::Context) {
    press(app, ctx, egui::Key::Space, egui::Modifiers::SHIFT);
}

#[test]
fn shift_space_stops_pending_and_active_playback_through_overlays() {
    for overlay in ["main", "curve", "roll", "play", "mixer", "help", "menu"] {
        for active in [false, true] {
            let mut app = fixture();
            let ctx = egui::Context::default();
            playback::start(&mut app, Mode::AllTracks);
            if active {
                advance(&mut app);
                app.audition.set_test_seconds(0.5);
                assert!(app.playback.active.is_some());
            } else {
                assert!(app.playback.pending.is_some());
            }
            app.playback.repeat = Some((0, 960));
            app.tempo_curve.open = overlay == "curve";
            app.piano_roll.open = overlay == "roll";
            app.playback.open = overlay == "play";
            app.playback.mixer_open = overlay == "mixer";
            app.help_open = overlay == "help";
            app.comma_menu.open = overlay == "menu";
            stop_with_shift_space(&mut app, &ctx);
            assert!(app.playback.pending.is_none(), "{overlay}");
            assert!(app.playback.active.is_none(), "{overlay}");
            assert!(app.playback.repeat.is_none(), "{overlay}");
            assert!(app.audition.seconds().is_none(), "{overlay}");
            assert_eq!(app.tempo_curve.open, overlay == "curve");
            assert_eq!(app.piano_roll.open, overlay == "roll");
            assert_eq!(app.playback.open, overlay == "play");
            assert_eq!(app.playback.mixer_open, overlay == "mixer");
            assert_eq!(app.help_open, overlay == "help");
            assert_eq!(app.comma_menu.open, overlay == "menu");
            assert_eq!(app.playback.last, Mode::AllTracks);
            advance(&mut app);
            assert!(
                app.playback.pending.is_none(),
                "stopped repeat must not restart"
            );
        }
    }
}

#[test]
fn pd_stops_playback_and_closes_only_the_play_overlay() {
    for curve in [false, true] {
        let mut app = fixture();
        let ctx = egui::Context::default();
        playback::start(&mut app, Mode::AllTracks);
        advance(&mut app);
        app.audition.set_test_seconds(0.5);
        app.playback.repeat = Some((0, 960));
        app.tempo_curve.open = curve;
        press(&mut app, &ctx, egui::Key::P, egui::Modifiers::NONE);
        assert!(app.playback.open);
        press(&mut app, &ctx, egui::Key::D, egui::Modifiers::NONE);
        assert!(!app.playback.open);
        assert_eq!(app.tempo_curve.open, curve);
        assert!(app.playback.active.is_none());
        assert!(app.playback.pending.is_none());
        assert!(app.playback.repeat.is_none());
        assert!(app.audition.seconds().is_none());
        assert_eq!(app.playback.last, Mode::AllTracks);
    }
}

#[test]
fn esc_closes_curve_and_play_overlay_without_stopping_or_clearing_repeat() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    playback::start(&mut app, Mode::AllTracks);
    advance(&mut app);
    app.audition.set_test_seconds(0.5);
    app.playback.repeat = Some((0, 960));
    app.tempo_curve.open = true;
    app.playback.open = true;
    press(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!app.playback.open);
    assert!(app.tempo_curve.open);
    assert!(app.playback.active.is_some());
    press(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!app.tempo_curve.open);
    assert!(app.playback.active.is_some());
    assert_eq!(app.playback.repeat, Some((0, 960)));
    assert_eq!(app.audition.seconds(), Some(0.5));
    press(&mut app, &ctx, egui::Key::Space, egui::Modifiers::NONE);
    assert!(app.playback.pending.is_some());
    assert!(app.playback.repeat.is_none());
}
