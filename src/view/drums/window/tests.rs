use super::*;

fn fixture(kind: Kind) -> App {
    match kind {
        Kind::Drums => super::super::tests::fixture(),
        Kind::Kit => {
            let mut app = super::super::kit_edit_tests::fixture(4);
            super::super::open(&mut app);
            app
        }
        Kind::Melody => super::super::melody_tests::fixture(),
    }
}

fn frame(app: &mut App, ctx: &egui::Context, focused: bool) -> egui::FullOutput {
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1800.0, 1000.0),
        )),
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .focused = Some(focused);
    ctx.run(input, |ctx| super::super::show(app, ctx))
}

#[test]
fn every_window_restores_independent_positions_after_exit_and_close_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let positions = [
        (Kind::Drums, [120.0, 160.0]),
        (Kind::Kit, [320.0, 260.0]),
        (Kind::Melody, [220.0, 210.0]),
    ];
    let mut store = Store::open(path.clone());
    for (kind, position) in positions {
        store.drum_windows.insert(kind.key().into(), position);
    }
    store.persist();
    for (kind, position) in positions {
        let mut app = fixture(kind);
        app.config = Store::open(path.clone());
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&mut app, &ctx, true);
        }
        assert_eq!(
            ctx.memory(|m| m.area_rect(kind.id())).unwrap().min,
            Pos2::from(position)
        );
        // A real title-bar drag must update the persisted place, without snapping back.
        let start = ctx.memory(|m| m.area_rect(kind.id())).unwrap().min + egui::vec2(150.0, 12.0);
        for (at, pressed) in [
            (start, Some(true)),
            (start + egui::vec2(45.0, 35.0), None),
            (start + egui::vec2(45.0, 35.0), Some(false)),
        ] {
            let mut events = vec![egui::Event::PointerMoved(at)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                });
            }
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        egui::vec2(1800.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |ctx| super::super::show(&mut app, ctx),
            );
        }
        frame(&mut app, &ctx, true);
        let moved = app.config.drum_windows[kind.key()];
        assert_ne!(moved, position);
        super::super::close(&mut app);
        super::super::open(&mut app);
        frame(&mut app, &ctx, true);
        assert_eq!(app.config.drum_windows[kind.key()], moved);
        eframe::App::on_exit(&mut app, None);
        let mut restarted = fixture(kind);
        restarted.config = Store::open(path.clone());
        let fresh = egui::Context::default();
        for _ in 0..3 {
            frame(&mut restarted, &fresh, true);
        }
        assert_eq!(
            fresh.memory(|m| m.area_rect(kind.id())).unwrap().min,
            Pos2::from(moved)
        );
        let other = match kind {
            Kind::Drums => Kind::Kit,
            Kind::Kit => Kind::Melody,
            Kind::Melody => Kind::Drums,
        };
        assert_eq!(
            restarted.config.drum_windows[other.key()],
            app.config.drum_windows[other.key()]
        );
    }
}

#[test]
fn kit_size_survives_resize_close_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let mut app = fixture(Kind::Kit);
    app.config = Store::open(path.clone());
    app.config
        .drum_window_sizes
        .insert(Kind::Kit.key().into(), [920.0, 420.0]);
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut app, &ctx, true);
    }
    let before = ctx.memory(|m| m.area_rect(Kind::Kit.id())).unwrap();
    let start = before.max - egui::vec2(2.0, 2.0);
    for (at, pressed) in [
        (start, Some(true)),
        (start + egui::vec2(80.0, 60.0), None),
        (start + egui::vec2(80.0, 60.0), Some(false)),
    ] {
        let mut events = vec![egui::Event::PointerMoved(at)];
        if let Some(pressed) = pressed {
            events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
        let _ = ctx.run(
            egui::RawInput {
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(1800.0, 1000.0),
                )),
                ..Default::default()
            },
            |ctx| super::super::show(&mut app, ctx),
        );
    }
    frame(&mut app, &ctx, true);
    let resized = ctx.memory(|m| m.area_rect(Kind::Kit.id())).unwrap();
    assert!(resized.width() > before.width() + 50.0);
    assert!(resized.height() > before.height() + 30.0);
    super::super::close(&mut app);
    super::super::open(&mut app);
    frame(&mut app, &ctx, true);
    assert_eq!(
        ctx.memory(|m| m.area_rect(Kind::Kit.id())).unwrap().size(),
        resized.size()
    );
    let saved_size = app.config.drum_window_sizes[Kind::Kit.key()];
    app.drum_editor.field = 1;
    super::super::tests::press(&mut app, &ctx, egui::Key::Enter);
    frame(&mut app, &ctx, true);
    assert_eq!(app.config.drum_window_sizes[Kind::Kit.key()], saved_size);
    super::super::tests::press(&mut app, &ctx, egui::Key::Escape);
    eframe::App::on_exit(&mut app, None);
    let mut restarted = fixture(Kind::Kit);
    restarted.config = Store::open(path);
    let fresh = egui::Context::default();
    for _ in 0..3 {
        frame(&mut restarted, &fresh, true);
    }
    assert_eq!(
        fresh
            .memory(|m| m.area_rect(Kind::Kit.id()))
            .unwrap()
            .size(),
        resized.size()
    );
}

#[test]
fn default_kit_has_only_a_small_margin_after_step_sixteen() {
    let mut app = fixture(Kind::Kit);
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut app, &ctx, true);
    }
    let last = ctx.data(|d| {
        d.get_temp::<egui::Rect>(egui::Id::new(("kit-cell", 0u8, 15usize)))
            .unwrap()
    });
    let clip = ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new("kit-clip")).unwrap());
    let gap = clip.right() - last.right();
    assert!(
        (0.0..=24.0).contains(&gap),
        "right margin {gap}: clip {clip:?}, last {last:?}"
    );
}

#[test]
fn both_titles_follow_focus_and_browser_ownership_without_coloring_contents() {
    for kind in [Kind::Drums, Kind::Kit] {
        let mut app = fixture(kind);
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let original = ctx.style();
        let mut title_colors = Vec::new();
        for (focused, browser) in [(true, false), (false, false), (true, true), (true, false)] {
            app.player_selection.open = browser;
            let mut output = frame(&mut app, &ctx, focused);
            for _ in 0..2 {
                output = frame(&mut app, &ctx, focused);
            }
            assert_eq!(ctx.style(), original);
            // The title is the filled rectangle at the top of the window.
            let rect = ctx.memory(|m| m.area_rect(kind.id())).unwrap();
            let color = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::epaint::Shape::Rect(r)
                        if r.rect.contains(rect.min + egui::vec2(100.0, 10.0))
                            && r.rect.height() < 50.0 =>
                    {
                        Some(r.fill)
                    }
                    _ => None,
                })
                .expect("title background");
            title_colors.push(color);
        }
        assert_ne!(title_colors[0], title_colors[1]);
        assert_eq!(title_colors[1], title_colors[2]);
        assert_eq!(title_colors[0], title_colors[3]);
    }
}

#[test]
fn both_sequencers_keep_common_keyboard_navigation_and_close_behavior() {
    use egui::Key;
    for kind in [Kind::Drums, Kind::Kit] {
        let mut app = fixture(kind);
        let ctx = egui::Context::default();
        for (key, page) in [(Key::PageDown, 1), (Key::PageUp, 0)] {
            super::super::tests::press(&mut app, &ctx, key);
            assert_eq!(app.drum_editor.page, page);
        }
        app.drum_editor.last = 2;
        super::super::tests::press(&mut app, &ctx, Key::CloseBracket);
        assert_eq!(app.drum_editor.first, 2);
        super::super::tests::press(&mut app, &ctx, Key::OpenBracket);
        assert_eq!(app.drum_editor.first, 1);
        super::super::tests::press(&mut app, &ctx, Key::Escape);
        assert!(!super::super::is_open(&app));
    }
}

#[test]
fn shell_restores_content_style_and_modal_ownership_for_both_kinds() {
    for kind in [Kind::Drums, Kind::Kit] {
        let mut app = fixture(kind);
        let ctx = egui::Context::default();
        let original = ctx.style();
        let _ = ctx.run(Default::default(), |ctx| {
            kind.show(&mut app, ctx, None, &mut true, |_, ui| {
                assert_eq!(ui.style(), &original);
            });
            assert_eq!(ctx.style(), original);
            crate::view::modal::mark_owned(ctx);
            assert!(!super::super::title::active(&app, ctx));
        });
    }
}

#[test]
fn existing_status_needs_no_migration_and_keeps_all_settings() {
    let source = std::env::var_os("CAT_SMF_DRUM_STATUS_FIXTURE");
    let original: serde_json::Value = if let Some(source) = &source {
        serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap()
    } else {
        serde_json::json!({"session": null, "meta_text_visible": false})
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    if let Some(source) = source {
        let startup = std::path::Path::new(&source).with_file_name("startup.json");
        if startup.exists() {
            std::fs::copy(startup, dir.path().join("startup.json")).unwrap();
        }
    }
    let mut store = Store::open(path.clone());
    assert!(store.error.is_none(), "{:?}", store.error);
    Kind::Drums.remember(&mut store, Pos2::new(123.0, 234.0));
    Kind::Kit.remember(&mut store, Pos2::new(456.0, 345.0));
    store.persist();
    assert!(store.error.is_none(), "{:?}", store.error);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (key, value) in original.as_object().unwrap() {
        if key != "drum_windows" {
            assert_eq!(&saved[key], value, "existing field {key}");
        }
    }
    let reopened = Store::open(path);
    assert_eq!(
        Kind::Drums.position(&reopened),
        Some(Pos2::new(123.0, 234.0))
    );
    assert_eq!(Kind::Kit.position(&reopened), Some(Pos2::new(456.0, 345.0)));
}
