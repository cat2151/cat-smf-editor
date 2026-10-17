use crate::{
    cursor::split::{
        self,
        tests::{fixture, press, w_then},
    },
    view::tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

#[test]
fn wh_on_wl_wh_focus_and_ww_off_keeps_active_track() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    split::go(&mut app, 0, 480);
    w_then(&mut app, &ctx, egui::Key::H);
    assert_eq!((app.track_view.split, app.track), (Some([0, 1]), 0));
    w_then(&mut app, &ctx, egui::Key::H);
    assert_eq!((app.track_view.split, app.track), (Some([0, 1]), 0));
    w_then(&mut app, &ctx, egui::Key::L);
    assert_eq!((app.track_view.split, app.track), (Some([0, 1]), 1));
    assert_eq!(crate::play::playback::location(&app).0, 480);
    w_then(&mut app, &ctx, egui::Key::L);
    assert_eq!(app.track, 1);
    w_then(&mut app, &ctx, egui::Key::H);
    assert_eq!((app.track_view.split, app.track), (Some([0, 1]), 0));
    w_then(&mut app, &ctx, egui::Key::L);
    // The overlay keys must not also move the cursor or open other overlays.
    assert_eq!(crate::play::playback::location(&app).0, 480);
    w_then(&mut app, &ctx, egui::Key::W);
    assert_eq!((app.track_view.split, app.track), (None, 1));
    assert_eq!(crate::play::playback::location(&app).0, 480);
    w_then(&mut app, &ctx, egui::Key::W);
    assert_eq!((app.track_view.split, app.track), (None, 1));
    w_then(&mut app, &ctx, egui::Key::Escape);
    assert_eq!(app.track_view.split, None);
}

#[test]
fn wl_and_wh_from_one_column_turn_on_two_columns_then_focus() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    split::go(&mut app, 2, 480);
    w_then(&mut app, &ctx, egui::Key::L);
    assert_eq!((app.track_view.split, app.track), (Some([2, 3]), 3));
    assert_eq!(crate::play::playback::location(&app).0, 480);
    w_then(&mut app, &ctx, egui::Key::W);
    w_then(&mut app, &ctx, egui::Key::H);
    assert_eq!((app.track_view.split, app.track), (Some([3, 0]), 3));
}

#[test]
fn wj_and_wk_step_only_the_focused_column_and_turn_on_two_columns_first() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    split::go(&mut app, 1, 480);
    w_then(&mut app, &ctx, egui::Key::J);
    assert_eq!((app.track_view.split, app.track), (Some([2, 2]), 2));
    w_then(&mut app, &ctx, egui::Key::L);
    w_then(&mut app, &ctx, egui::Key::J);
    assert_eq!((app.track_view.split, app.track), (Some([2, 3]), 3));
    w_then(&mut app, &ctx, egui::Key::J);
    assert_eq!((app.track_view.split, app.track), (Some([2, 0]), 0));
    w_then(&mut app, &ctx, egui::Key::K);
    assert_eq!((app.track_view.split, app.track), (Some([2, 3]), 3));
    // The overlay keys must not also move the cursor.
    assert_eq!(crate::play::playback::location(&app).0, 480);
}

#[test]
fn split_wraps_last_track_and_empty_destination_is_safe() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    app.select_track(3);
    w_then(&mut app, &ctx, egui::Key::H);
    assert_eq!(app.track_view.split, Some([3, 0]));
    app.loaded.as_mut().unwrap().music.tracks[0].events.clear();
    app.loaded.as_mut().unwrap().refresh();
    split::go(&mut app, 0, 960);
    press(&mut app, &ctx, &[]);
    assert_eq!(app.cursor_row, None);
    assert_eq!(crate::play::playback::location(&app).0, 960);
}

/// The whole row inside the outlined pane selects, not only its text columns.
#[test]
fn clicking_the_right_side_of_a_row_in_either_column_selects_it() {
    let note = |tick, status, velocity| Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![60, velocity],
    };
    let track = || Track {
        events: vec![note(480, 0x90, 100), note(960, 0x80, 0)],
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![track(), track()],
    });
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1900.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    w_then(&mut app, &ctx, egui::Key::H);
    for (slot, x) in [(1, 1600.0), (0, 800.0)] {
        let output = render(&mut app, vec![]);
        let y = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == "C4" => Some(t.pos.y + 5.0),
                _ => None,
            })
            .unwrap();
        let position = egui::pos2(x, y);
        let button = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        render(
            &mut app,
            vec![egui::Event::PointerMoved(position), button(true)],
        );
        render(&mut app, vec![button(false)]);
        assert_eq!(
            (app.track_view.active, app.cursor_row, app.selected_event),
            (slot, Some(1), Some(0)),
            "click at x = {x}"
        );
    }
}
