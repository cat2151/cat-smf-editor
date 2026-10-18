use super::{edit::click, music, opened_in, squares};
use crate::view::{piano_roll::tests::at, tests::app};
use eframe::egui;

pub(super) fn wheel(app: &mut crate::app::App, ctx: &egui::Context, time: f64) {
    let center = app.tempo_curve.viewport.tempo_rect.unwrap().center();
    at(
        app,
        ctx,
        time,
        vec![
            egui::Event::PointerMoved(center),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -40.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for i in 1..=10 {
        at(app, ctx, time + f64::from(i) * 0.05, vec![]);
    }
}

#[test]
fn content_wheel_scrolls_time_after_zoom_buttons_and_both_scrollbars() {
    for mode in 0..4 {
        let mut song = music();
        for track in &mut song.tracks {
            track.events.last_mut().unwrap().tick = 192_000;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(song);
        let (ctx, output) = opened_in(&mut app, 0);
        if mode == 3 {
            // A horizontal scrollbar must replace a previous zoom action.
            app.tempo_curve.viewport.zoom_edge(
                0,
                true,
                super::super::Axis {
                    low: 120.0,
                    high: 150.0,
                },
                100.0,
            );
            at(&mut app, &ctx, 10.5, vec![]);
        }
        let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
        let zoom_bars = squares(&output, super::super::viewport::BAR_COLOR);
        let pos = match mode {
            0 => zoom_bars[0].0.center(),
            1 => zoom_bars[1].0.center(),
            2 => egui::pos2(panel.right() + 20.0, panel.bottom() - 10.0),
            _ => egui::pos2(panel.center().x, panel.bottom() + 14.0),
        };
        click(&mut app, &ctx, 11.0, pos, egui::PointerButton::Primary);
        let before_axis = app.tempo_curve.viewport.axis;
        let before_scroll = app.tempo_curve.viewport.scroll;
        let before = super::circles(&at(&mut app, &ctx, 11.5, vec![]), super::TEMPO);
        let span = before[1].0.x - before[0].0.x;
        wheel(&mut app, &ctx, 12.0);
        let after = super::circles(&at(&mut app, &ctx, 13.0, vec![]), super::TEMPO);
        let next_span = after[1].0.x - after[0].0.x;
        assert!(app.tempo_curve.viewport.scroll > before_scroll);
        assert_eq!(app.tempo_curve.viewport.axis, before_axis);
        assert!((next_span - span).abs() < 0.01);
    }
}

#[test]
fn page_buttons_move_half_a_viewport_but_content_wheel_scrolls_time() {
    for vertical in [false, true] {
        for forward in [false, true] {
            let mut song = music();
            for track in &mut song.tracks {
                track.events.last_mut().unwrap().tick = 192_000;
                cat_smf_editor::edit::refresh_deltas(track).unwrap();
            }
            let mut app = app(song);
            let (ctx, _) = opened_in(&mut app, 0);
            let base = super::super::Axis {
                low: 120.0,
                high: 150.0,
            };
            app.tempo_curve.viewport.zoom_edge(0, true, base, 200.0);
            app.tempo_curve.viewport.zoom_edge(2, true, base, 0.5);
            app.tempo_curve.viewport.pending_scroll = Some(2000.0);
            at(&mut app, &ctx, 10.5, vec![]);
            let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
            let pos = if vertical {
                egui::pos2(
                    panel.right() + 17.0,
                    if forward {
                        panel.bottom() - 8.0
                    } else {
                        panel.top() + 8.0
                    },
                )
            } else {
                egui::pos2(
                    if forward { panel.right() } else { panel.left() },
                    panel.bottom() + 14.0,
                )
            };
            let before_scroll = app.tempo_curve.viewport.scroll;
            let width = app.tempo_curve.viewport.width;
            let before_axis = app.tempo_curve.viewport.axis.unwrap();
            let direction = if forward { 1.0 } else { -1.0 };
            click(&mut app, &ctx, 11.0, pos, egui::PointerButton::Primary);
            let axis = app.tempo_curve.viewport.axis.unwrap();
            let scroll = app.tempo_curve.viewport.scroll;
            if vertical {
                let span = before_axis.high - before_axis.low;
                assert!((axis.low - (before_axis.low - direction * span * 0.5)).abs() < 1e-6);
                assert!((axis.high - axis.low - span).abs() < 1e-6);
                assert_eq!(scroll, before_scroll);
            } else {
                assert!(
                    (f64::from(scroll - before_scroll) - direction * f64::from(width) * 0.5).abs()
                        < 0.01
                );
                assert_eq!(axis, before_axis);
            }
            wheel(&mut app, &ctx, 12.0);
            let next = app.tempo_curve.viewport.axis.unwrap();
            assert!(app.tempo_curve.viewport.scroll > scroll);
            assert_eq!(next, axis);
        }
    }
}

#[test]
fn wheel_over_a_bar_uses_its_operation_then_content_wheel_scrolls_time() {
    for mode in 0..4 {
        let mut song = music();
        for track in &mut song.tracks {
            track.events.last_mut().unwrap().tick = 192_000;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(song);
        let (ctx, output) = opened_in(&mut app, 0);
        let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
        let bars = squares(&output, super::super::viewport::BAR_COLOR);
        let pos = match mode {
            0 => bars[0].0.center(),
            1 => bars[1].0.center(),
            2 => egui::pos2(panel.right() + 17.0, panel.center().y),
            _ => egui::pos2(panel.center().x, panel.bottom() + 14.0),
        };
        // Start with a different operation so scrolling a bar must replace it.
        app.tempo_curve.viewport.zoom_edge(
            if mode == 0 { 2 } else { 0 },
            true,
            super::super::Axis {
                low: 120.0,
                high: 150.0,
            },
            if mode == 0 { 0.5 } else { 200.0 },
        );
        app.tempo_curve.viewport.pending_scroll = Some(2000.0);
        let before = at(&mut app, &ctx, 10.5, vec![]);
        let axis = app.tempo_curve.viewport.axis;
        let scroll = app.tempo_curve.viewport.scroll;
        let dots = super::circles(&before, super::TEMPO);
        let span = dots[1].0.x - dots[0].0.x;
        at(
            &mut app,
            &ctx,
            11.0,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -40.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let output = at(&mut app, &ctx, 11.5, vec![]);
        let dots = super::circles(&output, super::TEMPO);
        let next_span = dots[1].0.x - dots[0].0.x;
        if mode == 0 {
            assert!(next_span < span);
            assert_eq!(app.tempo_curve.viewport.axis, axis);
        } else if mode == 1 {
            assert!(app.tempo_curve.viewport.axis.is_some());
            assert!((next_span - span).abs() < 0.01);
        } else if mode == 2 {
            assert!(app.tempo_curve.viewport.axis.unwrap().low < 120.0);
            assert_eq!(app.tempo_curve.viewport.scroll, scroll);
            assert!((next_span - span).abs() < 0.01);
        } else {
            assert!(app.tempo_curve.viewport.scroll > scroll);
            assert_eq!(app.tempo_curve.viewport.axis, axis);
            assert!((next_span - span).abs() < 0.01);
        }
        let axis = app.tempo_curve.viewport.axis;
        let scroll = app.tempo_curve.viewport.scroll;
        wheel(&mut app, &ctx, 12.0);
        let output = at(&mut app, &ctx, 13.0, vec![]);
        let dots = super::circles(&output, super::TEMPO);
        assert!((dots[1].0.x - dots[0].0.x - next_span).abs() < 0.01);
        assert_eq!(app.tempo_curve.viewport.axis, axis);
        assert!(app.tempo_curve.viewport.scroll > scroll);
    }
}

#[test]
fn wheel_on_the_time_zoom_bar_zooms_only_while_no_repeat_drag_is_held() {
    for dragging in [false, true] {
        // Long enough that a wheel scroll is not clamped at the start.
        let mut song = music();
        for track in &mut song.tracks {
            track.events.last_mut().unwrap().tick = 192_000;
            cat_smf_editor::edit::refresh_deltas(track).unwrap();
        }
        let mut app = app(song);
        let (ctx, output) = opened_in(&mut app, 0);
        let bar = squares(&output, super::super::viewport::BAR_COLOR)[0].0;
        let dot = super::circles(&output, super::TEMPO)[0].0;
        let row = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r)
                    if r.fill == egui::Color32::from_gray(40)
                        && (r.rect.height() - 24.0).abs() < 0.01 =>
                {
                    Some(r.rect.center().y)
                }
                _ => None,
            })
            .unwrap();
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let from = egui::pos2(dot.x + 96.0, row);
        let mut time = 11.0;
        let mut step = |app: &mut crate::app::App, events| {
            let output = at(app, &ctx, time, events);
            time += 0.05;
            let dots = super::circles(&output, super::TEMPO);
            dots[1].0.x - dots[0].0.x
        };
        if dragging {
            step(&mut app, vec![egui::Event::PointerMoved(from)]);
            step(&mut app, vec![button(from, true)]);
            step(
                &mut app,
                vec![egui::Event::PointerMoved(from + egui::vec2(96.0, 0.0))],
            );
            assert!(!app.tempo_curve.repeat_drag.is_none());
        }
        let before = step(&mut app, vec![]);
        let scroll = app.tempo_curve.viewport.scroll;
        step(
            &mut app,
            vec![
                egui::Event::PointerMoved(bar.center()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -40.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        for _ in 0..10 {
            step(&mut app, vec![]);
        }
        let after = step(&mut app, vec![]);
        if dragging {
            assert!((after - before).abs() < 0.01, "span {before} -> {after}");
            assert_eq!(app.tempo_curve.viewport.scroll, scroll);
            step(&mut app, vec![button(bar.center(), false)]);
            step(&mut app, vec![]);
        } else {
            assert!((after - before).abs() > 1.0, "span {before} -> {after}");
        }
    }
}
