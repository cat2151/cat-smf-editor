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
            out.push(Command::plain(
                screen,
                format!("macro-variation-curve-{id}"),
                format!(
                    "Macro variation curve · {}",
                    super::super::parameter_editor::layer_label(patch, id)
                ),
                &[],
                move |a, _| {
                    super::super::parameter_editor::open_variation(a, id);
                },
            ));
            out.push(Command::plain(
                screen,
                format!("macro-edit-{id}"),
                format!(
                    "{} Macro edit · {}",
                    instrument.name(),
                    super::super::parameter_editor::layer_label(patch, id)
                ),
                &[],
                move |a, _| {
                    if let Err(e) = begin(a, id) {
                        a.player_selection.error = Some(e);
                    }
                },
            ));
        }
    }
    if screen != Screen::MacroEditor {
        return;
    }
    out.push(Command::plain(
        screen,
        "macro-cancel",
        "取消",
        &[Key::Escape],
        |a, _| {
            if a.macro_editor.help {
                a.macro_editor.help = false;
            } else if a
                .macro_editor
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
    out.push(Command::plain(
        screen,
        "help",
        "ヘルプ",
        &[Key::F1],
        |a, _| {
            if let Some(s) = &mut a.macro_editor.session {
                s.keyboard.count = None;
            }
            a.macro_editor.help = !a.macro_editor.help;
        },
    ));
    if app.macro_editor.help {
        return;
    }
    out.push(Command::plain(
        screen,
        "macro-confirm",
        "確定",
        &[Key::Enter],
        |a, _| {
            finish(a, true);
        },
    ));
    for (id, name, modifiers, full_range) in [
        (
            "macro-randomize",
            "全MacroをVariation範囲内でランダム化",
            egui::Modifiers::NONE,
            false,
        ),
        (
            "macro-randomize-full",
            "全Macroを全範囲でランダム化",
            egui::Modifiers::SHIFT,
            true,
        ),
    ] {
        out.push(Command::new(
            screen,
            id,
            name,
            &[(Key::R, modifiers)],
            move |a, _| {
                if let Some(s) = &mut a.macro_editor.session {
                    s.keyboard.count = None;
                    s.randomize(full_range);
                }
            },
        ));
    }
    for (id, name, key) in [
        ("macro-duplicate", "Variation複製", Key::D),
        ("macro-variation-prev", "前のVariation", Key::PageUp),
        ("macro-variation-next", "次のVariation", Key::PageDown),
    ] {
        out.push(Command::plain(screen, id, name, &[key], move |a, _| {
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
