use super::{Action, State};
use eframe::egui;

pub(super) fn handle(state: &mut State, ctx: &egui::Context, count: usize) -> Vec<Action> {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1))
        || ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Text(t) if t == "?"))
        })
    {
        state.help = !state.help;
        return Vec::new();
    }
    let keys: Vec<_> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if *modifiers == egui::Modifiers::NONE
                    || *modifiers == egui::Modifiers::SHIFT =>
                {
                    Some((*key, modifiers.shift))
                }
                _ => None,
            })
            .collect()
    });
    let mut actions = Vec::new();
    for (key, shift) in keys {
        if state.help {
            if key == egui::Key::Escape {
                state.help = false;
            }
            continue;
        }
        let selected = state.selected;
        let down = matches!(key, egui::Key::J | egui::Key::ArrowDown);
        let up = matches!(key, egui::Key::K | egui::Key::ArrowUp);
        match key {
            _ if shift && down => actions.push(Action::Move {
                from: selected,
                to: selected + 1,
            }),
            _ if shift && up && selected > 0 => actions.push(Action::Move {
                from: selected,
                to: selected - 1,
            }),
            _ if shift => {}
            egui::Key::Escape | egui::Key::Enter => {
                actions.push(Action::Close);
                break;
            }
            _ if down => {
                state.selected = (selected + 1).min(count.saturating_sub(1));
                state.reveal = true;
            }
            _ if up => {
                state.selected = selected.saturating_sub(1);
                state.reveal = true;
            }
            egui::Key::A => actions.push(Action::Browse(count)),
            egui::Key::X => actions.push(Action::Browse(if count == 0 { 0 } else { selected })),
            egui::Key::B if count > 0 => actions.push(Action::Bypass(selected)),
            egui::Key::Delete if count > 0 => actions.push(Action::Remove(selected)),
            _ => {}
        }
    }
    actions
}

pub(super) fn help(state: &mut State, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("エフェクト — ヘルプ");
        if ui.button("×").clicked() {
            state.help = false;
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
}
