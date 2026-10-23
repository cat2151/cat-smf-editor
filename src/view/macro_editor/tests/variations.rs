use super::*;

pub(super) fn key(app: &mut App, ctx: &egui::Context, key: egui::Key) {
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
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        },
        |_| {},
    );
}
#[test]
fn keyboard_variations_keep_independent_edits_live_bindings_and_undo() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let before = app.loaded.as_ref().unwrap().music.clone();
    app.macro_editor.session = Some(session(&app));
    let ctx = egui::Context::default();
    let binding = app.macro_editor.session.as_ref().unwrap().rows[0]
        .value
        .clone();
    key(&mut app, &ctx, egui::Key::D);
    assert_eq!(app.macro_editor.session.as_ref().unwrap().variation, 1);
    key(&mut app, &ctx, egui::Key::End);
    key(&mut app, &ctx, egui::Key::PageUp);
    assert_eq!(f64::from_bits(binding.load(Ordering::Relaxed)), 0.25);
    key(&mut app, &ctx, egui::Key::ArrowRight);
    key(&mut app, &ctx, egui::Key::PageUp); // Wrap to last.
    assert_eq!(f64::from_bits(binding.load(Ordering::Relaxed)), 1.0);
    key(&mut app, &ctx, egui::Key::PageDown);
    assert_eq!(f64::from_bits(binding.load(Ordering::Relaxed)), 0.375);
    key(&mut app, &ctx, egui::Key::PageDown);
    key(&mut app, &ctx, egui::Key::Enter);
    assert!(!app.macro_editor.open());
    let bytes = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let reopened = cat_smf_editor::ffi::parse(&bytes).unwrap();
    let saved = macro_values::read(&reopened.tracks[0]).unwrap();
    assert_eq!(saved.parameters[0].points[0].value, 1.0);
    assert_eq!(saved.variations[0].selected, 1);
    assert_eq!(saved.variations[0].snapshots[0][0].points[0].value, 0.375);
    let s = session(&app);
    let (snapshots, selected) = variations::restore(&saved, 1, &s.rows);
    assert_eq!(selected, 1);
    assert_eq!(snapshots, saved.variations[0].snapshots);
    app.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&before).unwrap(),
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!app.undo.available());
}
#[test]
fn cancel_discards_all_variation_edits_and_footer_is_always_visible() {
    let mut app = crate::view::tests::app(tracks::new_song());
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    app.macro_editor.session = Some(session(&app));
    let ctx = egui::Context::default();
    key(&mut app, &ctx, egui::Key::D);
    key(&mut app, &ctx, egui::Key::End);
    let output = ctx.run(Default::default(), |ctx| show(&mut app, ctx));
    let text = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("[Enter]") && text.contains("[Esc]"), "{text}");
    key(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.macro_editor.open());
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
    );
    assert!(!app.undo.available());
}
#[test]
fn invalid_variations_are_rejected_and_replaced_layers_clear_alternatives() {
    let app = crate::view::tests::app(tracks::new_song());
    let s = session(&app);
    let mut saved = Values {
        version: 1,
        parameters: vec![s.rows[0].saved()],
        variations: vec![s.saved_variations()],
    };
    saved.validate().unwrap();
    saved.variations[0].selected = 1;
    assert!(saved.validate().is_err());
    saved.variations[0].selected = 0;
    let mut other = saved.parameters.clone();
    other[0].target.layer = 2;
    saved.variations[0].snapshots.push(other);
    assert!(saved.validate().is_err());
    saved.variations[0].snapshots.pop();
    let mut track = app.loaded.as_ref().unwrap().music.tracks[0].clone();
    macro_values::write(&mut track, &saved).unwrap();
    let mut removed = s.patch.clone();
    removed.layers.clear();
    macro_values::retain_unchanged_layers(&mut track, &s.patch, &removed).unwrap();
    assert!(macro_values::read(&track).unwrap().variations.is_empty());
}

#[test]
#[ignore = "read-only integration check of the current local startup SMF"]
fn actual_startup_smf_preserves_existing_values_and_roundtrips_variations() {
    let root = crate::store::config::directory().unwrap();
    let startup = crate::store::startup::read(&root.join("status.json")).unwrap();
    let path = startup.last_mid.unwrap();
    let original = std::fs::read(&path).unwrap();
    use cat_smf_editor::patch_roots::Roots;
    let table: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    for plugin in &table.plugins {
        crate::player::patch_roots::Assumed
            .plugin(
                &cat_smf_editor::host::PluginKey {
                    format: plugin.format.clone(),
                    id: plugin.id.clone(),
                    name: plugin.id.clone(),
                    vendor: String::new(),
                    bundle_path: plugin.bundle_path.clone(),
                },
                false,
            )
            .unwrap();
    }
    let surge = table
        .plugins
        .iter()
        .find(|p| p.id == "org.surge-synth-team.surge-xt")
        .unwrap();
    crate::player::patch_roots::Assumed::set_root(std::path::Path::new(&surge.roots[0]));

    let mut app = crate::view::tests::app(tracks::new_song());
    app.loaded = None;
    app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(root.clone()));
    app.patch_roots = crate::player::patch_roots::Cache::new(Some(root.clone()));
    app.open_mid(path.clone());
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let owner = app
        .loaded
        .as_ref()
        .unwrap()
        .music
        .tracks
        .iter()
        .position(|t| macro_values::read(t).is_ok_and(|v| !v.parameters.is_empty()))
        .unwrap();
    app.select_track(owner);
    let values = macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[owner]).unwrap();
    let layer = values.parameters[0].target.layer;
    let mut s = session(&app);
    s.owner = owner;
    s.layer = layer;
    s.patch = app.loaded.as_ref().unwrap().patches[owner].clone();
    s.rows = values
        .parameters
        .iter()
        .filter(|c| c.target.layer == layer)
        .map(|c| Row {
            curve: c.clone(),
            value: Arc::new(AtomicU64::new(c.points[0].value.to_bits())),
            bipolar: false,
            readback: None,
        })
        .collect();
    (s.variations, s.variation) = variations::restore(&values, layer, &s.rows);
    s.duplicate_variation();
    s.rows[0].set(0.75);
    app.macro_editor.session = Some(s);
    finish(&mut app, true);
    assert!(!app.macro_editor.open());
    let music = &app.loaded.as_ref().unwrap().music;
    let work = tempfile::tempdir().unwrap();
    let copy = work.path().join("variations.mid");
    std::fs::write(&copy, cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
    let mut restarted = crate::view::tests::app(tracks::new_song());
    restarted.loaded = None;
    restarted.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(root.clone()));
    restarted.patch_roots = crate::player::patch_roots::Cache::new(Some(root));
    restarted.open_initial(Some(copy));
    assert!(restarted.load_error.is_none(), "{:?}", restarted.load_error);
    let restored =
        macro_values::read(&restarted.loaded.as_ref().unwrap().music.tracks[owner]).unwrap();
    assert_eq!(
        restored.variations[0].snapshots[0],
        values
            .parameters
            .iter()
            .filter(|c| c.target.layer == layer)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(restored.parameters[0].points[0].value, 0.75);
    assert_eq!(restored.variations[0].selected, 1);
    assert_eq!(std::fs::read(path).unwrap(), original);
}
