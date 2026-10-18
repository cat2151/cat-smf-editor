use eframe::egui::{self, Color32};

pub(super) fn rows(ui: &mut egui::Ui) {
    super::key_groups(
        ui,
        &[
            (
                Color32::from_rgb(0xe6, 0xdb, 0x74),
                &[
                    ("Ctrl+Z", "元に戻す"),
                    ("i", "MML・コード進行"),
                    ("ノート左端 / 中央 / 右端: ドラッグ", "開始 / 位置 / 長さ"),
                ],
            ),
            (
                Color32::from_rgb(0xa6, 0xe2, 0x2e),
                &[
                    ("音符表示: 左クリック", "試聴 / 演奏位置"),
                    ("p / Space", "再生menu / 前回の再生"),
                    ("Shift+Space", "演奏停止"),
                    ("F", "再生位置への追従"),
                ],
            ),
            (
                Color32::from_rgb(0xae, 0x81, 0xff),
                &[
                    ("A-B repeat: 左ドラッグ", "範囲指定"),
                    ("A-B repeat: 右クリック / ドラッグ", "端の移動"),
                    ("A-B repeat: 左クリック", "解除"),
                ],
            ),
            (
                Color32::from_rgb(0x66, 0xd9, 0xef),
                &[
                    ("空白: 左ドラッグ", "時間範囲の拡大"),
                    ("音符表示: 右クリック / ドラッグ", "時間範囲の端"),
                    ("minimap: 左クリック / ドラッグ", "位置 / 表示枠"),
                    ("上zoom: 左 / 右クリック", "時間の拡大 / 縮小"),
                    ("左zoom: 左 / 右クリック", "音高の拡大 / 縮小"),
                    ("zoomバー: 左ドラッグ", "範囲拡大"),
                    ("ホイール", "時間移動"),
                ],
            ),
            (Color32::from_gray(190), &[(",r", "最大化")]),
        ],
    );
}
