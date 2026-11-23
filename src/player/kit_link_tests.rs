//! Kit selection and challenge setup through the resident pipe.
use super::*;
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::{Kit, Note},
    score::pattern::Catalog,
};
fn offer_kit(
    out: &mut Sender,
    target: Open,
    notes: Vec<Note>,
) -> (tempfile::TempDir, resident_ipc::SharedState) {
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
            notes,
        }),
        reference: chosen.reference.map(Box::new),
        plugin: chosen.plugin,
        preset_name: chosen.preset_name,
        state: shared.reference.clone(),
    }))
    .unwrap();
    (work, shared)
}

#[test]
fn challenge_retargets_the_seed_kick_and_opens_each_container_browser_in_order() {
    let ctx = egui::Context::default();
    let directory = tempfile::tempdir().unwrap();
    let mut app = crate::view::tests::app(music());
    app.saved.directory = Some(directory.path().to_owned());
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    let _default_kit = crate::player::default_kit::fixture::provide(&mut app.state_cache, 36);
    app.new_challenge_song().unwrap();
    crate::view::palette::tests::frame(&mut app, &ctx, vec![]);
    crate::view::palette::tests::press(&mut app, &ctx, egui::Key::Enter);

    let (mut out, mut incoming) = resident_ipc::connect(launched().last().unwrap()).unwrap();
    poll_until(&mut app, &ctx, connected);
    let Some(ToPlayer::OpenFiltered { open: target, .. }) = incoming.receive().unwrap() else {
        panic!("expected Drum kit browser");
    };
    assert!(matches!(
        target,
        Open::Kit {
            track: 1,
            layer: Some(1),
            ..
        }
    ));
    let kick = 24;
    let (_work, _shared) = offer_kit(
        &mut out,
        target,
        vec![Note {
            note: kick,
            name: "Kick 1".into(),
            velocity: 100,
        }],
    );
    poll_until(&mut app, &ctx, |a| {
        a.loaded.as_ref().unwrap().patches[1].label() == "Kit"
    });
    assert_eq!(
        incoming.receive::<ToPlayer>().unwrap(),
        Some(ToPlayer::Applied(Ok(())))
    );
    let music = &app.loaded.as_ref().unwrap().music;
    let pattern = Catalog::read(music)
        .unwrap()
        .patterns
        .into_values()
        .next()
        .unwrap();
    let note = music.tracks[1]
        .events
        .iter()
        .find(|e| cat_smf_editor::score::note_view::note_on(e))
        .unwrap();
    assert_eq!((note.tick, note.data[0]), (pattern.tick, kick));
    assert_eq!(app.playback.repeat, Some((0, 1920)));
    out.send(&ToEditor::KitClosed { target }).unwrap();

    for track in 2..=4 {
        poll_until(&mut app, &ctx, |a| a.track == track);
        let Some(ToPlayer::OpenFiltered { open: target, .. }) = incoming.receive().unwrap() else {
            panic!("expected browser for container track {track}");
        };
        assert_eq!(
            target,
            Open::Layer {
                track,
                layer: 1,
                song: app.song_id
            }
        );
        if track == 4 {
            app.patterns.workspace.song_cell.focus = false;
        }
        out.send(&ToEditor::BrowserClosed { target }).unwrap();
    }
    poll_until(&mut app, &ctx, |a| a.patterns.workspace.song_cell.focus);
    assert_eq!(app.playback.repeat, Some((0, 1920)));
    assert_eq!(launched().len(), 1);
}

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
    let (_work, _shared) = offer_kit(
        &mut out,
        target,
        [0, 1, 2, 3, 4, 127]
            .into_iter()
            .map(|note| Note {
                note,
                name: format!("Source {note}"),
                velocity: 100,
            })
            .collect(),
    );
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
