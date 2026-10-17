use super::*;
use cat_smf_editor::model::{Music, Track};

fn fixture() -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }; 2],
    })
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn frame(app: &mut App, ctx: &egui::Context, mut events: Vec<egui::Event>) -> egui::FullOutput {
    let releases: Vec<_> = events
        .iter()
        .filter_map(|event| {
            if let egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                Some(egui::Event::Key {
                    key: *key,
                    physical_key: *physical_key,
                    pressed: false,
                    repeat: false,
                    modifiers: *modifiers,
                })
            } else {
                None
            }
        })
        .collect();
    events.extend(releases);
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn press(app: &mut App, ctx: &egui::Context, k: egui::Key) {
    frame(app, ctx, vec![key(k, egui::Modifiers::NONE)]);
}

fn replace(app: &mut App, ctx: &egui::Context, text: &str) {
    frame(
        app,
        ctx,
        vec![
            key(
                egui::Key::A,
                egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
            egui::Event::Text(text.into()),
        ],
    );
}

#[test]
fn keyboard_edit_confirm_cancel_duplicate_delete_and_recreate() {
    let mut app = fixture();
    app.select_track(1);
    let ctx = egui::Context::default();
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::T, egui::Modifiers::SHIFT)],
    );
    assert!(app.layer_editor.open);
    assert!(app.player_selection.resident.is_none());
    let original = app.layer_editor.selected;
    press(&mut app, &ctx, egui::Key::L);
    press(&mut app, &ctx, egui::Key::Enter);
    assert!(app.layer_editor.editing.is_some());
    replace(&mut app, &ctx, "2");
    press(&mut app, &ctx, egui::Key::Enter);
    assert!(app.layer_editor.editing.is_none());
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .octave,
        2
    );
    press(&mut app, &ctx, egui::Key::L);
    press(&mut app, &ctx, egui::Key::Enter);
    replace(&mut app, &ctx, "-9.5");
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.layer_editor.open);
    assert!(app.layer_editor.editing.is_none());
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .volume_db,
        0.0
    );
    press(&mut app, &ctx, egui::Key::Enter);
    replace(&mut app, &ctx, "-9.5");
    press(&mut app, &ctx, egui::Key::Enter);
    press(&mut app, &ctx, egui::Key::A);
    let patch = &app.loaded.as_ref().unwrap().patches[1];
    assert_eq!(
        patch.layers.len(),
        2,
        "editing={:?}, error={:?}, db={}",
        app.layer_editor.editing.as_ref().map(|e| &e.text),
        app.player_selection.error,
        patch.layers[0].settings.volume_db
    );
    assert_eq!(patch.layers[1].settings.octave, 2);
    assert_eq!(patch.layers[1].settings.volume_db, -9.5);
    assert_ne!(app.layer_editor.selected, original);
    press(&mut app, &ctx, egui::Key::K);
    assert_eq!(app.layer_editor.selected, original);
    press(&mut app, &ctx, egui::Key::Delete);
    press(&mut app, &ctx, egui::Key::Delete);
    assert!(app.loaded.as_ref().unwrap().patches[1].layers.is_empty());
    assert!(app.layer_editor.open);
    press(&mut app, &ctx, egui::Key::A);
    let saved = cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[1],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    assert_eq!(saved.layers.len(), 1);
    assert_eq!(saved.layers[0].octave, 0);
    assert_eq!(app.loaded.as_ref().unwrap().patches[0], Patch::default());
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.layer_editor.open);
}

#[test]
fn invalid_numeric_edits_are_not_saved_and_help_owns_navigation() {
    let mut app = fixture();
    open(&mut app);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::L);
    press(&mut app, &ctx, egui::Key::L);
    press(&mut app, &ctx, egui::Key::Enter);
    replace(&mut app, &ctx, "NaN");
    press(&mut app, &ctx, egui::Key::Enter);
    assert!(app.layer_editor.editing.is_some());
    assert!(app.player_selection.error.is_some());
    assert_eq!(app.loaded.as_ref().unwrap().patches[0], Patch::default());
    press(&mut app, &ctx, egui::Key::Escape);
    frame(&mut app, &ctx, vec![egui::Event::Text("?".into())]);
    assert!(app.layer_editor.help);
    press(&mut app, &ctx, egui::Key::A);
    assert_eq!(app.loaded.as_ref().unwrap().patches[0].layers.len(), 1);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.layer_editor.open);
    assert!(!app.layer_editor.help);
}

#[test]
fn header_click_opens_layers_for_the_clicked_split_pane_without_top_button() {
    let mut app = fixture();
    app.track_view.split = Some([0, 1]);
    app.track_view.active = 0;
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let mut positions: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text().trim() == "Init Saw" => {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 2);
    assert!(!output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Layers")));
    positions.sort_by(|a, b| a.x.total_cmp(&b.x));
    let pos = positions[1];
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    assert_eq!(app.track, 1);
    assert_eq!(app.track_view.active, 1);
    assert!(app.layer_editor.open);
    assert!(app.player_selection.resident.is_none());
}

#[test]
fn shift_t_does_not_escape_other_overlays_or_text_input() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    app.comma_menu.open = true;
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::T, egui::Modifiers::SHIFT)],
    );
    assert!(!app.layer_editor.open);
    assert!(app.comma_menu.open);
    app.comma_menu.open = false;
    app.playback.mixer_open = true;
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![
            key(egui::Key::T, egui::Modifiers::SHIFT),
            egui::Event::Text("T".into()),
        ],
    );
    assert!(app.playback.mixer_open);
    assert!(!app.layer_editor.open);
}

#[test]
fn browser_application_preserves_selected_layer_and_keyboard_cursor() {
    let mut app = fixture();
    open(&mut app);
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::A);
    let id = app.layer_editor.selected.unwrap();
    let chosen = crate::test_support::referenced(cat_smf_editor::chain_settings::PluginState {
        plugin: cat_smf_editor::host::PluginKey {
            format: "CLAP".into(),
            id: "test".into(),
            name: "Chosen".into(),
            bundle_path: "X:/plugin.clap".into(),
            vendor: String::new(),
        },
        state: vec![1],
        preset_name: Some("Chosen".into()),
        reference: None,
        bypass: false,
    });
    crate::player::resident::apply(
        &mut app,
        cat_smf_editor::resident_ipc::Open::Layer {
            track: 0,
            layer: id,
            song: 0,
        },
        chosen,
    )
    .unwrap();
    frame(&mut app, &ctx, vec![]);
    assert!(app.layer_editor.open);
    assert_eq!(app.layer_editor.selected, Some(id));
    assert_eq!(app.layer_editor.field, 0);
    assert!(app.loaded.as_ref().unwrap().patches[0].layers[0]
        .instrument
        .is_none());
}

#[test]
fn f_toggles_the_output_analysis_over_layers_and_comma_f_outside() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::T, egui::Modifiers::SHIFT)],
    );
    assert!(app.layer_editor.open);
    press(&mut app, &ctx, egui::Key::F);
    assert!(app.analysis.is_open());
    assert!(app.layer_editor.open, "the layers stay usable beside it");
    press(&mut app, &ctx, egui::Key::F);
    assert!(!app.analysis.is_open());
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.layer_editor.open);
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::F);
    assert!(app.analysis.is_open());
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::F);
    assert!(!app.analysis.is_open());
}
