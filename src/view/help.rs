use crate::app::App;
use eframe::egui::{self, Color32, RichText};

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

/// 機能ごとのキー。色は row.rs / pitch_keyboard.rs と同じ Monokai から、ACTIVE_PANE の orange を避けて選ぶ。
/// dialog・履歴などその場にキーが表示される操作は載せない。
const KEYS: [(Color32, &[(&str, &str)]); 5] = [
    (
        Color32::from_rgb(0xe6, 0xdb, 0x74),
        &[
            ("Ctrl+,", "設定menu"),
            ("Ctrl+O / Ctrl+P", "開く / 最近開いたファイル"),
            ("Ctrl+N", "新規SMF"),
            ("Ctrl+Shift+N", "トラック追加"),
            ("Ctrl+S", "保存"),
            ("Ctrl+Z", "元に戻す"),
            ("V", "Notes / Raw"),
            ("i / e / .", "MML・ドラム / 編集menu / 反復"),
            ("r", "ピアノロール / カーブ / ドラム"),
            ("Shift+D", "Drum sequencer"),
            ("左ドラッグ", "A-B repeat"),
            ("s", "範囲選択 / 範囲menu"),
        ],
    ),
    (Color32::from_rgb(0xae, 0x81, 0xff), &[("w", "2列menu")]),
    (
        Color32::from_rgb(0x66, 0xd9, 0xef),
        &[
            ("g", "移動menu"),
            ("j k / PageDown PageUp", "行 / 10行"),
            ("l h / J K", "小節 / 拍"),
            ("Ctrl+→ ←", "次 / 前のトラック"),
            ("F2", "トラック名変更"),
            ("Home / End", "track先頭 / 末尾"),
        ],
    ),
    (
        Color32::from_rgb(0xa6, 0xe2, 0x2e),
        &[
            ("p / Space", "再生menu / 前回の再生"),
            ("Shift+Space", "演奏停止"),
            ("F", "再生に追従"),
        ],
    ),
    (
        Color32::from_rgb(0xf9, 0x26, 0x72),
        &[
            ("M", "ミキサー"),
            ("T / Shift+t", "音色 / Instrument layers"),
            ("X", "エフェクト"),
            ("Ctrl+, → f", "Spectrum / Lissajous"),
        ],
    ),
];

pub(super) fn key_groups(ui: &mut egui::Ui, groups: &[(Color32, &[(&str, &str)])]) {
    for (i, (color, keys)) in groups.iter().enumerate() {
        if i > 0 {
            ui.end_row();
        }
        for (key, word) in *keys {
            ui.label(RichText::new(*key).monospace().color(*color));
            ui.label(*word);
            ui.end_row();
        }
    }
}

pub(super) fn close_guide(ui: &mut egui::Ui) {
    ui.separator();
    egui::Grid::new("common-help-keys").show(ui, |ui| {
        key_groups(
            ui,
            &[(
                Color32::from_rgb(0xe6, 0xdb, 0x74),
                &[
                    ("Ctrl+Shift+P", "コマンドパレット"),
                    ("↑ ↓ / PageUp PageDown", "候補 / 10候補"),
                    ("Ctrl+Home / Ctrl+End", "候補先頭 / 末尾"),
                ],
            )],
        );
    });
    ui.weak("Esc: 閉じる");
}

pub(crate) fn view_rows(ui: &mut egui::Ui) {
    key_groups(
        ui,
        &[(
            Color32::from_rgb(0x66, 0xd9, 0xef),
            &[
                ("j k", "候補選択"),
                ("h l", "階層"),
                ("Enter", "選択・表示"),
                ("Esc", "戻る・取消"),
                ("F1", "ヘルプ"),
            ],
        )],
    );
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let mut open = app.help_open;
    let window = egui::Window::new("ヘルプ");
    let window = if app.tempo_curve.open {
        window.default_height((ctx.screen_rect().height() - 80.0).max(200.0))
    } else {
        window
    };
    window.open(&mut open).resizable(false).show(ctx, |ui| {
        if app.tempo_curve.open && !app.piano_roll.open && !app.track_overview.open {
            egui::ScrollArea::vertical()
                .max_height((ctx.screen_rect().height() - 160.0).max(200.0))
                .show(ui, |ui| {
                    egui::Grid::new("tempo-help").show(ui, tempo_curve::rows);
                });
        } else {
            egui::Grid::new("help-keys").show(ui, |ui| {
                if app.track_overview.open {
                    track_overview::rows(ui);
                } else if app.parameter_editor.open {
                    parameter_curve::rows(ui);
                } else if app.piano_roll.open {
                    piano_roll::rows(ui);
                } else {
                    key_groups(ui, &KEYS);
                }
            });
        }
        close_guide(ui);
    });
    app.help_open = open;
}

#[cfg(test)]
mod tests;

mod piano_roll;
mod tempo_curve;
mod track_overview;

mod parameter_curve;
