use super::*;
use crate::view::commands::{self, Screen};
use cat_smf_editor::edit::tracks;
use egui::{Key, Modifiers};

#[test]
fn owner_view_edits_the_parameter_lane_without_replacing_the_score_track() {
    let mut a = app();
    let lane = a.track;
    let owner_bytes = |a: &App| {
        let mut music = a.loaded.as_ref().unwrap().music.clone();
        music.tracks.truncate(1);
        cat_smf_editor::smf_io::encode(&music).unwrap()
    };
    let before = owner_bytes(&a);
    a.select_track(0);
    open_existing(&mut a, lane);
    assert_eq!(a.track, 0);
    assert_eq!(a.parameter_editor.owner, Some(0));
    // Native parameter resolution can fail in this headless fixture; the stored curve
    // must still resolve to its dedicated lane for display and every edit.
    let mut curve = current(&a).unwrap();
    curve.points[0].value = 0.4;
    store(&mut a, curve.clone());
    assert_eq!(a.track, 0);
    assert_eq!(
        parameter_curve::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .curves[&lane],
        curve
    );
    assert_eq!(owner_bytes(&a), before);
    a.parameter_editor.close();
    assert_eq!(a.track, 0);
}
pub(super) fn app() -> App {
    let mut a = crate::view::tests::app(tracks::new_song());
    a.create_parameter_track(
        0,
        Curve {
            version: 1,
            target: Target {
                group: 0,
                layer: 1,
                plugin_format: "CLAP".into(),
                plugin_id: "fixture".into(),
                parameter_id: "101".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.0,
            maximum: 1.0,
            name: "Cutoff".into(),
            path: "Filter".into(),
            points: vec![
                Point {
                    tick: 0,
                    value: 0.2,
                },
                Point {
                    tick: 480,
                    value: 0.8,
                },
            ],
            controls: vec![Point {
                tick: 240,
                value: 0.3,
            }],
        },
    )
    .unwrap();
    a.parameter_editor = State {
        open: true,
        selected: Some((crate::view::tempo_curve::Target::Point, 0)),
        ..Default::default()
    };
    a
}
fn press(a: &mut App, c: &egui::Context, key: Key) {
    let _ = c.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events: [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: if key == Key::Comma {
                        Modifiers::CTRL | Modifiers::COMMAND
                    } else {
                        Modifiers::NONE
                    },
                })
                .collect(),
            ..Default::default()
        },
        |c| crate::view::show(a, c),
    );
}
#[test]
fn keyboard_point_control_input_cancel_delete_and_undo_are_isolated() {
    let mut a = app();
    let ctx = egui::Context::default();
    let before = current(&a).unwrap();
    let generation = a.generation;
    press(&mut a, &ctx, Key::J);
    assert_eq!(
        a.parameter_editor.selected,
        Some((crate::view::tempo_curve::Target::Control, 0))
    );
    press(&mut a, &ctx, Key::Enter);
    assert!(a.parameter_editor.input_open());
    assert_eq!(commands::owner(&a, &ctx), Screen::ParameterInput);
    press(&mut a, &ctx, Key::ArrowUp);
    press(&mut a, &ctx, Key::Escape);
    assert!(a.parameter_editor.open);
    assert_eq!(current(&a).unwrap(), before);
    assert_eq!(a.generation, generation);
    press(&mut a, &ctx, Key::Enter);
    press(&mut a, &ctx, Key::ArrowUp);
    press(&mut a, &ctx, Key::Enter);
    assert_eq!(current(&a).unwrap().controls[0].value, 0.31);
    a.undo_music().unwrap();
    assert_eq!(current(&a).unwrap(), before);
    press(&mut a, &ctx, Key::H);
    press(&mut a, &ctx, Key::Delete);
    assert_eq!(current(&a).unwrap().points.len(), 1);
    press(&mut a, &ctx, Key::Delete);
    assert!(current(&a).unwrap().points.is_empty());
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.parameter_editor.open);
}
#[test]
fn addition_preview_cancel_and_navigation_do_not_dirty() {
    let mut a = app();
    let ctx = egui::Context::default();
    let before = current(&a).unwrap();
    let generation = a.generation;
    crate::cursor::moves::at_tick(&mut a, 960);
    press(&mut a, &ctx, Key::PageUp);
    press(&mut a, &ctx, Key::A);
    assert!(a.parameter_editor.input_open());
    assert_eq!(current(&a).unwrap(), before);
    press(&mut a, &ctx, Key::Escape);
    assert!(a.parameter_editor.draft.is_none());
    assert_eq!(a.generation, generation);
    press(&mut a, &ctx, Key::A);
    press(&mut a, &ctx, Key::ArrowDown);
    press(&mut a, &ctx, Key::Enter);
    assert_eq!(
        current(&a).unwrap().points.last(),
        Some(&Point {
            tick: 960,
            value: 0.79
        })
    );
    a.undo_music().unwrap();
    assert_eq!(current(&a).unwrap(), before);
}
#[test]
fn invalid_numeric_input_keeps_curve_and_input() {
    let mut a = app();
    let before = current(&a).unwrap();
    curve::begin_input(&mut a);
    a.parameter_editor.input.as_mut().unwrap().tick = "bad tick".into();
    curve::commit_input(&mut a);
    assert!(a.parameter_editor.input_open());
    assert!(a.parameter_editor.error.is_some());
    assert_eq!(current(&a).unwrap(), before);
}
#[test]
fn layer_palette_contains_targeted_automation_and_picker_owns_text() {
    let a = app();
    let ctx = egui::Context::default();
    let commands = commands::definitions(&a, Screen::Layers);
    assert!(commands.iter().any(|c| c.id == "layer-automation"));
    assert!(!Screen::ParameterPicker.owns_keys());
    assert!(!Screen::ParameterInput.owns_keys());
    assert_eq!(commands::owner(&a, &ctx), Screen::Timeline);
}

pub(super) fn picker() -> Picker {
    Picker {
        history: Default::default(),
        tick: 0,
        previewed: false,
        help: false,
        title: "Fixture".into(),
        song: 99,
        generation: 0,
        owner: 0,
        layer: 1,
        patch: Patch::default(),
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "fixture".into(),
            name: "Fixture".into(),
            vendor: String::new(),
            bundle_path: String::new(),
        },
        parameters: vec![Parameter {
            stable_id: "101".into(),
            name: "音色".into(),
            path: "フィルター".into(),
            minimum: 0.0,
            maximum: 1.0,
            current: 0.4,
            normalized: false,
            eligible: true,
            binding: Default::default(),
        }],
        query: String::new(),
        search: Default::default(),
        selected: 0,
        focus: true,
        composing: false,
        reveal: true,
    }
}
#[test]
fn migemo_multiple_terms_and_stale_song_rejection_preserve_document() {
    let mut p = picker();
    p.query = "onshoku firuta".into();
    assert_eq!(filtered(&mut p), vec![0]);
    p.query = "音色 フィルター".into();
    assert_eq!(filtered(&mut p), vec![0]);
    let mut a = app();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    a.parameter_editor.picker = Some(p);
    assert!(create(&mut a).unwrap_err().contains("target changed"));
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
}

#[test]
fn keyboard_value_preview_clamps_and_commits_as_one_undo() {
    let mut a = app();
    let ctx = egui::Context::default();
    let before = current(&a).unwrap();
    press(&mut a, &ctx, Key::Enter);
    for _ in 0..110 {
        press(&mut a, &ctx, Key::ArrowUp);
    }
    assert_eq!(current(&a).unwrap(), before);
    assert_eq!(a.parameter_editor.input.as_ref().unwrap().value, 1.0);
    press(&mut a, &ctx, Key::Enter);
    assert_eq!(current(&a).unwrap().points[0].value, 1.0);
    a.undo_music().unwrap();
    assert_eq!(current(&a).unwrap(), before);
    press(&mut a, &ctx, Key::Enter);
    for _ in 0..110 {
        press(&mut a, &ctx, Key::ArrowDown);
    }
    assert_eq!(a.parameter_editor.input.as_ref().unwrap().value, 0.0);
    press(&mut a, &ctx, Key::Escape);
    assert_eq!(current(&a).unwrap(), before);
}
#[test]
fn minimap_navigation_cancels_follow_without_audition_or_edit() {
    let mut a = app();
    let ctx = egui::Context::default();
    a.playback.follow = true;
    let frame = |a: &mut App, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |c| crate::view::show(a, c),
        );
    };
    frame(&mut a, vec![]);
    frame(&mut a, vec![]);
    let rect = a.parameter_editor.viewport.minimap_rect.unwrap();
    let pos = rect.center();
    let generation = a.generation;
    frame(
        &mut a,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        &mut a,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!a.playback.follow);
    assert!(a.playback.pending.is_none());
    assert!(a.playback.active.is_none());
    assert_eq!(a.generation, generation);
}

#[test]
fn canvas_control_drag_previews_and_commits_on_release() {
    control_drag(false);
    control_drag(true);
}
fn control_drag(coalesced: bool) {
    let mut a = app();
    let ctx = egui::Context::default();
    let before = current(&a).unwrap();
    let frame = |a: &mut App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |c| crate::view::show(a, c),
        )
    };
    frame(&mut a, vec![]);
    let output = frame(&mut a, vec![]);
    let start = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::QuadraticBezier(b) => Some(b.points[1]),
            _ => None,
        })
        .expect("drawn control point");
    let end = start + egui::vec2(8.0, 30.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(&mut a, vec![egui::Event::PointerMoved(start)]);
    if coalesced {
        frame(
            &mut a,
            vec![
                button(start, true),
                egui::Event::PointerMoved(end),
                button(end, false),
            ],
        );
    } else {
        frame(&mut a, vec![button(start, true)]);
        frame(&mut a, vec![egui::Event::PointerMoved(end)]);
        assert!(a.parameter_editor.drag);
        assert_ne!(
            a.parameter_editor.draft.as_ref().unwrap().controls,
            before.controls
        );
        assert_eq!(current(&a).unwrap(), before);
        frame(&mut a, vec![button(end, false)]);
    }
    assert_ne!(current(&a).unwrap().controls, before.controls);
    a.undo_music().unwrap();
    assert_eq!(current(&a).unwrap(), before);
}

#[test]
fn parameter_curve_maximization_is_independent_persistent_and_defaults_on() {
    let root = std::env::temp_dir().join(format!(
        "cat-smf-parameter-maximized-{}",
        std::process::id()
    ));
    let status = root.join("status.json");
    let mut app = app();
    app.config = crate::store::config::Store::open(status.clone());
    assert!(app.config.parameter_curve_maximized);
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::Comma);
    assert!(app.comma_menu.open);
    press(&mut app, &ctx, egui::Key::C);
    let saved = crate::store::config::Store::open(status.clone());
    assert!(!saved.parameter_curve_maximized);
    assert!(saved.tempo_curve_maximized && saved.piano_roll_maximized);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.parameter_editor.open);
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::C);
    assert!(crate::store::config::Store::open(status.clone()).parameter_curve_maximized);
    std::fs::write(&status, r#"{"session":null,"tempo_curve_maximized":false}"#).unwrap();
    let saved = crate::store::config::Store::open(status);
    assert!(saved.parameter_curve_maximized && !saved.tempo_curve_maximized);
    std::fs::remove_dir_all(root).unwrap();
}
