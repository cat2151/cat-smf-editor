use super::*;
use cat_smf_editor::{
    model::{Event, Music, Track},
    smf_io,
};

#[cfg(windows)]
#[test]
fn path_cells_simplify_safe_paths_without_changing_history() {
    let long = format!(r"\\?\X:\{}\{}\song.mid", "a".repeat(150), "b".repeat(150));
    for (stored, displayed) in [
        (r"\\?\X:\音楽\song.mid", r"X:\音楽\song.mid"),
        (r"X:\music\song.mid", r"X:\music\song.mid"),
        (long.as_str(), long.as_str()),
        (r"\\?\X:\music.\song.mid", r"\\?\X:\music.\song.mid"),
        (r"\\?\X:\NUL.mid", r"\\?\X:\NUL.mid"),
        (
            r"\\?\UNC\server\share\song.mid",
            r"\\?\UNC\server\share\song.mid",
        ),
    ] {
        let entry = Entry {
            path: stored.into(),
            opened: 0,
            title: String::new(),
            seconds: 0.0,
            tracks: 1,
        };
        assert_eq!(cells(&entry, 0, 0)[6], displayed);
        assert_eq!(entry.path, std::path::Path::new(stored));
    }
}

pub(super) fn music() -> Music {
    Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0xff,
                    meta_type: 3,
                    data: b"Example".to_vec(),
                },
                Event {
                    tick: 0,
                    delta: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![60, 100],
                },
                Event {
                    tick: 960,
                    delta: 960,
                    status: 0x80,
                    meta_type: 0,
                    data: vec![60, 0],
                },
                Event {
                    tick: 960,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    }
}

pub(super) fn press(
    app: &mut App,
    ctx: &egui::Context,
    key: Key,
    modifiers: Modifiers,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

#[test]
fn all_direct_keys_preserve_case_and_reserve_navigation() {
    for i in 0..61 {
        let label = shortcut(i);
        let ch = label.chars().next().unwrap();
        let modifiers = if matches!(ch, 'j' | 'k') {
            Modifiers::ALT
        } else if ch.is_ascii_uppercase() {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        let key = Key::from_name(&ch.to_ascii_uppercase().to_string()).unwrap();
        assert_eq!(direct(key, modifiers), Some(i), "{label}");
    }
    assert_eq!(direct(Key::J, Modifiers::NONE), None);
    assert_eq!(direct(Key::K, Modifiers::NONE), None);
    assert_eq!(direct(Key::A, Modifiers::CTRL), None);
    assert_eq!(shortcut(61), "");
}

#[test]
fn timestamp_boundaries_use_six_distinct_monokai_colors() {
    let values = [
        (0, "1m"),
        (3599, "59m"),
        (3600, "1h"),
        (86400, "1d"),
        (604800, "1w"),
        (2592000, "1mon"),
        (31536000, "1y"),
    ];
    for (seconds, label) in values {
        assert_eq!(age(seconds).0, label);
    }
    let colors: std::collections::HashSet<_> = values[1..].iter().map(|(s, _)| age(*s).1).collect();
    assert_eq!(colors.len(), 6);
}

#[test]
fn preview_cancel_and_confirm_preserve_unsaved_document_and_persist_order() {
    let root = std::env::temp_dir().join(format!("smf-recent-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = music();
    let first = root.join("first.mid");
    let second = root.join("second.mid");
    for path in [&first, &second] {
        std::fs::write(path, smf_io::encode(&source).unwrap()).unwrap();
    }
    let mut app = crate::view::tests::app(source.clone());
    app.loaded = None;
    app.recent.history = History::open(root.join("recent-files.json"));
    app.recent.prefetch.directory = Some(root.join("wav"));
    app.open_mid(first.clone());
    app.open_mid(second.clone());
    assert_eq!(app.recent.history.entries.len(), 2);
    assert_eq!(app.recent.history.entries[0].title, "Example");
    assert_eq!(app.recent.history.entries[0].seconds, 1.0);
    app.edit_music(|m| {
        m.tracks[0].events[1].data[0] = 67;
        Ok(())
    })
    .unwrap();
    app.cursor_row = Some(1);
    let loaded_path = app.loaded.as_ref().unwrap().original.clone();
    let order: Vec<_> = app
        .recent
        .history
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.opened))
        .collect();
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::P, Modifiers::CTRL);
    assert!(app.recent.open);
    assert_eq!(app.recent.cursor, 0);
    press(&mut app, &ctx, Key::J, Modifiers::NONE);
    assert_eq!(app.recent.cursor, 1);
    assert!(app.recent.listen.is_some());
    press(&mut app, &ctx, Key::F2, Modifiers::NONE);
    assert!(!app.track_view.go_open && !app.edit_menu.open);
    assert_eq!(app.loaded.as_ref().unwrap().original, loaded_path);
    assert_eq!(
        app.loaded.as_ref().unwrap().music.tracks[0].events[1].data[0],
        67
    );
    assert!(app.saved.dirty());
    assert_eq!(app.cursor_row, Some(1));
    assert_eq!(
        app.recent
            .history
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.opened))
            .collect::<Vec<_>>(),
        order
    );
    press(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    assert!(!app.recent.open && app.recent.listen.is_none());
    assert!(app.saved.dirty());
    assert_eq!(app.cursor_row, Some(1));
    press(&mut app, &ctx, Key::P, Modifiers::CTRL);
    press(&mut app, &ctx, Key::Num2, Modifiers::NONE);
    assert!(!app.recent.open);
    assert!(app.config.navigation.departure.is_some());
    assert_eq!(app.loaded.as_ref().unwrap().original, loaded_path);
    press(&mut app, &ctx, Key::N, Modifiers::NONE);
    crate::view::opening::tests::finish(&mut app, &ctx);
    assert_eq!(
        app.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&first).unwrap()
    );
    assert_eq!(app.playback.last, crate::play::playback::Mode::AllTracks);
    assert!(app.playback.active.is_some());
    let persisted = History::open(root.join("recent-files.json"));
    assert_eq!(
        persisted.entries[0].path,
        std::fs::canonicalize(&first).unwrap()
    );
    assert_eq!(persisted.entries.len(), 2);
    assert_eq!(
        std::fs::read(&second).unwrap(),
        smf_io::encode(&source).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn paging_clamps_and_overlay_does_not_open_over_other_editors() {
    let mut app = crate::view::tests::app(music());
    app.recent.history.entries = (0..70)
        .map(|i| Entry {
            path: format!("X:/music/{i}.mid").into(),
            opened: now(),
            title: "Title".into(),
            seconds: 90.0,
            tracks: 3,
        })
        .collect();
    let ctx = egui::Context::default();
    app.help_open = true;
    press(&mut app, &ctx, Key::P, Modifiers::CTRL);
    assert!(!app.recent.open);
    app.help_open = false;
    press(&mut app, &ctx, Key::P, Modifiers::CTRL);
    press(&mut app, &ctx, Key::PageDown, Modifiers::NONE);
    assert_eq!(app.recent.cursor, 10);
    assert!(app.recent.error.is_some());
    assert!(app.load_error.is_none());
    press(&mut app, &ctx, Key::PageUp, Modifiers::NONE);
    assert_eq!(app.recent.cursor, 0);
    press(&mut app, &ctx, Key::ArrowUp, Modifiers::NONE);
    assert_eq!(app.recent.cursor, 0);
    app.recent.cursor = 65;
    press(&mut app, &ctx, Key::PageDown, Modifiers::NONE);
    assert_eq!(app.recent.cursor, 69);
    press(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    assert!(!app.recent.open);
}

#[test]
fn fitting_list_keeps_all_columns_visible_and_overflow_stays_inside_screen() {
    for (count, path_len) in [(3, 10), (80, 10), (3, 400)] {
        let mut app = crate::view::tests::app(music());
        app.recent.open = true;
        app.recent.history.entries = (0..count)
            .map(|i| Entry {
                path: format!("X:/music/{}/file-{i}.mid", "x".repeat(path_len)).into(),
                opened: now(),
                title: "Title".into(),
                seconds: 100.0,
                tracks: 4,
            })
            .collect();
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            ..Default::default()
        };
        for _ in 0..3 {
            let _ = ctx.run(input(), |ctx| {
                show(&mut app, ctx);
            });
        }
        let out = ctx.run(input(), |ctx| {
            show(&mut app, ctx);
        });
        let text: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some((s.clip_rect, t)),
                _ => None,
            })
            .collect();
        let list: Vec<_> = text
            .iter()
            .filter(|(_, t)| t.galley.text().starts_with("X:/music/"))
            .collect();
        assert!(!list.is_empty());
        for (clip, _) in &list {
            assert!(clip.left() >= 0.0 && clip.top() >= 0.0);
            assert!(clip.right() <= 1500.0 && clip.bottom() <= 900.0, "{clip:?}");
        }
        if count == 3 && path_len == 10 {
            assert_eq!(list.len(), count);
            for (clip, t) in list {
                let bounds = egui::Rect::from_min_size(t.pos, t.galley.size());
                assert!(
                    clip.contains_rect(bounds),
                    "text {bounds:?} outside {clip:?}"
                );
            }
        }
    }
}
