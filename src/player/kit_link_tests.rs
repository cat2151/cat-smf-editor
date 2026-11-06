//! Keyboard container choice and real resident IPC compose without an initial four-lane side effect.
use super::*;
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::{Kit, Note},
    score::pattern::Catalog,
};
#[test]
fn kit_first_pattern_choice_confirm_and_cancel_are_atomic_over_resident_link() {
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    app.select_track(1);
    app.note_mode = true;
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    crate::view::patterns::library::begin_add(&mut app, &ctx).unwrap();
    for key in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::Enter] {
        crate::view::palette::tests::press(&mut app, &ctx, key);
    }
    assert!(!app.patterns.confirming_creation());
    assert!(app.player_selection.kit_open);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    let name = launched().last().unwrap().clone();
    let (mut out, mut incoming) = resident_ipc::connect(&name).unwrap();
    poll_until(&mut app, &ctx, connected);
    let ToPlayer::OpenFiltered { open: target, .. } = incoming.receive().unwrap().unwrap() else {
        panic!("kit browser request");
    };
    assert!(matches!(target, Open::Kit { layer: None, .. }));
    let work = tempfile::tempdir().unwrap();
    let bundle = work.path().join("kit.clap");
    let patch = work.path().join("kit.preset");
    std::fs::write(&bundle, b"fixture plugin identity").unwrap();
    std::fs::write(&patch, b"fixture patch identity").unwrap();
    crate::player::patch_roots::Assumed::set_root(work.path());
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "kit".into(),
        name: "Kit".into(),
        vendor: String::new(),
        bundle_path: bundle.to_string_lossy().into(),
    };
    let chosen = PluginState {
        reference: Some(cat_smf_editor::patch_reference::PatchReference {
            plugin: plugin.clone(),
            source: cat_smf_editor::patch_reference::Source::Catalog {
                path: patch.clone(),
                file: patch,
            },
            display: "Kit".into(),
        }),
        plugin,
        state: vec![1, 2, 3],
        preset_name: Some("Kit".into()),
        bypass: false,
    };
    let shared = out.share(&chosen.state).unwrap();
    out.send(&ToEditor::Selected(Selected {
        target,
        kit: Some(Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes: [0, 1, 2, 3, 4, 127]
                .into_iter()
                .map(|note| Note {
                    note,
                    name: format!("Source {note}"),
                    velocity: 100,
                })
                .collect(),
        }),
        reference: chosen.reference.map(Box::new),
        plugin: chosen.plugin,
        preset_name: chosen.preset_name,
        state: shared.reference.clone(),
    }))
    .unwrap();
    poll_until(&mut app, &ctx, |a| {
        a.loaded
            .as_ref()
            .unwrap()
            .patches
            .iter()
            .any(|p| p.kit.is_some())
    });
    assert_eq!(
        incoming.receive::<ToPlayer>().unwrap(),
        Some(ToPlayer::Applied(Ok(())))
    );
    out.send(&ToEditor::KitClosed { target }).unwrap();
    poll_until(&mut app, &ctx, |a| !a.player_selection.kit_open);
    let c = Catalog::read(&app.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(c.sources.len(), 1);
    assert_eq!(c.patterns.len(), 1);
    assert!(app
        .loaded
        .as_ref()
        .unwrap()
        .patches
        .iter()
        .all(|p| p.drums.is_none()));
    assert_eq!(
        app.loaded.as_ref().unwrap().patches[app.track].layers.len(),
        1
    );
    assert!(crate::view::drums::is_open(&app));
    let _ = ctx.run(Default::default(), |ctx| {
        crate::view::drums::show(&mut app, ctx)
    });
    crate::view::palette::tests::press(&mut app, &ctx, egui::Key::Enter);
    assert!(app.loaded.as_ref().unwrap().visible().tracks[app.track]
        .events
        .iter()
        .any(|e| cat_smf_editor::score::note_view::note_on(e) && e.data[0] == 49));
    let before_cancel =
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    crate::player::kit_selection::open(&mut app, true, false);
    let ToPlayer::OpenFiltered { open: target, .. } = incoming.receive().unwrap().unwrap() else {
        panic!("kit browser request");
    };
    out.send(&ToEditor::KitClosed { target }).unwrap();
    poll_until(&mut app, &ctx, |a| !a.player_selection.kit_open);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before_cancel
    );
}
