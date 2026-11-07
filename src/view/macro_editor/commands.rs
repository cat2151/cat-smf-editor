use super::*;
use crate::view::commands::{Command, Screen};
use egui::Key;
pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if ((screen == Screen::Main
        || screen == Screen::Timeline
            && (app.piano_roll.open || app.tempo_curve.open || app.parameter_editor.open))
        || screen == Screen::Layers && app.layer_editor.open)
        && let Some(patch) = app.loaded.as_ref().and_then(|l| l.patches.get(app.track))
    {
        if super::super::parameter_editor::is_track(app) {
            return;
        }
        for layer in &patch.layers {
            let Some(instrument) = layer.instrument.as_ref().and_then(|p| {
                cat_smf_editor::instrument_macros::Instrument::of(&p.plugin.format, &p.plugin.id)
            }) else {
                continue;
            };
            let id = layer.settings.id;
            let layer_name = super::super::parameter_editor::layer_label(patch, id);
            out.push(
                Command::plain(
                    screen,
                    format!("macro.variation-curve-{id}"),
                    &[],
                    move |a, _| {
                        super::super::parameter_editor::open_variation(a, id);
                    },
                )
                .named(format!("Macro variation curve · {layer_name}")),
            );
            out.push(
                Command::plain(screen, format!("macro.edit-{id}"), &[], move |a, _| {
                    if let Err(e) = begin(a, id) {
                        a.player_selection.error = Some(e);
                    }
                })
                .named(format!("{} Macro edit · {layer_name}", instrument.name())),
            );
        }
    }
    if screen != Screen::MacroEditor {
        return;
    }
    out.push(Command::plain(
        screen,
        "macro.cancel",
        &[Key::Escape],
        |a, _| {
            if a.macro_editor
                .session
                .as_mut()
                .is_some_and(|s| s.keyboard.count.take().is_some())
            {
                // Escape cancels a pending Vim count before closing the editor.
            } else {
                finish(a, false);
            }
        },
    ));
    out.push(Command::plain(screen, "app.help", &[Key::F1], |a, _| {
        if let Some(s) = &mut a.macro_editor.session {
            s.keyboard.count = None;
        }
        crate::view::help::summon(a);
    }));
    out.push(Command::plain(
        screen,
        "macro.confirm",
        &[Key::Enter],
        |a, _| {
            finish(a, true);
        },
    ));
    for (id, modifiers, full_range) in [
        ("macro.randomize", egui::Modifiers::NONE, false),
        ("macro.randomize-full", egui::Modifiers::SHIFT, true),
    ] {
        out.push(Command::new(
            screen,
            id,
            &[(Key::R, modifiers)],
            move |a, _| {
                if let Some(s) = &mut a.macro_editor.session {
                    s.keyboard.count = None;
                    s.randomize(full_range);
                }
            },
        ));
    }
    for (id, key) in [
        ("macro.duplicate", Key::D),
        ("macro.variation-prev", Key::PageUp),
        ("macro.variation-next", Key::PageDown),
    ] {
        out.push(Command::plain(screen, id, &[key], move |a, _| {
            let Some(s) = &mut a.macro_editor.session else {
                return;
            };
            s.keyboard.count = None;
            let count = s.variations.len().max(1);
            match key {
                Key::D => s.duplicate_variation(),
                Key::PageUp => s.switch_variation((s.variation + count - 1) % count),
                Key::PageDown => s.switch_variation((s.variation + 1) % count),
                _ => {}
            }
        }));
    }
    keyboard::collect(screen, out);
}
