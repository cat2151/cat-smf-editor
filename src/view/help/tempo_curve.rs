use eframe::egui::{self, Color32, RichText};

fn group(ui: &mut egui::Ui, color: Color32, rows: &[(&str, &str)]) {
    for (key, action) in rows {
        ui.label(RichText::new(*key).monospace().color(color));
        ui.label(*action);
        ui.end_row();
    }
    ui.end_row();
}

pub(super) fn rows(ui: &mut egui::Ui) {
    group(
        ui,
        Color32::from_rgb(0xe6, 0xdb, 0x74),
        &[
            ("Ctrl+Z", "元に戻す"),
            ("空白: 左クリック / ドラッグ", "テンポを追加・移動"),
            ("円 / 四角: ドラッグ", "テンポ / カーブを調整"),
            ("円: 右クリック", "テンポを削除"),
            ("破線: ドラッグ", "目安BPMを調整"),
        ],
    );
    group(
        ui,
        Color32::from_rgb(0xf9, 0x26, 0x72),
        &[("Trill text: クリック", "長さ・間隔を編集")],
    );
    group(
        ui,
        Color32::from_rgb(0xa6, 0xe2, 0x2e),
        &[
            ("音符表示: 左クリック", "試聴 / 演奏位置を変更"),
            ("p / Space", "再生menu / 前回の再生"),
            ("Shift+Space", "演奏停止"),
            ("F", "再生位置に追従"),
        ],
    );
    group(
        ui,
        Color32::from_rgb(0xae, 0x81, 0xff),
        &[
            ("A-B repeat: 左ドラッグ", "範囲指定"),
            ("A-B repeat: 右クリック / ドラッグ", "端を移動"),
            ("A-B repeat: 左クリック", "解除"),
        ],
    );
    group(
        ui,
        Color32::from_rgb(0x66, 0xd9, 0xef),
        &[
            ("音符表示: 左ドラッグ", "時間範囲を拡大"),
            ("音符表示: 右クリック / ドラッグ", "時間範囲の端を調整"),
            ("minimap: 左クリック / ドラッグ", "移動 / 表示枠を移動"),
            ("上zoom: 左 / 右クリック", "時間を拡大 / 縮小"),
            ("左zoom: 左 / 右クリック", "BPMを拡大 / 縮小"),
            ("zoomバー: 左ドラッグ", "範囲を拡大"),
            ("ホイール", "時間を移動"),
        ],
    );
    group(
        ui,
        Color32::from_gray(190),
        &[("r / Esc", "閉じる"), (",m", "最大化を切替")],
    );
}
