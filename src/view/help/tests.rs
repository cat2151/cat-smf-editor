use crate::view::tests::app;
use cat_smf_editor::model::{Music, Track};
use eframe::egui;

#[test]
fn help_shortcuts_and_warnings_tooltip_preserve_view_state() {
    let mut app = app(Music {
        format: 2,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    });
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    let texts = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::epaint::Shape::Text(t) = &s.shape {
                    Some((t.galley.text().to_owned(), t.pos))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
    render(&mut app, vec![]);
    let closed = render(&mut app, vec![]);
    let closed = texts(&closed);
    assert!(!closed.iter().any(|(t, _)| t == "エフェクト"));
    let warnings = closed.iter().find(|(t, _)| t == "Warnings 1").unwrap().1;
    let hover = vec![egui::Event::PointerMoved(warnings + egui::vec2(4.0, 4.0))];
    render(&mut app, hover.clone());
    let hovered = render(&mut app, hover);
    assert!(texts(&hovered)
        .iter()
        .any(|(t, _)| t.contains("4/4 とみなした")));
    assert!(!app.help_open, "warnings are read without opening help");
    render(
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::Slash,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::Text("?".into()),
        ],
    );
    assert!(app.help_open);
    let opened = render(&mut app, vec![]);
    let labels = texts(&opened);
    assert!(
        labels.iter().any(|(t, _)| t == "エフェクト"),
        "the last key is visible without scrolling"
    );
    assert!(app.note_mode);
    render(
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::Slash,
                physical_key: None,
                pressed: true,
                repeat: true,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::Text("?".into()),
        ],
    );
    assert!(app.help_open);
    render(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!app.help_open);
    app.loaded = None;
    render(&mut app, vec![egui::Event::Text("?".into())]);
    assert!(app.help_open, "help is available before loading a file");
    render(&mut app, vec![egui::Event::Text("?".into())]);
    assert!(!app.help_open);
}

#[test]
fn tempo_help_groups_operations_with_distinct_colors_and_clear_area_names() {
    let ctx = egui::Context::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 2200.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::Grid::new("grouped-tempo-help-test").show(ui, super::tempo_curve::rows);
            });
        },
    );
    let groups = [
        "空白: 左クリック / ドラッグ",
        "Trill text: クリック",
        "p / Space",
        "A-B repeat: 左ドラッグ",
        "minimap: 左クリック / ドラッグ",
        "Ctrl+, → m",
    ];
    let mut colors = std::collections::BTreeSet::new();
    for label in groups {
        let text = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing help group {label}"));
        colors.insert(text.galley.job.sections[0].format.color.to_array());
    }
    assert_eq!(colors.len(), groups.len());
    for heading in [
        "テンポ編集（下半分）",
        "Trill",
        "演奏と開始位置",
        "A–B repeat",
        "表示範囲と倍率",
        "画面",
    ] {
        assert!(!output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text()==heading)));
    }
    assert!(
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .all(|text| text.chars().count() <= 32),
        "help rows should stay concise"
    );
    assert!(!output.shapes.iter().any(
        |s| matches!(&s.shape,egui::epaint::Shape::Text(t) if t.galley.text().starts_with("ロール"))
    ));
}
