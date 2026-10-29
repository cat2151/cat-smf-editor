use eframe::egui::{self, Color32};

pub(super) fn rows(ui: &mut egui::Ui) {
    ui.heading("Pattern / Song");
    ui.heading("候補");
    egui::Grid::new("song-candidate-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[(
                Color32::from_rgb(0x66, 0xd9, 0xef),
                &[
                    ("j k", "候補選択"),
                    ("f", "配置先"),
                    ("E", "選択候補の編集"),
                    ("Shift+Delete", "候補pattern削除"),
                    ("Esc", "終了"),
                ],
            )],
        );
    });
    ui.heading("セル");
    egui::Grid::new("song-cell-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[(
                Color32::from_rgb(0xe6, 0xdb, 0x74),
                &[
                    ("h j k l", "小節・レーン"),
                    ("f", "配置"),
                    ("Esc", "候補"),
                    ("E", "セル内容の編集"),
                    ("Ctrl+← → ↑ ↓", "配置移動"),
                    ("Delete", "配置解除"),
                    ("Ctrl+/", "コメントアウト"),
                ],
            )],
        );
    });
    ui.heading("Timeline");
    egui::Grid::new("song-timeline-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[(
                Color32::from_rgb(0xae, 0x81, 0xff),
                &[
                    ("zoom: 左 / 右クリック", "時間の拡大 / 縮小"),
                    ("zoom: 左ドラッグ", "範囲拡大"),
                    ("zoom: ホイール", "時間の拡大 / 縮小"),
                    ("A-B repeat: 左ドラッグ", "範囲指定"),
                    ("A-B repeat: 右クリック / ドラッグ", "端の移動"),
                    ("A-B repeat: 左クリック", "解除"),
                ],
            )],
        );
    });
    ui.heading("共通");
    egui::Grid::new("song-common-help").show(ui, |ui| {
        super::key_groups(
            ui,
            &[
                (
                    Color32::from_rgb(0xf9, 0x26, 0x72),
                    &[
                        ("N", "新規pattern"),
                        ("Ctrl+Z", "Undo"),
                        ("Shift+R", "Virtual MIDI keyboard"),
                    ],
                ),
                (
                    Color32::from_rgb(0xa6, 0xe2, 0x2e),
                    &[
                        ("Space", "再生"),
                        ("Shift+Space", "停止"),
                        ("P", "再生menu"),
                        ("S", "Scene window"),
                        ("PageUp / PageDown", "時間の拡大 / 縮小"),
                    ],
                ),
            ],
        );
    });
}
