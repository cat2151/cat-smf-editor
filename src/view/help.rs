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
            ("Shift+R", "Virtual MIDI keyboard"),
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
                &[("Ctrl+Shift+P", "コマンドパレット")],
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

pub(crate) use context::{input, observe, observe_recording, State};
pub(crate) use window::{lift, restore, show, summon, viewport};
mod context;
mod overlays;
mod pages;
mod placement;
mod song;
mod window;

pub(super) fn body(app: &mut App, ui: &mut egui::Ui) {
    use context::Target;
    match app.help.target {
        Target::Mml => overlays::mml(ui),
        Target::Palette => overlays::palette(ui),
        Target::Recording => overlays::recording(ui),
        Target::Mixer => overlays::mixer(ui),
        Target::Singer => overlays::singer(ui),
        Target::Recent => overlays::recent(ui),
        Target::Selection => overlays::selection(ui),
        Target::SelectionMenu => {
            ui.heading("範囲menu");
            egui::Grid::new("selection-menu-help").show(ui, |ui| {
                key_groups(
                    ui,
                    &[(Color32::from_rgb(0xe6, 0xdb, 0x74), &[("Esc", "戻る")])],
                );
            });
        }
        Target::Layers => {
            if crate::view::layers::keyboard::help(ui) {
                app.help_open = false;
            }
        }
        Target::Effects => {
            if crate::view::effects::keyboard::help(ui) {
                app.help_open = false;
            }
        }
        Target::Drums => {
            if let Some(l) = &app.loaded {
                ui.heading(format!("Drum sequencer · {}", l.track_titles[app.track]));
            }
            pages::drums(ui);
        }
        Target::Macro => pages::macros(ui),
        Target::Picker => {
            if let Some(title) = app.parameter_editor.help_title() {
                ui.heading(format!("{title} / Automation"));
            }
            pages::picker(ui);
        }
        Target::Scene => pages::arrangement(ui, crate::view::patterns::workspace::Pane::Scene),
        Target::Song => pages::arrangement(ui, crate::view::patterns::workspace::Pane::Song),
        Target::Views => {
            ui.heading("Views");
            egui::Grid::new("view-palette-help").show(ui, view_rows);
        }
        target => {
            ui.heading(match target {
                Target::Piano => "ピアノロール",
                Target::Tempo => "Tempo curve",
                Target::Parameter => "Parameter curve",
                Target::Overview => "Track Overview",
                _ => "Event list",
            });
            egui::Grid::new("help-keys").show(ui, |ui| match target {
                Target::Piano => piano_roll::rows(ui),
                Target::Tempo => tempo_curve::rows(ui),
                Target::Parameter => parameter_curve::rows(ui),
                Target::Overview => track_overview::rows(ui),
                _ => key_groups(ui, &KEYS),
            });
            close_guide(ui);
        }
    }
}

#[cfg(test)]
mod tests;

mod piano_roll;
mod tempo_curve;
mod track_overview;

mod parameter_curve;

#[cfg(test)]
mod context_tests;

#[cfg(test)]
pub(crate) fn test_target(a: &App) -> String {
    format!("{:?}", a.help.target)
}
#[cfg(test)]
mod native_tests;

#[cfg(test)]
mod overlay_tests;
