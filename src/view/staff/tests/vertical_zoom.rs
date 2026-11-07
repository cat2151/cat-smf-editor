//! Vertical zoom keeps the staff area and decides whether the staff scrolls.
use super::note;
use crate::view::tests::app;
use cat_smf_editor::model::{Music, Track};
use eframe::egui;

/// Notes far above and below the staves, so the staff needs ledger room.
fn wide_range() -> crate::app::App {
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: [20u8, 110, 40, 90]
                    .iter()
                    .enumerate()
                    .flat_map(|(i, &pitch)| note(i as u64 * 480, pitch, 240))
                    .collect(),
            },
        ],
    });
    app.track = 1;
    app.playback.follow = false;
    app
}

fn vertical_zoom(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    button: egui::PointerButton,
) {
    use crate::view::piano_roll::tests::at;
    let pos = app.staff.viewport.zoom_bars[1]
        .expect("vertical zoom bar")
        .center();
    let event = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(app, ctx, time + 0.01, vec![event(true)]);
    at(app, ctx, time + 0.02, vec![event(false)]);
    at(app, ctx, time + 0.03, vec![]);
    at(app, ctx, time + 0.5, vec![]);
}

/// Scrolls down over the vertical scrollbar and returns the vertical offset it reached.
fn scrolled_down(app: &mut crate::app::App, ctx: &egui::Context, time: f64) -> f32 {
    use crate::view::piano_roll::tests::at;
    let pos = app.staff.viewport.bars[1]
        .expect("vertical scrollbar")
        .center();
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(
        app,
        ctx,
        time + 0.01,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            modifiers: egui::Modifiers::NONE,
        }],
    );
    for i in 2..40 {
        at(app, ctx, time + f64::from(i) * 0.05, vec![]);
    }
    app.staff.viewport.scroll.y
}

#[test]
fn vertical_zoom_keeps_the_staff_area_and_zooming_out_fits_the_whole_staff() {
    use crate::view::piano_roll::tests::at;
    let mut app = wide_range();
    let ctx = egui::Context::default();
    at(&mut app, &ctx, 10.0, vec![]);
    at(&mut app, &ctx, 10.1, vec![]);
    let height = app.staff.viewport.panel.expect("panel").height();
    assert_eq!(
        scrolled_down(&mut app, &ctx, 11.0),
        0.0,
        "natural size fits"
    );
    for i in 0..3 {
        vertical_zoom(
            &mut app,
            &ctx,
            20.0 + f64::from(i),
            egui::PointerButton::Secondary,
        );
        assert_eq!(app.staff.viewport.panel.unwrap().height(), height);
    }
    assert_eq!(scrolled_down(&mut app, &ctx, 30.0), 0.0, "zoomed out fits");
    for i in 0..6 {
        vertical_zoom(
            &mut app,
            &ctx,
            40.0 + f64::from(i),
            egui::PointerButton::Primary,
        );
        assert_eq!(app.staff.viewport.panel.unwrap().height(), height);
    }
    assert!(
        scrolled_down(&mut app, &ctx, 50.0) > 0.0,
        "zoomed in scrolls"
    );
}
