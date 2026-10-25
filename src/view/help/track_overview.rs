use eframe::egui::{self, Color32};

pub(super) fn rows(ui: &mut egui::Ui) {
    super::key_groups(
        ui,
        &[
            (
                Color32::from_rgb(0xe6, 0xdb, 0x74),
                &[("Ctrl+,", "設定menu"), ("Ctrl+Z", "元に戻す")],
            ),
            (
                Color32::from_rgb(0xa6, 0xe2, 0x2e),
                &[
                    ("左クリック / Enter", "全trackをその小節から演奏"),
                    ("右クリック", "track play / jump"),
                    ("t / g", "track play / jump"),
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
                    ("l h / j k", "小節 / track"),
                    ("minimap: 左クリック / ドラッグ", "位置 / 表示枠"),
                    ("上zoom: 左 / 右クリック", "時間の拡大 / 縮小"),
                    ("左zoom: 左 / 右クリック", "trackの高さの拡大 / 縮小"),
                    ("zoomバー: 左ドラッグ", "範囲拡大"),
                    ("ホイール", "時間移動"),
                ],
            ),
        ],
    );
}
