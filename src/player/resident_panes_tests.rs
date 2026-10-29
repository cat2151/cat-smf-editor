use super::*;

#[test]
fn pane_changes_persist_without_sound_selection_and_targets_stay_independent() {
    let ctx = egui::Context::default();
    let mut app = crate::view::tests::app(music());
    let root = std::env::temp_dir().join(format!("editor-panes-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    app.config = crate::store::config::Store::open(root.join("status.json"));
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    app.select_track(1);
    open(&mut app, Browser::Patch);
    let (mut out, mut input) = resident_ipc::connect(launched().last().unwrap()).unwrap();
    poll_until(&mut app, &ctx, connected);
    let Some(ToPlayer::OpenFiltered { context: patch, .. }) = input.receive().unwrap() else {
        panic!("open")
    };
    let chosen = ["Bass".to_string(), "Warm".to_string()];
    out.send(&ToEditor::Panes {
        context: patch.clone(),
        panes: chosen.clone(),
        query: "bass plugin:alpha -bright".into(),
    })
    .unwrap();
    poll_until(&mut app, &ctx, |app| {
        app.config.browser_panes.get(&patch) == Some(&chosen)
    });
    let restored = crate::store::config::Store::open(root.join("status.json"));
    assert_eq!(restored.browser_panes.get(&patch), Some(&chosen));
    assert_eq!(
        restored.browser_queries.get(&patch).unwrap(),
        "bass plugin:alpha -bright"
    );
    app.config = restored;
    for browser in [Browser::Effect, Browser::MasterEffect] {
        open(&mut app, browser);
        let Some(ToPlayer::OpenFiltered {
            context,
            panes,
            query,
            ..
        }) = input.receive().unwrap()
        else {
            panic!("open")
        };
        assert_ne!(context, patch);
        assert_eq!(panes, [String::new(), String::new()]);
        assert!(query.is_empty());
        let query = if browser == Browser::MasterEffect {
            "hall -plate"
        } else {
            "delay"
        };
        out.send(&ToEditor::Panes {
            context: context.clone(),
            panes,
            query: query.into(),
        })
        .unwrap();
        poll_until(&mut app, &ctx, |app| {
            app.config
                .browser_queries
                .get(&context)
                .is_some_and(|saved| saved == query)
        });
        let restored = crate::store::config::Store::open(root.join("status.json"));
        assert_eq!(restored.browser_queries.get(&context).unwrap(), query);
    }
    open(&mut app, Browser::Patch);
    let Some(ToPlayer::OpenFiltered {
        context,
        panes,
        query,
        ..
    }) = input.receive().unwrap()
    else {
        panic!("open")
    };
    assert_eq!(context, patch);
    assert_eq!(panes, chosen);
    assert_eq!(query, "bass plugin:alpha -bright");
    let mut target = app
        .player_selection
        .resident
        .as_ref()
        .unwrap()
        .target
        .clone();
    target.open = Open::Patch { track: 0 };
    assert_ne!(pane_context(&target), patch);
    target.open = Open::Patch { track: 1 };
    target.original = root.join("other.mid");
    assert_ne!(pane_context(&target), patch);
    drop(app);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn named_cursor_is_sent_to_a_new_player_after_editor_restart() {
    let ctx = egui::Context::default();
    let root = std::env::temp_dir().join(format!("editor-cursor-{}", std::process::id()));
    let make_app = || {
        let mut app = crate::view::tests::app(music());
        app.config = crate::store::config::Store::open(root.join("status.json"));
        app.player_selection.executable = Some(std::env::current_exe().unwrap());
        app.player_selection.launch = Some(record);
        app.select_track(1);
        app
    };
    let patch = PatchIdentity {
        format: "CLAP".into(),
        plugin_id: "a".into(),
        path: "X:/patches/a/P3".into(),
        display: "P3".into(),
    };
    let mut app = make_app();
    open(&mut app, Browser::Patch);
    let (mut out, mut input) = resident_ipc::connect(launched().last().unwrap()).unwrap();
    poll_until(&mut app, &ctx, connected);
    let Some(ToPlayer::OpenFiltered {
        context, cursor, ..
    }) = input.receive().unwrap()
    else {
        panic!("open")
    };
    assert!(cursor.is_none());
    out.send(&ToEditor::Cursor {
        context: context.clone(),
        patch: Some(patch.clone()),
    })
    .unwrap();
    poll_until(&mut app, &ctx, |app| {
        app.config.patch_cursors.get(&context) == Some(&patch)
    });
    drop(app);
    drop(out);
    drop(input);
    let mut app = make_app();
    assert_eq!(app.config.patch_cursors.get(&context), Some(&patch));
    open(&mut app, Browser::Patch);
    let (out, mut input) = resident_ipc::connect(launched().last().unwrap()).unwrap();
    poll_until(&mut app, &ctx, connected);
    let Some(ToPlayer::OpenFiltered {
        context: restored,
        cursor,
        ..
    }) = input.receive().unwrap()
    else {
        panic!("open")
    };
    assert_eq!(restored, context);
    assert_eq!(cursor, Some(patch));
    drop(app);
    drop(out);
    drop(input);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn drum_rows_open_normal_browser_with_role_filters_and_preserve_repeat_on_preview() {
    use cat_smf_editor::score::instrument::ResolvedLayer;
    let mut app = crate::view::tests::app(music());
    let settings = cat_smf_editor::score::drums::Settings::default();
    let patch = Settings {
        drums: Some(settings.clone()),
        layers: (1..=4)
            .map(|id| ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        effects: vec![],
    };
    crate::player::selection::store_track(&mut app, 1, &patch).unwrap();
    app.track = 1;
    crate::view::drums::open(&mut app);
    app.player_selection.executable = Some(std::env::current_exe().unwrap());
    app.player_selection.launch = Some(record);
    let ctx = egui::Context::default();
    assert!(app.playback.repeat.is_none());
    crate::view::palette::tests::press(&mut app, &ctx, egui::Key::R);
    crate::player::selection::open_layer(&mut app, 1);
    assert!(app.player_selection.error.is_none());
    let (mut out, mut input) = resident_ipc::connect(launched().last().unwrap()).unwrap();
    poll_until(&mut app, &ctx, connected);
    for (row, role) in ["kick", "snare", "hi-hat", "percussion"].iter().enumerate() {
        if row > 0 {
            crate::player::selection::open_layer(&mut app, row as u64 + 1);
        }
        let Some(ToPlayer::OpenFiltered {
            open, panes, query, ..
        }) = input.receive().unwrap()
        else {
            panic!("open");
        };
        assert_eq!(panes, ["Drum tracks", role]);
        assert_eq!(query, "plugin:surgext");
        let plugin = PluginKey {
            format: "CLAP".into(),
            id: "synth".into(),
            name: "synth".into(),
            vendor: String::new(),
            bundle_path: "X:/plugins/synth.clap".into(),
        };
        let shared = out.share(&[1, 2, 3]).unwrap();
        out.send(&ToEditor::Selected(Selected {
            target: open,
            plugin: plugin.clone(),
            reference: Some(Box::new(cat_smf_editor::patch_reference::PatchReference {
                plugin,
                source: cat_smf_editor::patch_reference::Source::Catalog {
                    path: "X:/patches/a.preset".into(),
                    file: "X:/patches/a.preset".into(),
                },
                display: role.to_string(),
            })),
            preset_name: Some(role.to_string()),
            state: shared.reference.clone(),
        }))
        .unwrap();
        poll_until(&mut app, &ctx, |a| {
            a.loaded.as_ref().unwrap().patches[1].layers[row]
                .instrument
                .is_some()
        });
        assert_eq!(input.receive().unwrap(), Some(ToPlayer::Applied(Ok(()))));
        assert_eq!(app.playback.repeat, Some((0, 1920)));
        assert_eq!(
            app.playback
                .pending
                .as_ref()
                .unwrap()
                .parts
                .iter()
                .map(|p| p.0)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            app.loaded.as_ref().unwrap().patches[1].drums,
            Some(settings.clone())
        );
    }
}
