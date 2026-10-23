use super::*;
use cat_smf_editor::edit::tracks;
use cat_smf_editor::score::macro_values::Values;
fn session(app: &App) -> Session {
    let curve = Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.surge-synth-team.surge-xt".into(),
            parameter_id: "macro-fixture".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Macro 1".into(),
        path: "Macros".into(),
        points: vec![Point {
            tick: 0,
            value: 0.25,
        }],
        controls: vec![],
    };
    Session {
        owner: 0,
        layer: 1,
        song: app.song_id,
        generation: app.generation,
        patch: app.loaded.as_ref().unwrap().patches[0].clone(),
        title: "Macros".into(),
        rows: vec![Row {
            curve,
            value: Arc::new(AtomicU64::new(0.25f64.to_bits())),
            bipolar: false,
            readback: None,
        }],
        selected: 0,
        keyboard: Default::default(),
        variations: vec![],
        variation: 0,
        readback_at: None,
    }
}
#[test]
fn draft_cancel_preserves_document_and_confirm_roundtrips_in_one_undo() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    app.macro_editor.session = Some(session(&app));
    app.macro_editor.session.as_ref().unwrap().rows[0].set(0.8);
    finish(&mut app, false);
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!app.undo.available());
    app.macro_editor.session = Some(session(&app));
    app.macro_editor.session.as_ref().unwrap().rows[0].set(0.6);
    finish(&mut app, true);
    assert!(!app.macro_editor.open());
    let bytes = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let music = cat_smf_editor::ffi::parse(&bytes).unwrap();
    assert_eq!(
        macro_values::read(&music.tracks[0]).unwrap().parameters[0].points[0].value,
        0.6
    );
    app.undo_music().unwrap();
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!app.undo.available());
}
#[test]
fn live_binding_updates_without_restarting_transport_and_commands_own_input() {
    let mut app = crate::view::tests::app(tracks::new_song());
    app.macro_editor.session = Some(session(&app));
    let row = &app.macro_editor.session.as_ref().unwrap().rows[0];
    let binding = crate::play::automation::Binding {
        curve: Arc::new(row.curve.clone()),
        live: Some(row.value.clone()),
        value: Default::default(),
    };
    row.set(0.7);
    assert_eq!(binding.at(500.0), Some(0.7));
    let ctx = egui::Context::default();
    assert_eq!(
        crate::view::commands::owner(&app, &ctx),
        crate::view::commands::Screen::MacroEditor
    );
    let commands =
        crate::view::commands::definitions(&app, crate::view::commands::Screen::MacroEditor);
    let command = commands.iter().find(|c| c.id == "macro-more").unwrap();
    crate::view::commands::run_direct(&mut app, &ctx, command);
    assert!((binding.at(0.0).unwrap() - 0.825).abs() < 1e-12);
    assert!(!commands.iter().any(|c| c.id == "next-track"));
}
#[test]
fn stale_target_cancels_draft_and_invalid_metadata_is_rejected() {
    let mut app = crate::view::tests::app(tracks::new_song());
    app.macro_editor.session = Some(session(&app));
    app.generation += 1;
    validate(&mut app);
    assert!(!app.macro_editor.open());
    let mut values = Values {
        version: 1,
        variations: vec![],
        parameters: vec![session(&app).rows[0].saved()],
    };
    values.parameters.push(values.parameters[0].clone());
    assert!(values.validate().is_err());
    values.parameters.pop();
    values.parameters[0].points[0].value = f64::NAN;
    assert!(values.validate().is_err());
}

#[test]
fn playback_fixed_values_automation_precedence_and_repeat_draft() {
    use cat_smf_editor::chain_settings::PluginState;
    let mut app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    let key = cat_smf_editor::host::PluginKey {
        format: "CLAP".into(),
        id: "org.surge-synth-team.surge-xt".into(),
        name: "Surge XT".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/surge.clap".into(),
    };
    let slot = crate::test_support::referenced(PluginState {
        reference: None,
        plugin: key,
        state: vec![1],
        preset_name: None,
        bypass: false,
    });
    app.loaded.as_mut().unwrap().patches[0].layers[0].instrument = Some(slot);
    s.patch = app.loaded.as_ref().unwrap().patches[0].clone();
    assert_eq!(
        crate::view::commands::candidates(&app, &egui::Context::default())
            .iter()
            .filter(|c| c.id.starts_with("macro-edit-"))
            .count(),
        1
    );

    let fixed = s.rows[0].saved();
    macro_values::write(
        &mut app.loaded.as_mut().unwrap().music.tracks[0],
        &Values {
            version: 1,
            variations: vec![],
            parameters: vec![fixed.clone()],
        },
    )
    .unwrap();
    let parts = vec![(0, s.patch.clone(), Sequence::new(vec![], 1.0))];
    let mut owners = BTreeMap::new();
    plans(&app, &parts, &mut owners).unwrap();
    assert_eq!(owners[&0][0].points[0].value, 0.25);
    let mut automation = fixed.clone();
    automation.points[0].value = 0.9;
    owners.insert(0, vec![Arc::new(automation)]);
    plans(&app, &parts, &mut owners).unwrap();
    assert_eq!(owners[&0].len(), 1);
    assert_eq!(owners[&0][0].points[0].value, 0.9);
    s.rows[0].set(0.7);
    app.macro_editor.session = Some(s);
    plans(&app, &parts, &mut owners).unwrap();
    assert_eq!(owners[&0].len(), 1);
    assert_eq!(owners[&0][0].points[0].value, 0.7);
    let prepared = transport::repeat::prepare(&app, (0, 480)).unwrap();
    assert!(prepared.active.automation.error.is_none());
    assert_eq!(prepared.active.automation.owners[0].curves.len(), 1);
    let values = live(&app, 0);
    app.macro_editor.session.as_ref().unwrap().rows[0].set(0.8);
    assert_eq!(
        f64::from_bits(values[&fixed.target].load(Ordering::Relaxed)),
        0.8
    );
    assert!(
        cat_smf_editor::score::macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[0])
            .unwrap()
            .parameters[0]
            .points[0]
            .value
            == 0.25
    );
    finish(&mut app, false);
    let prepared = transport::prepare(&app, crate::play::playback::Mode::AllTracks).unwrap();
    assert_eq!(prepared.active.automation.owners.len(), 1);
    macro_values::write(
        &mut app.loaded.as_mut().unwrap().music.tracks[0],
        &Values {
            version: 1,
            variations: vec![],
            parameters: vec![],
        },
    )
    .unwrap();
    crate::play::selection_playback::restore(&mut app, prepared);
    assert!(app
        .playback
        .pending
        .as_ref()
        .unwrap()
        .active
        .automation
        .owners
        .is_empty());
}

#[test]
fn keyboard_edit_confirm_cancel_do_not_reach_score() {
    fn key(app: &mut App, ctx: &egui::Context, key: egui::Key) {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| {
                crate::view::commands::route(app, ctx);
                show(app, ctx);
            },
        );
    }
    let mut app = crate::view::tests::app(tracks::new_song());
    let ctx = egui::Context::default();
    let before_cursor = app.cursor_row;
    app.macro_editor.session = Some(session(&app));
    key(&mut app, &ctx, egui::Key::ArrowDown);
    assert_eq!(app.cursor_row, before_cursor);
    key(&mut app, &ctx, egui::Key::End);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[0].get(),
        1.0
    );
    key(&mut app, &ctx, egui::Key::ArrowLeft);
    assert_eq!(
        app.macro_editor.session.as_ref().unwrap().rows[0].get(),
        0.875
    );
    key(&mut app, &ctx, egui::Key::Enter);
    assert!(!app.macro_editor.open());
    assert_eq!(
        macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[0])
            .unwrap()
            .parameters[0]
            .points[0]
            .value,
        0.875
    );
    app.macro_editor.session = Some(session(&app));
    key(&mut app, &ctx, egui::Key::Home);
    key(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.macro_editor.open());
    assert_eq!(
        macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[0])
            .unwrap()
            .parameters[0]
            .points[0]
            .value,
        0.875
    );
}

#[test]
fn bipolar_display_and_api_read_diagnostic_do_not_change_the_draft() {
    let app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    let row = &mut s.rows[0];
    row.bipolar = true;
    row.set(0.5);
    assert_eq!(row.signal(), 0.0);
    row.set(0.0);
    assert_eq!(row.signal(), -1.0);
    row.readback = Some((-1.0, Ok("0".into())));
    assert_eq!(row.saved().points[0].value, 0.0);
    assert!(row.curve.domain().unwrap().value(-1.0).is_err());
    assert_eq!(row.get(), 0.0);
}

#[test]
fn rendered_dual_values_and_bar_drag_preserve_the_bipolar_center() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let mut s = session(&app);
    s.rows[0].bipolar = true;
    s.rows[0].readback = Some((-1.0, Ok("0.000".into())));
    let mut second = s.rows[0].curve.clone();
    second.name = "Macro 2 with a longer name".into();
    second.target.parameter_id = "second".into();
    s.rows.push(Row {
        curve: second,
        value: Arc::new(AtomicU64::new(0.7_f64.to_bits())),
        bipolar: false,
        readback: None,
    });
    app.macro_editor.session = Some(s);
    let ctx = egui::Context::default();
    let frame = |app: &mut App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| show(app, ctx),
        )
    };
    frame(&mut app, vec![]);
    frame(&mut app, vec![]);
    let output = frame(&mut app, vec![]);
    let text: String = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("Macro -0.500") && text.contains("pos 0.250"),
        "{text}"
    );
    assert!(
        text.contains("bipolar [-1, 1]") && text.contains("read -1 outside [0, 1]"),
        "{text}"
    );
    let backgrounds: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if (r.rect.width() - 280.0).abs() < 0.1 && (r.rect.height() - 26.0).abs() < 0.1 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(backgrounds.len(), 2);
    assert_eq!(backgrounds[0].left(), backgrounds[1].left());
    let bar = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r)
                if (r.rect.width() - 280.0).abs() < 0.1 && (r.rect.height() - 26.0).abs() < 0.1 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .expect("macro bar");
    let start = egui::pos2(bar.left() + bar.width() * 0.25, bar.center().y);
    frame(
        &mut app,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let end = bar.center();
    frame(&mut app, vec![egui::Event::PointerMoved(end)]);
    frame(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let row = &app.macro_editor.session.as_ref().unwrap().rows[0];
    assert!((row.get() - 0.5).abs() < 0.001);
    assert!(row.signal().abs() < 0.002);
    assert!(!app.undo.available());
}

#[path = "tests/variations.rs"]
mod variation_tests;

#[path = "tests/randomize.rs"]
mod randomize_tests;

#[path = "tests/levels.rs"]
mod level_tests;

mod keyboard;
