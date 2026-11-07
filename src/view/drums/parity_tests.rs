//! Compare the real command registrations, so a new operation or changed binding
//! on either sequencer requires matching behavior or an explicit layout exception.
use super::*;
use crate::view::commands::{Command, Screen};
use std::collections::BTreeMap;

fn common_commands(app: &App) -> BTreeMap<String, (Vec<(Key, egui::Modifiers)>, bool)> {
    let mut commands: Vec<Command> = Vec::new();
    super::commands::collect(app, Screen::Drums, &mut commands);
    commands
        .into_iter()
        .filter_map(|command| {
            let id = match command.id.as_str() {
                // The kit has a single instrument and a variable note list.
                "effect.open" | "kit.note-picker" | "kit.add-rows" | "kit.query"
                | "kit.context" => return None,
                "kit.field-next" => "drum.step-next".into(),
                "kit.field-prev" => "drum.step-prev".into(),
                "kit.activate" => "drum.activate".into(),
                "kit.mute" => "drum.mute".into(),
                "kit.row-next" => "drum.row-next".into(),
                "kit.row-prev" => "drum.row-prev".into(),
                "kit.toggle" => "drum.toggle".into(),
                id => id.to_owned(),
            };
            Some((id, (command.keys, command.repeat)))
        })
        .collect()
}

#[test]
fn both_sequencers_offer_the_same_common_operations_and_bindings() {
    let mut drums = super::tests::fixture();
    let mut kit = super::kit_edit_tests::fixture(4);
    super::open(&mut kit);
    assert!(drums.undo.available() && kit.undo.available());
    assert_eq!(common_commands(&drums), common_commands(&kit));
    drums.undo = Default::default();
    kit.undo = Default::default();
    assert_eq!(common_commands(&drums), common_commands(&kit));
    assert!(!common_commands(&drums).contains_key("app.undo"));
}

#[test]
fn both_swing_controls_hide_disabled_amount_without_moving_the_steps() {
    let drums = super::tests::fixture();
    let mut kit = super::kit_edit_tests::fixture(4);
    super::open(&mut kit);
    for mut app in [drums, kit] {
        let is_kit = super::kit::active(&app);
        let c = egui::Context::default();
        c.style_mut(|s| s.animation_time = 0.0);
        let mut baseline = None;
        for enabled in [false, true, false] {
            if baseline.is_some() {
                let steps = playback::measure(&app).unwrap().steps.len();
                app.drum_editor.field = steps + if is_kit { 3 } else { 4 };
                super::tests::press(&mut app, &c, Key::Enter);
            }
            let mut output = Default::default();
            for _ in 0..3 {
                output = c.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1600.0, 1000.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| super::show(&mut app, ctx),
                );
            }
            assert_eq!(
                output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::epaint::Shape::Text(t) if t.galley.text() == "50%"
                )),
                enabled
            );
            let rect = if is_kit {
                c.data(|d| {
                    d.get_temp::<egui::Rect>(egui::Id::new(("kit-cell", 0u8, 0usize)))
                        .unwrap()
                })
            } else {
                c.data(|d| {
                    d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                        "drum-grid-bounds",
                    ))
                })
                .unwrap()
                .2[3 * 18 + 2]
            };
            assert_eq!(*baseline.get_or_insert(rect), rect);
        }
    }
}

#[test]
fn both_grids_draw_gray_boundaries_every_four_steps() {
    let drums = super::tests::fixture();
    let mut kit = super::kit_edit_tests::fixture(4);
    super::open(&mut kit);
    for mut app in [drums, kit] {
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut output = Default::default();
        for _ in 0..3 {
            output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |ctx| super::show(&mut app, ctx),
            );
        }
        let lines: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::LineSegment { points, stroke }
                    if *stroke == egui::Stroke::new(2.0_f32, egui::Color32::from_gray(150)) =>
                {
                    Some(*points)
                }
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 4, "kit: {}", super::kit::active(&app));
        assert!(lines.iter().all(|p| p[0].x == p[1].x && p[0].y < p[1].y));
        assert!(lines.windows(2).all(|p| p[0][0].x < p[1][0].x));
    }
}

#[test]
fn muted_rows_have_identical_hit_empty_step_and_name_colors() {
    let mut drums = super::tests::fixture();
    super::commands::apply(&mut drums, super::commands::Op::Toggle);
    let mut kit = super::kit_edit_tests::fixture(4);
    super::open(&mut kit);
    let initial = egui::Context::default();
    let _ = initial.run(Default::default(), |ctx| super::show(&mut kit, ctx));
    super::tests::press(&mut kit, &initial, Key::F);
    let mut colors = Vec::new();
    for mut app in [drums, kit] {
        let is_kit = super::kit::active(&app);
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut cases = Vec::new();
        for muted in [false, true] {
            if muted {
                super::tests::press(&mut app, &ctx, Key::M);
            }
            let mut output = Default::default();
            for _ in 0..3 {
                output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1600.0, 1000.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| super::show(&mut app, ctx),
                );
            }
            let fills: Vec<_> = (0..2)
                .map(|step| {
                    let rect = if is_kit {
                        ctx.data(|d| {
                            d.get_temp::<egui::Rect>(egui::Id::new(("kit-cell", 0u8, step)))
                                .unwrap()
                        })
                    } else {
                        ctx.data(|d| {
                            d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                                "drum-grid-bounds",
                            ))
                            .unwrap()
                            .2[3 * 18 + 2 + step]
                        })
                    };
                    output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::epaint::Shape::Rect(r) if r.rect == rect => Some(r.fill),
                            _ => None,
                        })
                        .unwrap()
                })
                .collect();
            if muted {
                let label = if is_kit { "Source 0" } else { drums::NAMES[0] };
                assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::epaint::Shape::Text(t) if t.galley.text() == label
                        && t.galley.job.sections.iter().any(|s| s.format.color == egui::Color32::from_gray(100))
                )));
            }
            cases.push(fills);
        }
        assert_ne!(cases[0][0], cases[1][0]);
        assert_ne!(cases[0][1], cases[1][1]);
        colors.push(cases);
    }
    assert_eq!(colors[0], colors[1]);
}
