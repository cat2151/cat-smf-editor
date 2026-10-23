//! Navigation shared by searchable selection lists.
use eframe::egui::{self, Key, Modifiers};

#[derive(Clone, Copy)]
pub(super) enum Move {
    Step(isize),
    First,
    Last,
}
impl Move {
    pub(super) fn apply(self, selected: usize, count: usize) -> usize {
        let last = count.saturating_sub(1);
        match self {
            Self::Step(delta) => selected.saturating_add_signed(delta).min(last),
            Self::First => 0,
            Self::Last => last,
        }
    }
}
pub(super) fn bindings() -> [(Key, Modifiers, Move, &'static str); 6] {
    [
        (Key::ArrowUp, Modifiers::NONE, Move::Step(-1), "前の候補"),
        (Key::ArrowDown, Modifiers::NONE, Move::Step(1), "次の候補"),
        (Key::PageUp, Modifiers::NONE, Move::Step(-10), "10候補上"),
        (Key::PageDown, Modifiers::NONE, Move::Step(10), "10候補下"),
        (Key::Home, Modifiers::CTRL, Move::First, "候補先頭"),
        (Key::End, Modifiers::CTRL, Move::Last, "候補末尾"),
    ]
}
pub(super) fn movement(key: Key, modifiers: Modifiers) -> Option<Move> {
    bindings().into_iter().find_map(|(k, m, action, _)| {
        (key == k && super::commands::matches(modifiers, m)).then_some(action)
    })
}
pub(super) fn position(ui: &mut egui::Ui, selected: usize, count: usize) {
    let position = if count == 0 {
        0
    } else {
        selected.min(count - 1) + 1
    };
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{position} / {count}"))
                    .small()
                    .weak(),
            );
        });
    });
}
