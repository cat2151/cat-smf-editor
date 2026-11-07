//! A new container is created only after its instrument kind is confirmed.
use super::*;

const LABELS: [&str; 3] = ["Init Saw", "Drum", "Drum Kit"];

fn confirm(s: &mut Session) {
    s.clicked = s.new_sound.take().map(|i| match i {
        0 => Button::CreateSound(false),
        1 => Button::CreateSound(true),
        _ => Button::NewKit,
    });
}

/// Consume the dialog's keys before they can play pads or close the keyboard.
pub(in crate::view::recording) fn keys(ctx: &egui::Context, s: &mut Session) -> bool {
    if s.new_sound.is_none() {
        return false;
    }
    let events = ctx.input(|i| i.events.clone());
    ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. })));
    for event in events {
        if let egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        {
            if !commands::matches(modifiers, Modifiers::NONE) {
                continue;
            }
            let Some(i) = s.new_sound.as_mut() else { break };
            match key {
                Key::ArrowUp => *i = i.saturating_sub(1),
                Key::ArrowDown => *i = (*i + 1).min(LABELS.len() - 1),
                Key::Enter => confirm(s),
                Key::Escape => s.new_sound = None,
                _ => {}
            }
        }
    }
    true
}

pub(in crate::view::recording) fn show(ctx: &egui::Context, s: &mut Session) {
    if s.new_sound.is_none() {
        return;
    }
    crate::view::presentation::modal(ctx, egui::Id::new("keyboard-new-container")).show(
        ctx,
        |ui| {
            for (i, label) in LABELS.iter().enumerate() {
                if ui
                    .selectable_label(s.new_sound == Some(i), *label)
                    .clicked()
                {
                    s.new_sound = Some(i);
                    confirm(s);
                }
            }
            ui.horizontal(|ui| {
                if ui.button("Enter: Create").clicked() {
                    confirm(s);
                }
                if ui.button("Esc: Cancel").clicked() {
                    s.new_sound = None;
                }
            });
        },
    );
}
