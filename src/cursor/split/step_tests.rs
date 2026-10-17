use crate::cursor::split::{self, tests::fixture, tests::press};
use eframe::egui;

#[test]
fn ctrl_left_steps_forward_and_ctrl_right_steps_back_with_wrap() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let ctrl = egui::Modifiers::CTRL | egui::Modifiers::COMMAND;
    split::go(&mut app, 0, 600);
    let mut visited = Vec::new();
    for _ in 0..4 {
        press(&mut app, &ctx, &[(egui::Key::ArrowLeft, ctrl)]);
        visited.push(app.track);
        assert_eq!(crate::play::playback::location(&app).0, 600);
    }
    assert_eq!(visited, [1, 2, 3, 0]);
    visited.clear();
    for _ in 0..4 {
        press(&mut app, &ctx, &[(egui::Key::ArrowRight, ctrl)]);
        visited.push(app.track);
    }
    assert_eq!(visited, [3, 2, 1, 0]);
    // Split view replaces the active pane, as gt does.
    app.track_view.split = Some([0, 1]);
    press(&mut app, &ctx, &[(egui::Key::ArrowRight, ctrl)]);
    assert_eq!((app.track_view.split, app.track), (Some([3, 1]), 3));
}

#[test]
fn gtn_and_gtp_step_tracks_with_wrap_at_the_time_before_g() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let none = egui::Modifiers::NONE;
    for (start, key, target) in [
        (0, egui::Key::N, 1),
        (3, egui::Key::N, 0),
        (0, egui::Key::P, 3),
        (2, egui::Key::P, 1),
    ] {
        split::go(&mut app, start, 600);
        press(
            &mut app,
            &ctx,
            &[(egui::Key::G, none), (egui::Key::T, none), (key, none)],
        );
        assert_eq!(app.track, target, "{start} {key:?}");
        assert_eq!(crate::play::playback::location(&app).0, 600);
        assert!(!app.track_view.go_open);
    }
}

#[test]
fn ctrl_steps_change_only_the_active_pane() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let ctrl = egui::Modifiers::CTRL | egui::Modifiers::COMMAND;
    split::go(&mut app, 0, 600);
    app.track_view.split = Some([0, 1]);
    let mut seen = Vec::new();
    for _ in 0..3 {
        press(&mut app, &ctx, &[(egui::Key::ArrowLeft, ctrl)]);
        seen.push((app.track_view.split, app.track));
    }
    // Landing on the other pane's track shows it in both panes.
    assert_eq!(
        seen,
        [(Some([1, 1]), 1), (Some([2, 1]), 2), (Some([3, 1]), 3)]
    );
    split::focus(&mut app, 1);
    press(&mut app, &ctx, &[(egui::Key::ArrowRight, ctrl)]);
    assert_eq!((app.track_view.split, app.track), (Some([3, 0]), 0));
    // gtn / gtp share the rule.
    let none = egui::Modifiers::NONE;
    press(
        &mut app,
        &ctx,
        &[
            (egui::Key::G, none),
            (egui::Key::T, none),
            (egui::Key::N, none),
        ],
    );
    assert_eq!((app.track_view.split, app.track), (Some([3, 1]), 1));
}

#[test]
fn single_track_still_opens_two_columns_with_the_same_track() {
    let mut app = crate::cursor::split::tests::tracks(1);
    let ctx = egui::Context::default();
    let none = egui::Modifiers::NONE;
    press(
        &mut app,
        &ctx,
        &[(egui::Key::W, none), (egui::Key::L, none)],
    );
    assert_eq!(
        (app.track_view.split, app.track_view.active, app.track),
        (Some([0, 0]), 1, 0)
    );
    press(
        &mut app,
        &ctx,
        &[(egui::Key::W, none), (egui::Key::H, none)],
    );
    assert_eq!(
        (app.track_view.split, app.track_view.active),
        (Some([0, 0]), 0)
    );
}
