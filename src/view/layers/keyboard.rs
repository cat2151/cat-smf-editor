use eframe::egui;

pub(crate) fn help(ui: &mut egui::Ui) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.heading("レイヤー — ヘルプ");
        if ui.button("×").clicked() {
            close = true;
        }
    });
    egui::Grid::new("layer-help").show(ui, |ui| {
        crate::view::help::key_groups(
            ui,
            &[
                (
                    egui::Color32::from_rgb(0x66, 0xd9, 0xef),
                    &[("j k / h l", "レイヤー / 項目")],
                ),
                (
                    egui::Color32::from_rgb(0xf9, 0x26, 0x72),
                    &[
                        ("Enter", "音色選択 / 数値入力"),
                        ("o / i", "Octave +1 / −1"),
                        ("] / [", "音量 +6 / −6 dB"),
                        ("m / s", "mute / solo"),
                        ("a / Delete", "複製 / 削除"),
                        ("f", "Spectrum / Lissajous"),
                        ("u", "Automation"),
                        ("Ctrl+Z", "元に戻す"),
                    ],
                ),
            ],
        );
    });
    crate::view::help::close_guide(ui);
    close
}
