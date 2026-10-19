use super::{fixture, key};
use crate::{
    cursor::moves,
    play::{playback, transport},
};
use eframe::egui;

fn render(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    pressed: Option<egui::Key>,
) -> egui::FullOutput {
    render_events(app, ctx, pressed.into_iter().map(key).collect())
}

fn render_events(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 400.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

#[test]
fn stop_returns_the_view_to_the_edit_cursor_after_following_playback() {
    for split in [false, true] {
        for cursor in [None, Some(0), Some(10)] {
            for method in ["shortcut", "menu", "stop"] {
                let mut app = fixture();
                app.loaded.as_mut().unwrap().music.tracks[0]
                    .events
                    .last_mut()
                    .unwrap()
                    .tick = 192_000;
                app.loaded.as_mut().unwrap().refresh();
                app.track_view.split = split.then_some([0, 0]);
                if let Some(row) = cursor {
                    moves::place(&mut app, row);
                }
                let location = playback::location(&app);
                let ctx = egui::Context::default();
                render(&mut app, &ctx, None);
                playback::start(&mut app, playback::Mode::Track);
                transport::advance(&mut app);
                app.audition.set_test_seconds(80.0);
                render(&mut app, &ctx, None);
                assert!(!cursor_visible(&render(&mut app, &ctx, None)));
                let output = match method {
                    "shortcut" => render_events(
                        &mut app,
                        &ctx,
                        vec![egui::Event::Key {
                            key: egui::Key::Space,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::SHIFT,
                        }],
                    ),
                    "menu" => {
                        render(&mut app, &ctx, Some(egui::Key::P));
                        render(&mut app, &ctx, Some(egui::Key::D))
                    }
                    _ => {
                        transport::stop(&mut app);
                        render(&mut app, &ctx, None)
                    }
                };
                assert!(app.playback.active.is_none());
                assert!(app.playback.pending.is_none());
                assert_eq!(playback::location(&app), location);
                assert!(
                    cursor_visible(&output),
                    "{method}, split={split}, cursor={cursor:?}"
                );
                assert!(cursor_visible(&render(&mut app, &ctx, None)));
            }
        }
    }
}

fn cursor_visible(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Rect(rect)
            if rect.stroke.color == egui::Color32::LIGHT_BLUE
                && rect.rect.intersect(shape.clip_rect).is_positive())
    })
}

#[test]
fn row_navigation_reveals_unchanged_cursor_after_playback_follow() {
    for note_mode in [true, false] {
        for (end, pressed) in [
            (false, egui::Key::Home),
            (false, egui::Key::ArrowUp),
            (false, egui::Key::K),
            (false, egui::Key::PageUp),
            (true, egui::Key::End),
            (true, egui::Key::ArrowDown),
            (true, egui::Key::J),
            (true, egui::Key::PageDown),
        ] {
            let mut app = fixture();
            app.note_mode = note_mode;
            app.loaded.as_mut().unwrap().music.tracks[0]
                .events
                .last_mut()
                .unwrap()
                .tick = 192_000;
            app.loaded.as_mut().unwrap().refresh();
            let row = if end {
                crate::cursor::split::row_count(&app, 0) - 1
            } else {
                0
            };
            moves::place(&mut app, row);
            let ctx = egui::Context::default();
            render(&mut app, &ctx, None);
            playback::start(&mut app, playback::Mode::Track);
            transport::advance(&mut app);
            app.audition.set_test_seconds(80.0);
            render(&mut app, &ctx, None);
            assert_eq!(app.cursor_row, Some(row));
            // Raw mode has only two rows, both can remain visible.
            if note_mode {
                assert!(!cursor_visible(&render(&mut app, &ctx, None)));
            }
            let output = render(&mut app, &ctx, Some(pressed));
            assert_eq!(app.cursor_row, Some(row));
            assert!(app.playback.pending.is_none());
            assert!(app.playback.active.is_none());
            assert!(
                cursor_visible(&output),
                "destination cursor was not painted"
            );
            assert!(cursor_visible(&render(&mut app, &ctx, None)));
        }
    }
}

#[test]
fn stop_preserves_implicit_tick_zero_in_raw_mode_with_a_delayed_first_event() {
    let mut app = fixture();
    app.note_mode = false;
    let location = playback::location(&app);
    assert_eq!(location.0, 0);
    transport::stop(&mut app);
    assert_eq!(playback::location(&app), location);
    assert!(cursor_visible(&render(
        &mut app,
        &egui::Context::default(),
        None
    )));
}

#[test]
fn beat_navigation_at_an_edge_reveals_the_cursor_and_stops_following() {
    let mut app = fixture();
    moves::place(&mut app, 0);
    playback::start(&mut app, playback::Mode::Track);
    transport::advance(&mut app);
    app.audition.set_test_seconds(20.0);
    let ctx = egui::Context::default();
    render(&mut app, &ctx, None);
    moves::boundary(&mut app, false, false);
    assert!(app.playback.active.is_none());
    assert!(app.reveal_cursor);
    assert_eq!(app.cursor_row, Some(0));
    assert!(cursor_visible(&render(&mut app, &ctx, None)));
}

#[test]
fn home_and_end_clear_exact_tick_within_the_same_row() {
    for end in [false, true] {
        let mut app = fixture();
        let tick = if end { 48_001 } else { 1 };
        moves::at_tick(&mut app, tick);
        let row = app.cursor_row;
        let ctx = egui::Context::default();
        render(
            &mut app,
            &ctx,
            Some(if end { egui::Key::End } else { egui::Key::Home }),
        );
        assert_eq!(app.cursor_row, row);
        assert_eq!(app.track_view.cursor_tick, None);
        assert_eq!(app.cursor_position, None);
        assert_ne!(playback::location(&app).0, tick);
    }
}
