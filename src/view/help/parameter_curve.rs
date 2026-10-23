use eframe::egui::{self, Color32};
pub(super) fn rows(ui: &mut egui::Ui) {
    super::key_groups(
        ui,
        &[(
            Color32::from_rgb(0xe6, 0xdb, 0x74),
            &[
                ("h l", "Point選択"),
                ("j k", "Control選択"),
                ("a / Delete", "Point追加 / 削除"),
                ("Enter", "Point / Control編集"),
                ("↑ ↓", "編集中の値 / Variation"),
                ("円 / 四角: ドラッグ", "Point / Control"),
                ("円: 右クリック", "Point削除"),
                ("Ctrl+Z", "元に戻す"),
                ("PageUp / PageDown", "時間拡大 / 縮小"),
                ("minimap: ドラッグ", "表示範囲"),
                ("A-B repeat: 左ドラッグ", "範囲指定"),
                ("A-B repeat: 右ドラッグ", "端の移動"),
                ("音符表示: 左クリック", "演奏位置"),
                ("p / Space", "再生menu / 前回の再生"),
                ("Shift+Space", "演奏停止"),
                ("F", "再生位置追従"),
                ("Ctrl+→ ←", "次 / 前のtrack"),
                ("r / Esc", "閉じる"),
            ],
        )],
    );
}
