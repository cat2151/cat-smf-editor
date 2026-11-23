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
    let shared = offer_sound(
        out,
        target,
        work.path(),
        "Kit",
        Some(Kit {
            layer: 1,
            rows: None,
            muted: vec![],
            swing: Default::default(),
            notes,
        }),
    );
    (work, shared)
}

fn offer_sound(
    out: &mut Sender,
    target: Open,
    root: &std::path::Path,
    name: &str,
    kit: Option<Kit>,
) -> resident_ipc::SharedState {
    let bundle = root.join(format!("{name}.clap"));
    let patch = root.join(format!("{name}.preset"));
    std::fs::write(&bundle, b"fixture plugin identity").unwrap();
    std::fs::write(&patch, b"fixture patch identity").unwrap();
    crate::player::patch_roots::Assumed::set_root(root);
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: name.into(),
        name: name.into(),
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
            display: name.into(),
        }),
        plugin,
        state: vec![1, 2, 3],
        preset_name: Some(name.into()),
        bypass: false,
    };
    let shared = out.share(&chosen.state).unwrap();
    out.send(&ToEditor::Selected(Selected {
        target,
        kit,
        reference: chosen.reference.map(Box::new),
        plugin: chosen.plugin,
        preset_name: chosen.preset_name,
        state: shared.reference.clone(),
    }))
    .unwrap();
    shared
}

#[test]
fn challenge_retargets_the_seed_kick_and_opens_each_container_browser_in_order() {
    use crate::play::{playback::Mode, transport::repeat};
    use crate::view::patterns::workspace_tests::{context, frame, key};
    let ctx = context();
    let root = egui::ViewportId::ROOT;
    let directory = tempfile::tempdir().unwrap();
    let mut app = crate::view::tests::app(music());
    app.saved.directory = Some(directory.path().to_owned());
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    let _default_kit = crate::player::default_kit::fixture::provide(&mut app.state_cache, 36);
    app.new_challenge_song().unwrap();
    frame(&mut app, &ctx, root, vec![]);
    key(
        &mut app,
        &ctx,
        root,
        egui::Key::Enter,
        egui::Modifiers::NONE,
    );
    assert!(launched().is_empty(), "the duration dialog just closed");
    let focusing_root = || {
        ctx.data(|d| {
            d.get_temp::<Vec<egui::ViewportCommand>>(egui::Id::new(("workspace-commands", root)))
                .is_some_and(|commands| commands.contains(&egui::ViewportCommand::Focus))
        })
    };
    let mut restored_focus = focusing_root();
    for _ in 0..4 {
        frame(&mut app, &ctx, root, vec![]);
        if !launched().is_empty() {
            assert!(
                restored_focus,
                "editor focus must be restored before launch"
            );
            assert!(
                !focusing_root(),
                "the launch frame must not focus the editor"
            );
            break;
        }
        restored_focus |= focusing_root();
    }

    let (mut out, mut incoming) = resident_ipc::connect(&launched()[0]).unwrap();
    poll_until(&mut app, &ctx, connected);
    let Some(ToPlayer::OpenFiltered {
        open: target,
        random_patch_role: None,
        ..
    }) = incoming.receive().unwrap()
    else {
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
    let (patch_root, _shared) = offer_kit(
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
    let assert_preview = |app: &App, track| {
        assert_eq!(app.playback.repeat, Some((0, 1920)));
        let Some(Mode::Pattern(id)) = app.playback.repeat_mode else {
            panic!("expected a local pattern repeat");
        };
        let prepared = repeat::prepare(app, (0, 1920)).unwrap();
        assert_eq!(prepared.active.mode, Mode::Pattern(id));
        assert_eq!(prepared.active.end, 2.0);
        assert_eq!(prepared.parts.len(), 1);
        assert_eq!(prepared.parts[0].0, track);
        assert_eq!(prepared.parts[0].2.notes[0].start, 0.0);
    };
    assert_preview(&app, 1);
    out.send(&ToEditor::BrowserConfirmed { target }).unwrap();

    for (track, role, sound) in [
        (2, "bass", "Test Bass"),
        (3, "chord", "Test Piano"),
        (4, "lead", "Test Lead"),
    ] {
        poll_until(&mut app, &ctx, |a| a.track == track);
        assert_preview(&app, track);
        let Some(ToPlayer::OpenFiltered {
            open: target,
            random_patch_role,
            ..
        }) = incoming.receive().unwrap()
        else {
            panic!("expected browser for container track {track}");
        };
        assert_eq!(random_patch_role.as_deref(), Some(role));
        assert_eq!(
            target,
            Open::Layer {
                track,
                layer: 1,
                song: app.song_id
            }
        );
        let _shared = offer_sound(&mut out, target, patch_root.path(), sound, None);
        poll_until(&mut app, &ctx, |a| {
            a.loaded.as_ref().unwrap().patches[track].label() == sound
        });
        assert_eq!(
            incoming.receive::<ToPlayer>().unwrap(),
            Some(ToPlayer::Applied(Ok(())))
        );
        if track == 4 {
            app.patterns.workspace.song_cell.focus = false;
        }
        out.send(&ToEditor::BrowserConfirmed { target }).unwrap();
    }
    poll_until(&mut app, &ctx, |a| a.patterns.workspace.song_cell.focus);
    assert_eq!(app.playback.repeat, Some((0, 7680)));
    let prepared = repeat::prepare(&app, (0, 7680)).unwrap();
    assert_eq!(prepared.active.mode, Mode::Arrangement(None));
    assert_eq!(prepared.active.containers, None);
    assert_eq!(prepared.active.end, 8.0);
    assert_eq!(
        prepared
            .parts
            .iter()
            .map(|part| (part.0, part.2.notes[0].start))
            .collect::<Vec<_>>(),
        [(1, 0.0), (2, 2.0), (3, 4.0), (4, 6.0)]
    );
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
