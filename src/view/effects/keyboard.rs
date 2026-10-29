use eframe::egui;

pub(crate) fn help(ui: &mut egui::Ui) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.heading("エフェクト — ヘルプ");
        if ui.button("×").clicked() {
            close = true;
        }
    });
    egui::Grid::new("effect-help").show(ui, |ui| {
        crate::view::help::key_groups(
            ui,
            &[
                (
                    egui::Color32::from_rgb(0x66, 0xd9, 0xef),
                    &[("j k", "エフェクト"), ("Shift+j k", "順番")],
                ),
                (
                    egui::Color32::from_rgb(0xf9, 0x26, 0x72),
                    &[
                        ("a / x", "追加 / 差し替え"),
                        ("b", "bypass"),
                        ("Delete", "削除"),
                        ("Enter / Esc", "閉じる"),
                        ("Ctrl+Z", "元に戻す"),
                    ],
                ),
            ],
        );
    });
    crate::view::help::close_guide(ui);
    close
}
