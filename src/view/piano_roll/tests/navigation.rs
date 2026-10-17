use super::*;

fn button(pos: egui::Pos2, pressed: bool, secondary: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: if secondary {
            egui::PointerButton::Secondary
        } else {
            egui::PointerButton::Primary
        },
        modifiers: egui::Modifiers::NONE,
    }
}

pub(super) fn click(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    pos: egui::Pos2,
    secondary: bool,
) {
    at(app, ctx, time, vec![egui::Event::PointerMoved(pos)]);
    at(app, ctx, time + 0.01, vec![button(pos, true, secondary)]);
    at(app, ctx, time + 0.02, vec![button(pos, false, secondary)]);
    at(app, ctx, time + 0.03, vec![]);
}

pub(super) fn wheel(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    pos: egui::Pos2,
    delta: f32,
) -> egui::FullOutput {
    at(
        app,
        ctx,
        time,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, delta),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for i in 1..10 {
        at(app, ctx, time + f64::from(i) * 0.05, vec![]);
    }
    at(app, ctx, time + 0.5, vec![])
}

pub(super) fn zone(app: &crate::app::App, axis: usize) -> egui::Pos2 {
    let panel = app.piano_roll.viewport.panel.unwrap();
    if axis == 0 {
        egui::pos2(
            panel.center().x,
            app.piano_roll.viewport.zoom_bars[0].unwrap().center().y,
        )
    } else {
        egui::pos2(
            app.piano_roll.viewport.zoom_bars[1].unwrap().center().x,
            panel.center().y,
        )
    }
}

#[test]
fn zoom_clicks_scale_the_selected_axis_but_content_wheel_scrolls_time() {
    for axis in 0..2 {
        let mut app = arpeggio();
        let (ctx, output) = opened_in(&mut app);
        let before = all_notes(&output)[2];
        let pos = zone(&app, axis);
        click(&mut app, &ctx, 11.0, pos, false);
        let output = at(&mut app, &ctx, 11.1, vec![]);
        let after = all_notes(&output)[2];
        assert!((after.size()[axis] - before.size()[axis] * 1.5).abs() < 0.01);
        assert_eq!(after.size()[1 - axis], before.size()[1 - axis]);
        let pos = zone(&app, axis);
        click(&mut app, &ctx, 12.0, pos, true);
        let output = at(&mut app, &ctx, 12.1, vec![]);
        let restored = all_notes(&output)[2];
        assert!((restored.size()[axis] - after.size()[axis] * (2.0 / 3.0)).abs() < 0.01);
        assert_eq!(restored.size()[1 - axis], before.size()[1 - axis]);
        let center = app.piano_roll.viewport.panel.unwrap().center();
        let offset = app.piano_roll.viewport.scroll;
        let output = wheel(&mut app, &ctx, 13.0, center, -20.0);
        let next = all_notes(&output)[2];
        assert_eq!(next.size(), restored.size());
        assert!(app.piano_roll.viewport.scroll.x > offset.x);
        assert_eq!(app.piano_roll.viewport.scroll.y, offset.y);
        assert_eq!(next.size()[1 - axis], before.size()[1 - axis]);
        let pos = zone(&app, axis);
        let output = wheel(&mut app, &ctx, 14.0, pos, 20.0);
        assert!(all_notes(&output)[2].size()[axis] > next.size()[axis]);
    }
}

#[test]
fn range_zoom_works_in_both_directions_on_either_axis() {
    for axis in 0..2 {
        for reverse in [false, true] {
            let mut app = with_notes(&[(0, 480, 24), (1920, 480, 108)]);
            let (ctx, output) = opened_in(&mut app);
            let before = all_notes(&output)[0];
            let panel = app.piano_roll.viewport.panel.unwrap();
            let mut from = zone(&app, axis);
            let mut to = from;
            from[axis] = panel.min[axis] + panel.size()[axis] * 0.2;
            to[axis] = panel.min[axis] + panel.size()[axis] * 0.6;
            if reverse {
                std::mem::swap(&mut from, &mut to);
            }
            super::drag::drag(&mut app, &ctx, 11.0, from, to);
            let output = at(&mut app, &ctx, 12.0, vec![]);
            let after = all_notes(&output)[0];
            assert!((after.size()[axis] / before.size()[axis] - 2.25).abs() < 0.01);
            assert_eq!(after.size()[1 - axis], before.size()[1 - axis]);
            assert_eq!(
                super::drag::notes_of(&app),
                vec![(0, 480, 24), (1920, 480, 108)]
            );
        }
    }
}

#[test]
fn page_buttons_scroll_the_selected_axis_but_content_wheel_scrolls_time() {
    for axis in 0..2 {
        let mut app = with_notes(&[(0, 480, 0), (1920, 480, 127)]);
        let (ctx, _) = opened_in(&mut app);
        let panel = app.piano_roll.viewport.panel.unwrap();
        let bar = app.piano_roll.viewport.bars[axis].unwrap();
        let mut pos = bar.center();
        pos[axis] = bar.max[axis] - 8.0;
        click(&mut app, &ctx, 11.0, pos, false);
        let offset = app.piano_roll.viewport.scroll;
        assert!(
            (offset[axis] - panel.size()[axis] * 0.5).abs() < 0.1,
            "axis {axis}: {offset:?}, panel {panel:?}, bar {bar:?}"
        );
        assert_eq!(offset[1 - axis], 0.0);
        wheel(&mut app, &ctx, 12.0, panel.center(), -40.0);
        let next = app.piano_roll.viewport.scroll;
        assert!(next.x > offset.x);
        assert_eq!(next.y, offset.y);
    }
}

#[test]
fn note_drag_uses_zoomed_tick_scale_after_scrolling() {
    let mut app = arpeggio();
    crate::cursor::split::go(&mut app, 0, 3840);
    let (ctx, _) = opened_in(&mut app);
    let mut pos = zone(&app, 0);
    pos.x = app.piano_roll.viewport.panel.unwrap().left() + 100.0;
    click(&mut app, &ctx, 11.0, pos, false);
    let output = at(&mut app, &ctx, 11.1, vec![]);
    let note = all_notes(&output)[4];
    let from = note.center();
    super::drag::drag(&mut app, &ctx, 12.0, from, from + egui::vec2(24.0, 0.0));
    assert_eq!(super::drag::notes_of(&app)[4], (3920, 960, 60));
}

#[test]
fn scrollbar_drag_and_hovered_wheel_override_the_previous_zoom_mode() {
    for axis in 0..2 {
        let mut app = with_notes(&[(0, 480, 0), (1920, 480, 127)]);
        let (ctx, _) = opened_in(&mut app);
        let pos = zone(&app, 1 - axis);
        click(&mut app, &ctx, 11.0, pos, false);
        let bar = app.piano_roll.viewport.bars[axis].unwrap();
        let mut from = bar.center();
        from[axis] = bar.min[axis] + 45.0;
        let mut to = from;
        to[axis] += 60.0;
        let old = app.piano_roll.viewport.scroll;
        super::drag::drag(&mut app, &ctx, 12.0, from, to);
        assert!(app.piano_roll.viewport.scroll[axis] > old[axis]);
        let output = at(&mut app, &ctx, 13.0, vec![]);
        let size = all_notes(&output)[0].size();
        let offset = app.piano_roll.viewport.scroll;
        let panel = app.piano_roll.viewport.panel.unwrap();
        let output = wheel(&mut app, &ctx, 14.0, panel.center(), -40.0);
        assert!(app.piano_roll.viewport.scroll.x > offset.x);
        assert_eq!(app.piano_roll.viewport.scroll.y, offset.y);
        assert_eq!(all_notes(&output)[0].size(), size);
        let pos = zone(&app, 1 - axis);
        click(&mut app, &ctx, 15.0, pos, false);
        let output = at(&mut app, &ctx, 15.1, vec![]);
        let size = all_notes(&output)[0].size();
        let bar = app.piano_roll.viewport.bars[axis].unwrap();
        let output = wheel(&mut app, &ctx, 16.0, bar.center(), -40.0);
        assert_eq!(all_notes(&output)[0].size(), size);
    }
}
