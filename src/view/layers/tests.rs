use super::*;
#[test]
fn duplicates_have_independent_ids_and_deleted_ids_are_not_recycled() {
    let mut patch = Patch::default();
    add(&mut patch).unwrap();
    let id = patch.layers.pop().unwrap().settings.id;
    add(&mut patch).unwrap();
    assert_ne!(patch.layers[1].settings.id, id);
    patch.layers[1].settings.octave = -2;
    assert_eq!(patch.layers[0].settings.octave, 0);
}
#[test]
fn modal_consumes_score_shortcuts_without_opening_windows_or_audio() {
    let mut app = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }],
    });
    open(&mut app);
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::T,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        show(&mut app, ctx);
        assert!(ctx.input(|i| i
            .events
            .iter()
            .all(|e| !matches!(e, egui::Event::Key { .. }))));
    });
    assert!(app.layer_editor.open);
    assert!(app.player_selection.resident.is_none());
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| show(app, ctx),
    )
}

fn click_text(app: &mut App, ctx: &egui::Context, label: &str) {
    // These are egui input data, never OS input or an application window.
    let output = frame(app, ctx, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "Missing control {label}; rendered text: {:?}",
                output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            )
        });
    for pressed in [true, false] {
        frame(
            app,
            ctx,
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
}

#[test]
fn rendered_controls_add_remove_last_layer_and_recreate_from_empty() {
    let mut app = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }],
    });
    open(&mut app);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    click_text(&mut app, &ctx, "+");
    assert_eq!(app.loaded.as_ref().unwrap().patches[0].layers.len(), 2);
    click_text(&mut app, &ctx, "−");
    click_text(&mut app, &ctx, "−");
    assert!(app.loaded.as_ref().unwrap().patches[0].layers.is_empty());
    assert!(cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[0],
        &mut crate::player::patch_roots::Assumed
    )
    .unwrap()
    .layers
    .is_empty());
    click_text(&mut app, &ctx, "+");
    assert_eq!(app.loaded.as_ref().unwrap().patches[0].layers.len(), 1);
    assert!(
        app.player_selection.error.is_none(),
        "{:?}",
        app.player_selection.error
    );
}
