//! Menus of the keyboard window's top row: the container, velocity and modulation.
use super::window::Button;
use super::*;
use cat_smf_editor::score::pattern::recording::{Modulation, Velocity};

/// A click on `text` lists `all` with `current` marked; returns another one clicked.
fn choose<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    text: egui::RichText,
    all: &[T],
    current: T,
    label: impl Fn(T) -> String,
    tip: &str,
) -> Option<T> {
    let mut chosen = None;
    ui.menu_button(text, |ui| {
        for &v in all {
            if ui.selectable_label(v == current, label(v)).clicked() {
                chosen = Some(v);
                ui.close_menu();
            }
        }
    })
    .response
    .on_hover_text(tip);
    chosen.filter(|&v| v != current)
}

/// The playing container; until a take is armed, a click lists the others and a new one.
pub(super) fn container(ui: &mut egui::Ui, s: &mut Session) {
    let name = &s.sounds[s.selected].name;
    if play::armed(s) {
        ui.label(name);
        return;
    }
    let tip = format!("Container  ← / → / {}", setup::Item::Sound.keys());
    let mut clicked = None;
    ui.menu_button(name.as_str(), |ui| {
        for (i, sound) in s.sounds.iter().enumerate() {
            if ui.selectable_label(i == s.selected, &sound.name).clicked() {
                clicked = (i != s.selected).then_some(Button::Sound(i));
                ui.close_menu();
            }
        }
        if ui
            .selectable_label(false, "+")
            .on_hover_text("New container  Ctrl+Shift+N")
            .clicked()
        {
            clicked = Some(Button::NewSound);
            ui.close_menu();
        }
    })
    .response
    .on_hover_text(tip);
    if clicked.is_some() {
        s.clicked = clicked;
    }
}

#[path = "choices/creation.rs"]
mod creation;
pub(super) use creation::{keys as creation_keys, show as creation};
mod kit;
pub(super) use kit::{poll as poll_kit, start as create_kit, KitCreation};

/// The last press's velocity and the CC1 sent now; a click lists the choices, the modulation's
/// only until a take is armed.
pub(super) fn expression(ui: &mut egui::Ui, s: &mut Session) {
    let now = s.clock.elapsed();
    let velocity = super::phrase::replayed(s, now).or_else(|| {
        s.recorder
            .as_ref()
            .map(Recorder::velocity)
            .filter(|&v| v > 0)
    });
    let shown = |v: Option<u8>| v.map_or("-".into(), |v| v.to_string());
    let t = *s.settings();
    // Right to left, so the modulation comes first.
    let text = egui::RichText::new(format!("mod {:>3}", shown(s.cc1(now)))).monospace();
    let tip = setup::tip(setup::Item::Modulation);
    if !setup::enabled(s, setup::Item::Modulation) {
        ui.label(text).on_hover_text(tip);
    } else if let Some(m) = choose(
        ui,
        text,
        &Modulation::ALL,
        t.modulation,
        Modulation::label,
        &tip,
    ) {
        s.clicked = Some(Button::Modulation(m));
    }
    let text = egui::RichText::new(format!("vel {:>3}", shown(velocity))).monospace();
    let tip = setup::tip(setup::Item::Velocity);
    let label = |v: Velocity| v.label().to_owned();
    if let Some(v) = choose(ui, text, &Velocity::ALL, t.velocity, label, &tip) {
        s.clicked = Some(Button::Velocity(v));
    }
}
