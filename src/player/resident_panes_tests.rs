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
