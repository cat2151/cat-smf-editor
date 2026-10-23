use super::*;
use cat_smf_editor::{
    edit::tracks,
    score::{
        macro_curve,
        macro_values::{self, Values, Variations},
    },
};

fn fixture() -> App {
    let mut music = tracks::new_song();
    let lane = |value| Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-fixture".into(),
        },
        minimum: 0.0,
        maximum: 1.0,
        value_space: ValueSpace::Plain,
        name: "Macro 1".into(),
        path: "Macros".into(),
        points: vec![Point { tick: 0, value }],
        controls: vec![],
    };
    macro_values::write(
        &mut music.tracks[0],
        &Values {
            version: 1,
            parameters: vec![lane(0.2)],
            variations: vec![Variations {
                layer: 1,
                selected: 0,
                snapshots: vec![vec![lane(0.2)], vec![lane(0.8)], vec![lane(0.4)]],
            }],
        },
    )
    .unwrap();
    let mut app = crate::view::tests::app(music);
    // Use the saved instrument identity without opening native plugins.
    let mut c = lane(0.0);
    c.target.parameter_id = "macro-variation".into();
    c.target.plugin_id = "CLAP:org.surge-synth-team.surge-xt".into();
    c.maximum = 2.0;
    c.points.clear();
    macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &c).unwrap();
    app.parameter_editor = State {
        open: true,
        variation_layer: Some(1),
        owner: Some(0),
        ..Default::default()
    };
    app
}

fn index_curve() -> Curve {
    Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "CLAP:org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-variation".into(),
        },
        minimum: 0.0,
        maximum: 2.0,
        value_space: ValueSpace::Plain,
        name: "Macro variation curve".into(),
        path: "Macros".into(),
        points: vec![
            Point {
                tick: 0,
                value: 0.0,
            },
            Point {
                tick: 480,
                value: 1.0,
            },
            Point {
                tick: 960,
                value: 2.0,
            },
        ],
        controls: vec![
            Point {
                tick: 240,
                value: 0.0,
            },
            Point {
                tick: 720,
                value: 1.5,
            },
        ],
    }
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}
fn key(app: &mut App, ctx: &egui::Context, key: egui::Key) {
    frame(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect(),
    );
}

#[test]
fn keyboard_selection_cancel_confirm_control_undo_and_close() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    key(&mut app, &ctx, egui::Key::A);
    assert!(
        app.parameter_editor.input_open(),
        "{:?}",
        app.parameter_editor.error
    );
    let rendered = frame(&mut app, &ctx, vec![]);
    assert!(!rendered.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::Text(t) if t.galley.text() == "Tick")));
    key(&mut app, &ctx, egui::Key::ArrowUp);
    assert_eq!(app.parameter_editor.input.as_ref().unwrap().value, 1.0);
    key(&mut app, &ctx, egui::Key::Escape);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
    key(&mut app, &ctx, egui::Key::A);
    key(&mut app, &ctx, egui::Key::ArrowUp);
    key(&mut app, &ctx, egui::Key::Enter);
    assert_eq!(current(&app).unwrap().points[0].value, 1.0);
    assert!(!app.parameter_editor.input_open());
    crate::cursor::moves::at_tick(&mut app, 480);
    key(&mut app, &ctx, egui::Key::A);
    key(&mut app, &ctx, egui::Key::ArrowDown);
    key(&mut app, &ctx, egui::Key::Enter);
    assert_eq!(current(&app).unwrap().points[1].value, 0.0);
    key(&mut app, &ctx, egui::Key::J);
    key(&mut app, &ctx, egui::Key::Enter);
    let rendered = frame(&mut app, &ctx, vec![]);
    assert!(!rendered.shapes.iter().any(|s| matches!(&s.shape,
        egui::epaint::Shape::Text(t) if t.galley.text() == "Tick")));
    key(&mut app, &ctx, egui::Key::ArrowUp);
    key(&mut app, &ctx, egui::Key::Enter);
    assert!(
        (current(&app).unwrap().controls[0].value - 0.52).abs() < 1e-12,
        "{:?} / {:?}",
        current(&app).unwrap(),
        app.parameter_editor.error
    );
    app.undo_music().unwrap();
    assert_eq!(current(&app).unwrap().controls[0].value, 0.5);
    key(&mut app, &ctx, egui::Key::H);
    key(&mut app, &ctx, egui::Key::Delete);
    assert_eq!(current(&app).unwrap().points.len(), 1);
    key(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.parameter_editor.open);
}

#[test]
fn canvas_empty_click_opens_choice_without_committing() {
    let mut app = fixture();
    let c = index_curve();
    macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &c).unwrap();
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let segment = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::QuadraticBezier(b) => Some(b.points),
            _ => None,
        })
        .unwrap();
    let pos = egui::pos2((segment[0].x + segment[2].x) / 2.0, segment[2].y);
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    assert!(app.parameter_editor.input_open());
    assert_eq!(current(&app).unwrap(), c);
    assert!(!app.undo.available());
    key(&mut app, &ctx, egui::Key::Escape);
    assert_eq!(current(&app).unwrap(), c);
}

#[test]
fn maximization_setting_is_independent_and_restores() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("status.json");
    std::fs::write(&path, b"{}").unwrap();
    // Real status has required fields; older data lacks only the new preference.
    let mut original = serde_json::to_value(crate::store::config::Status::default()).unwrap();
    original
        .as_object_mut()
        .unwrap()
        .remove("macro_variation_curve_maximized");
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let mut store = crate::store::config::Store::open(path.clone());
    assert!(store.macro_variation_curve_maximized);
    store.select_macro_variation_curve_maximized(false);
    let restored = crate::store::config::Store::open(path);
    assert!(!restored.macro_variation_curve_maximized);
    assert!(restored.parameter_curve_maximized && restored.tempo_curve_maximized);
}

#[test]
fn playback_plan_uses_curve_and_explicit_parameter_automation_takes_priority() {
    let mut app = fixture();
    let loaded = app.loaded.as_mut().unwrap();
    macro_curve::write(&mut loaded.music.tracks[0], &index_curve()).unwrap();
    let template = macro_values::read(&loaded.music.tracks[0])
        .unwrap()
        .parameters[0]
        .clone();
    loaded.patches[0].layers[0].instrument = Some(cat_smf_editor::chain_settings::PluginState {
        reference: None,
        plugin: PluginKey {
            format: template.target.plugin_format.clone(),
            id: template.target.plugin_id.clone(),
            name: "Surge XT".into(),
            vendor: String::new(),
            bundle_path: String::new(),
        },
        state: vec![],
        preset_name: None,
        bypass: false,
    });
    let parts = vec![(
        0,
        loaded.patches[0].clone(),
        crate::play::audio_sequence::Sequence::new(vec![], 2.0),
    )];
    let plan = crate::play::automation::Automation::prepare(&app, &parts, true);
    assert!(plan.error.is_none(), "{:?}", plan.error);
    let owner = plan.owner(0).unwrap();
    assert_eq!(owner.curves.len(), 1);
    assert!((owner.curves[0].value_at(240.0).unwrap() - 0.35).abs() < 1e-10);
    assert!(owner.boundaries.iter().any(|&(tick, _)| tick == 480));
    let mut explicit = template;
    explicit.points[0].value = 0.9;
    explicit.target.group = 0;
    parameter_curve::create(&mut app.loaded.as_mut().unwrap().music, 0, explicit).unwrap();
    let plan = crate::play::automation::Automation::prepare(&app, &parts, true);
    assert!(plan.error.is_none(), "{:?}", plan.error);
    let owner = plan.owner(0).unwrap();
    assert_eq!(owner.curves.len(), 1);
    assert_eq!(owner.curves[0].value_at(240.0), Some(0.9));
}

#[test]
fn bezier_snapshots_roundtrip_reverse_skip_and_missing_reference() {
    let mut app = fixture();
    let track = &mut app.loaded.as_mut().unwrap().music.tracks[0];
    let original = macro_values::read(track).unwrap();
    let c = index_curve();
    macro_curve::write(track, &c).unwrap();
    let lane = &macro_curve::expand(track).unwrap()[0];
    assert_eq!(lane.value_at(0.0), Some(0.2));
    assert!((lane.value_at(240.0).unwrap() - 0.35).abs() < 1e-10);
    assert!((lane.value_at(720.0).unwrap() - 0.6).abs() < 1e-10);
    assert_eq!(lane.value_at(10000.0), Some(0.4));
    assert_eq!(original, macro_values::read(track).unwrap());
    let bytes = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let restored = cat_smf_editor::ffi::parse(&bytes).unwrap();
    assert_eq!(
        macro_curve::read(&restored.tracks[0]).unwrap(),
        vec![c.clone()]
    );
    let track = &mut app.loaded.as_mut().unwrap().music.tracks[0];
    let mut skip = c;
    skip.points = vec![
        Point {
            tick: 0,
            value: 2.0,
        },
        Point {
            tick: 480,
            value: 0.0,
        },
    ];
    skip.controls = vec![Point {
        tick: 240,
        value: 1.0,
    }];
    macro_curve::write(track, &skip).unwrap();
    assert!(
        (macro_curve::expand(track).unwrap()[0]
            .value_at(240.0)
            .unwrap()
            - 0.3)
            .abs()
            < 1e-10
    );
    skip.maximum = 3.0;
    skip.points[0].value = 3.0;
    let before = track.clone();
    assert!(macro_curve::write(track, &skip).is_err());
    assert_eq!(format!("{before:?}"), format!("{track:?}"));
}

#[test]
#[ignore = "Read-only check of the current local SMF"]
fn current_user_smf_preserves_variations_through_curve_save_and_reload() {
    let path = std::env::var_os("MACRO_CURVE_TEST_SMF").expect("Set MACRO_CURVE_TEST_SMF");
    let bytes = std::fs::read(path).unwrap();
    let mut music = cat_smf_editor::ffi::parse(&bytes).unwrap();
    let (owner, values, set) = music
        .tracks
        .iter()
        .enumerate()
        .find_map(|(owner, t)| {
            let values = macro_values::read(t).ok()?;
            let set = values
                .variations
                .iter()
                .find(|s| s.snapshots.len() >= 2)?
                .clone();
            Some((owner, values, set))
        })
        .expect("Actual SMF must contain the user's two variations");
    let template = &set.snapshots[0][0];
    let mut c = index_curve();
    c.target.layer = set.layer;
    c.target.plugin_id = format!(
        "{}:{}",
        template.target.plugin_format, template.target.plugin_id
    );
    c.maximum = (set.snapshots.len() - 1) as f64;
    c.points.truncate(2);
    c.controls.truncate(1);
    c.controls[0].value = -c.maximum;
    macro_curve::write(&mut music.tracks[owner], &c).unwrap();
    let restored =
        cat_smf_editor::ffi::parse(&cat_smf_editor::smf_io::encode(&music).unwrap()).unwrap();
    assert_eq!(macro_values::read(&restored.tracks[owner]).unwrap(), values);
    assert_eq!(macro_curve::read(&restored.tracks[owner]).unwrap(), vec![c]);
    let lanes = macro_curve::expand(&restored.tracks[owner]).unwrap();
    assert_eq!(lanes.len(), set.snapshots[0].len());
    for (i, lane) in lanes.iter().enumerate() {
        assert_eq!(
            lane.value_at(0.0),
            Some(set.snapshots[0][i].points[0].value)
        );
        assert_eq!(
            lane.value_at(480.0),
            Some(set.snapshots[1][i].points[0].value)
        );
    }
    for (i, (before, after)) in music.tracks.iter().zip(&restored.tracks).enumerate() {
        if i != owner {
            assert_eq!(format!("{before:?}"), format!("{after:?}"));
        }
    }
}

#[test]
fn opening_repeats_the_entire_owner_and_keeps_repeat_after_point_edit() {
    use cat_smf_editor::model::Event;
    let mut app = fixture();
    let track = &mut app.loaded.as_mut().unwrap().music.tracks[0];
    for (tick, status) in [(0, 0x90), (960, 0x80)] {
        cat_smf_editor::edit::insert(
            track,
            Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![60, 80],
            },
        );
    }
    cat_smf_editor::edit::refresh_deltas(track).unwrap();
    app.loaded.as_mut().unwrap().refresh();
    variation::open(&mut app, 1);
    assert_eq!(app.playback.repeat, Some((0, 960)));
    assert!(app.playback.pending.is_some());
    let ctx = egui::Context::default();
    key(&mut app, &ctx, egui::Key::A);
    assert_eq!(app.playback.repeat, Some((0, 960)));
    key(&mut app, &ctx, egui::Key::ArrowUp);
    key(&mut app, &ctx, egui::Key::Enter);
    assert_eq!(app.playback.repeat, Some((0, 960)));
    assert!(app.playback.pending.is_some() || app.playback.active.is_some());
    assert_eq!(current(&app).unwrap().points[0].value, 1.0);
}

mod occlusion;
mod unbounded_controls;
mod zoom;

#[test]
fn view_entry_keeps_playback_and_saved_curve_unchanged() {
    let mut app = fixture();
    let curve = index_curve();
    macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &curve).unwrap();
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    variation::open_view(&mut app, 1);
    assert!(app.parameter_editor.open);
    assert!(app.playback.repeat.is_none());
    assert!(app.playback.pending.is_none() && app.playback.active.is_none());
    assert_eq!(current(&app).unwrap(), curve);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
