use crate::view::patterns::workspace::Pane;
use eframe::egui::{self, Color32};

pub(super) fn drums(ui: &mut egui::Ui) {
    egui::Grid::new("drum-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[(
                Color32::LIGHT_BLUE,
                &[
                    ("h j k l", "移動"),
                    ("Ctrl+→ ←", "次 / 前のトラック"),
                    ("V", "Notes / Raw"),
                    ("f / Space", "Step"),
                    ("Enter", "音色 / Note / Step"),
                    ("PageUp / PageDown", "小節"),
                    ("[ / ]", "A小節"),
                    ("Shift+[ / ]", "B小節"),
                    ("Shift+Space", "停止"),
                    ("Ctrl+Z", "元に戻す"),
                    ("Esc", "閉じる"),
                ],
            )],
        );
    });
}

pub(super) fn picker(ui: &mut egui::Ui) {
    egui::Grid::new("parameter-picker-help").show(ui, |ui| {
        crate::view::help::key_groups(
            ui,
            &[(
                egui::Color32::LIGHT_BLUE,
                &[
                    ("↑ ↓", "Parameter選択・Preview"),
                    ("PageUp / PageDown", "10候補移動"),
                    ("Ctrl+Home / Ctrl+End", "候補先頭 / 末尾"),
                    ("Shift+Space", "演奏停止"),
                    ("Esc", "閉じる"),
                ],
            )],
        );
    });
}

pub(super) fn arrangement(ui: &mut egui::Ui, p: Pane) {
    if p == Pane::Song {
        super::song::rows(ui);
        return;
    }
    ui.heading(p.title());
    let mut editing = vec![
        ("Enter", "Put"),
        ("E", "本体編集"),
        ("Shift+E", "部品編集"),
        ("N", "新規pattern"),
        ("Shift+R", "Virtual MIDI keyboard"),
        ("Delete", "配置解除"),
    ];
    if p == Pane::Song {
        editing.push(("Shift+Delete", "候補pattern削除"));
    }
    editing.extend([("Ctrl+Z", "Undo"), ("Ctrl+/", "コメントアウト")]);
    egui::Grid::new((p.title(), "help")).show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (Color32::from_rgb(0xe6, 0xdb, 0x74), &editing),
                (
                    Color32::from_rgb(0x66, 0xd9, 0xef),
                    &[
                        ("← → / ↑ ↓", "小節 / レーン"),
                        ("PageUp / PageDown", "部品選択"),
                        ("Ctrl+← → ↑ ↓", "配置移動"),
                    ],
                ),
                (
                    Color32::from_rgb(0xa6, 0xe2, 0x2e),
                    &[
                        ("Space", "再生"),
                        ("Shift+Space", "停止"),
                        ("P", "再生menu"),
                    ],
                ),
            ],
        );
        ui.end_row();
        let windows: &[(&str, &str)] = if p == Pane::Song {
            &[("S", "Scene window"), ("Esc", "終了")]
        } else {
            &[("Esc", "終了")]
        };
        super::key_groups(ui, &[(Color32::from_rgb(0xae, 0x81, 0xff), windows)]);
    });
}

pub(super) fn macros(ui: &mut egui::Ui) {
    ui.heading("Macro edit — ヘルプ");
    egui::Grid::new("macro-help").show(ui, |ui| {
        crate::view::help::key_groups(
            ui,
            &[(
                egui::Color32::from_rgb(0x66, 0xd9, 0xef),
                &[
                    ("j k", "Macro選択"),
                    ("h l", "値"),
                    ("1–9… + h j k l", "反復回数"),
                    ("[ / ]", "変化量 ÷2 / ×2"),
                    ("Home / End", "最小 / 最大"),
                    ("PageUp / PageDown", "Variation切替"),
                    ("r", "全Macroランダム・Variation範囲"),
                    ("Shift+R", "全Macroランダム・全範囲"),
                ],
            )],
        );
    });
    crate::view::help::close_guide(ui);
}
