use super::context::Target;
use eframe::egui::{self, Key, Modifiers};

fn input(key: Key, modifiers: Modifiers) -> egui::RawInput {
    egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
        ..Default::default()
    }
}

#[test]
fn palette_over_mml_owns_help_and_escape_restores_mml_help() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let c = egui::Context::default();
    a.piano_roll.open = true;
    a.mml_input.open = true;
    let frame = |a: &mut crate::app::App, raw| {
        let _ = c.run(raw, |c| {
            let saved = crate::view::modal::begin(a, c);
            crate::view::modal::end(a, c, saved);
            super::observe(a, c);
        });
    };
    frame(&mut a, Default::default());
    assert_eq!(a.help.target, Target::Mml);
    frame(&mut a, input(Key::P, Modifiers::CTRL | Modifiers::SHIFT));
    assert_eq!(a.help.target, Target::Palette);
    frame(&mut a, input(Key::F1, Modifiers::NONE));
    assert!(a.help_open);
    assert!(a.palette.is_open());
    assert!(a.mml_input.open);
    assert_eq!(a.help.target, Target::Palette);
    frame(&mut a, input(Key::Escape, Modifiers::NONE));
    assert!(!a.palette.is_open());
    assert!(a.help_open);
    assert!(a.mml_input.open);
    assert_eq!(a.help.target, Target::Mml);
    a.mml_input.open = false;
    // Help retaining focus must still discard a closed overlay's content.
    frame(
        &mut a,
        egui::RawInput {
            focused: false,
            ..Default::default()
        },
    );
    assert_eq!(a.help.target, Target::Piano);
}

#[test]
fn help_headings_and_arrangement_colors_identify_the_active_editor() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let c = egui::Context::default();
    for (target, heading) in [
        (Target::Main, "Event list"),
        (Target::Piano, "ピアノロール"),
        (Target::Mml, "MML / Chord progression"),
        (Target::Palette, "コマンドパレット"),
        (Target::Mixer, "Mixer"),
        (Target::Singer, "VOICEVOX voice"),
        (Target::Recent, "最近開いたファイル"),
        (Target::Selection, "範囲選択"),
        (Target::Song, "Pattern / Song"),
        (Target::Scene, "Scene"),
    ] {
        a.help.target = target;
        let output = c.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1500.0, 2000.0),
                )),
                ..Default::default()
            },
            |c| {
                egui::CentralPanel::default().show(c, |ui| super::body(&mut a, ui));
            },
        );
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0].galley.text(), heading);
        if matches!(target, Target::Song | Target::Scene) {
            let keys = if target == Target::Song {
                for section in ["候補", "セル", "共通"] {
                    assert!(texts.iter().any(|t| t.galley.text() == section));
                }
                assert!(!texts.iter().any(|t| t.galley.text() == "PageUp / PageDown"));
                ["j k", "h j k l", "Space", "N"]
            } else {
                ["Enter", "← → / ↑ ↓", "Space", "Esc"]
            };
            let colors: std::collections::BTreeSet<_> = keys
                .into_iter()
                .map(|key| {
                    texts
                        .iter()
                        .find(|t| t.galley.text() == key)
                        .unwrap()
                        .galley
                        .job
                        .sections[0]
                        .format
                        .color
                        .to_array()
                })
                .collect();
            assert_eq!(colors.len(), 4);
            assert_eq!(
                texts.iter().any(|t| t.galley.text() == "S"),
                target == Target::Song
            );
        }
        if target == Target::Main {
            assert!(!texts.iter().any(|t| t.galley.text() == "候補先頭 / 末尾"));
        }
        if target == Target::Mml {
            // Confirm and cancel already carry their keys on the overlay buttons.
            assert!(!texts
                .iter()
                .any(|t| matches!(t.galley.text(), "Enter" | "Esc")));
        }
    }
}
