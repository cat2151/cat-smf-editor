use crate::{app::App, store::config::ProgramTarget as Target};
use eframe::egui;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// `,t` was pressed: the overlay lists the program change targets.
    pub(crate) program: bool,
}

pub fn open(app: &mut App) {
    app.comma_menu = State {
        open: true,
        program: false,
    };
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

pub(crate) fn swallow(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events
            .retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)))
    });
}

pub(crate) fn choose(app: &mut App, target: Target) {
    app.config.select_program_target(target);
    app.comma_menu.open = false;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.comma_menu.open {
        return;
    }
    let mut open = true;
    egui::Window::new("help")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if app.comma_menu.program {
                let current = app.config.program_target;
                for (target, label, description) in [
                    (
                        Target::Auto,
                        "a: auto",
                        "ノートのあるtrackが複数あり、すべて同じProgram change名なら、まとめて同じ音色を割り当てます。それ以外は現在のtrackだけに割り当てます。",
                    ),
                    (
                        Target::Current,
                        "c: current track",
                        "Program change名が同じでも、現在のtrackだけに音色を割り当てます。trackごとに個別の音色を選べます。",
                    ),
                ] {
                    if ui
                        .selectable_label(current == target, label)
                        .on_hover_text(description)
                        .clicked()
                    {
                        choose(app, target);
                    }
                }
            } else {
                let mut auto_save = app
                    .loaded
                    .as_ref()
                    .is_some_and(|loaded| crate::store::auto_save::enabled(&loaded.music));
                if ui
                    .add_enabled(
                        app.loaded.is_some(),
                        egui::Checkbox::new(&mut auto_save, "s: このファイルのAuto save"),
                    )
                    .on_hover_text("このファイルの編集内容を自動保存します。有効にするときは現在の編集内容を保存します。")
                    .changed()
                {
                    app.toggle_auto_save();
                }
                ui.separator();
                let mut maximized = app.config.tempo_curve_maximized;
                if ui
                    .checkbox(&mut maximized, "m: Tempo curve always maximized")
                    .on_hover_text("Tempo curve画面を最大化表示します。")
                    .changed()
                {
                    app.config.select_tempo_curve_maximized(maximized);
                }
                let mut maximized = app.config.piano_roll_maximized;
                if ui
                    .checkbox(&mut maximized, "r: Piano roll always maximized")
                    .on_hover_text("Piano roll画面を最大化表示します。")
                    .changed()
                {
                    app.config.select_piano_roll_maximized(maximized);
                }
                let mut visible = app.config.meta_text_visible;
                if ui
                    .checkbox(&mut visible, "a: All tracks meta text")
                    .on_hover_text("meta eventのtextを、他のtrackにいるときもTempo curve・Piano roll画面に表示します。")
                    .changed()
                {
                    app.config.select_meta_text_visible(visible);
                }
                if ui
                    .button("t: program change target")
                    .on_hover_text("Program changeにInstrumentオーディオプラグインの音色を割り当てる対象を選びます。autoは、ノートのある複数のtrackがすべて同じProgram change名ならまとめて同じ音色を割り当て、それ以外は現在のtrackだけに割り当てます。current trackは常に現在のtrackだけに割り当てます。")
                    .clicked()
                {
                    app.comma_menu.program = true;
                }
                ui.separator();
                let mut analysis = app.analysis.is_open();
                if ui
                    .checkbox(&mut analysis, "f: Spectrum / Lissajous")
                    .on_hover_text("出力音声の周波数成分と、左右チャンネルの関係を別ウィンドウに表示します。")
                    .changed()
                {
                    crate::view::analysis::toggle(app);
                }
                if ui
                    .button("p: Reset spectrum window position")
                    .on_hover_text("Spectrum / Lissajousウィンドウの保存された位置とサイズをリセットします。画面外に移動して見えなくなったときにも使えます。")
                    .clicked()
                {
                    crate::view::analysis::reset_window(app);
                }
            }
        });
    if !open {
        app.comma_menu.open = false;
    }
}

#[cfg(test)]
mod tests;
