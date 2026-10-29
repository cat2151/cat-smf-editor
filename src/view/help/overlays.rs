use eframe::egui::{self, Color32};

const EDIT: Color32 = Color32::from_rgb(0xe6, 0xdb, 0x74);
const MOVE: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);
const PLAY: Color32 = Color32::from_rgb(0xa6, 0xe2, 0x2e);
const SOUND: Color32 = Color32::from_rgb(0xf9, 0x26, 0x72);

pub(super) fn recording(ui: &mut egui::Ui, choosing: bool) {
    ui.heading("Virtual MIDI keyboard");
    egui::Grid::new("recording-help").show(ui, |ui| {
        if choosing {
            super::key_groups(
                ui,
                &[
                    (MOVE, &[("↑ ↓", "Container選択")]),
                    (EDIT, &[("Esc", "取消")]),
                ],
            );
            return;
        }
        super::key_groups(
            ui,
            &[
                (PLAY, &[("c d e f g a b", "C D E F G A B")]),
                (
                    EDIT,
                    &[("Esc", "録音終了・pattern確定"), ("Shift+Esc", "録音破棄")],
                ),
            ],
        );
    });
}

pub(super) fn mml(ui: &mut egui::Ui) {
    ui.heading("MML / Chord progression");
    egui::Grid::new("mml-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (
                    EDIT,
                    &[
                        ("Ctrl+A", "全選択"),
                        ("Ctrl+X / Ctrl+C / Ctrl+V", "切り取り / コピー / 貼り付け"),
                        ("Ctrl+Z", "入力を元に戻す"),
                        ("Backspace / Delete", "前 / 後の文字削除"),
                    ],
                ),
                (
                    MOVE,
                    &[
                        ("← → / ↑ ↓", "文字 / 行移動・試聴"),
                        ("Home / End", "行頭 / 行末"),
                        ("Shift+← → ↑ ↓", "文字選択"),
                    ],
                ),
                (PLAY, &[("Shift+Space", "演奏停止")]),
            ],
        );
    });
}

fn candidates(ui: &mut egui::Ui) {
    super::key_groups(
        ui,
        &[(
            MOVE,
            &[
                ("↑ ↓", "候補選択"),
                ("PageUp / PageDown", "10候補移動"),
                ("Ctrl+Home / Ctrl+End", "候補先頭 / 末尾"),
            ],
        )],
    );
}

pub(super) fn palette(ui: &mut egui::Ui) {
    ui.heading("コマンドパレット");
    egui::Grid::new("command-palette-help").show(ui, |ui| {
        candidates(ui);
        ui.end_row();
        super::key_groups(ui, &[(EDIT, &[("Enter", "実行"), ("Esc", "取消")])]);
    });
}

pub(super) fn singer(ui: &mut egui::Ui) {
    ui.heading("VOICEVOX voice");
    egui::Grid::new("singer-help").show(ui, |ui| {
        candidates(ui);
        ui.end_row();
        super::key_groups(ui, &[(EDIT, &[("Enter", "確定")])]);
    });
}

pub(super) fn mixer(ui: &mut egui::Ui) {
    ui.heading("Mixer");
    egui::Grid::new("mixer-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (MOVE, &[("h l", "Track / Master選択")]),
                (
                    SOUND,
                    &[
                        ("j k", "Track音量 −3 / +3 dB"),
                        ("Enter", "エフェクト"),
                        ("f", "Spectrum / Lissajous"),
                    ],
                ),
                (PLAY, &[("Shift+Space", "演奏停止")]),
                (
                    EDIT,
                    &[
                        ("Ctrl+Z", "元に戻す"),
                        ("m", "閉じる"),
                        ("Esc", "Event list"),
                    ],
                ),
            ],
        );
    });
}

pub(super) fn recent(ui: &mut egui::Ui) {
    ui.heading("最近開いたファイル");
    egui::Grid::new("recent-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (
                    MOVE,
                    &[
                        ("j k", "ファイル選択・試聴"),
                        ("PageUp / PageDown", "10ファイル移動"),
                    ],
                ),
                (EDIT, &[("Enter", "開く"), ("Esc", "取消")]),
                (PLAY, &[("Shift+Space", "プレビュー停止")]),
            ],
        );
    });
}

pub(super) fn selection(ui: &mut egui::Ui) {
    ui.heading("範囲選択");
    egui::Grid::new("selection-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (
                    MOVE,
                    &[
                        ("j k / PageDown PageUp", "行 / 10行"),
                        ("l h / J K", "小節 / 拍"),
                        ("Home / End", "Track先頭 / 末尾"),
                    ],
                ),
                (EDIT, &[("s", "範囲menu"), ("Esc", "戻る・取消")]),
                (PLAY, &[("Shift+Space", "演奏停止")]),
            ],
        );
    });
}
