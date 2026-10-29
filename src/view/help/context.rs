use crate::app::App;
use crate::view::commands::{owner, Screen};
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Target {
    #[default]
    Main,
    Piano,
    Tempo,
    Parameter,
    Overview,
    Layers,
    Effects,
    Drums,
    Macro,
    Picker,
    Scene,
    Song,
    Views,
    Mml,
    Palette,
    Recording,
    Mixer,
    Singer,
    Recent,
    Selection,
    SelectionMenu,
}

#[derive(Default)]
pub struct State {
    pub(super) target: Target,
    pub(super) geometry: Option<[f32; 4]>,
}

/// Called before modal suspension, including native companion viewports.
pub(crate) fn input(a: &mut App, ctx: &egui::Context) {
    if ctx.viewport_id() == super::viewport() {
        return;
    }
    let pressed = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(e,
        egui::Event::Key { key: egui::Key::F1, pressed: true, repeat: false, modifiers, .. }
        if *modifiers == egui::Modifiers::NONE)
        })
    });
    if pressed {
        a.help_open = !a.help_open;
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::F1,
                        ..
                    }
                )
            })
        });
        ctx.request_repaint();
    }
}

pub(crate) fn observe(a: &mut App, ctx: &egui::Context) {
    if ctx.viewport_id() == super::viewport() {
        return;
    }
    let next = resolve(a, ctx);
    // A disposed editor must not survive merely because Help has focus.
    if ctx.input(|i| i.viewport().focused.unwrap_or(i.focused)) || !valid(a, a.help.target) {
        a.help.target = next;
    }
}

fn resolve(a: &App, ctx: &egui::Context) -> Target {
    if a.patterns.recording.is_open() {
        return Target::Recording;
    }
    if a.palette.is_open() {
        return Target::Palette;
    }
    if a.command_input.views_open() {
        return Target::Views;
    }
    match owner(a, ctx) {
        Screen::Mml => Target::Mml,
        Screen::Mixer => Target::Mixer,
        Screen::Singer => Target::Singer,
        Screen::Recent => Target::Recent,
        Screen::Selection => Target::Selection,
        Screen::SelectionMenu => Target::SelectionMenu,
        Screen::Scene => Target::Scene,
        Screen::Song => Target::Song,
        // A query belongs to its pane, not to an unrelated root editor.
        Screen::ArrangementInput => {
            crate::view::patterns::workspace::pane(ctx).map_or(Target::Main, |p| match p {
                crate::view::patterns::workspace::Pane::Scene => Target::Scene,
                crate::view::patterns::workspace::Pane::Song => Target::Song,
            })
        }
        Screen::Analysis => Target::Main,
        _ => resolve_root(a, ctx),
    }
}

fn resolve_root(a: &App, ctx: &egui::Context) -> Target {
    match owner(a, ctx) {
        Screen::Layers | Screen::LayerInput => Target::Layers,
        Screen::Effects => Target::Effects,
        Screen::Drums | Screen::DrumInput | Screen::DrumCreate => Target::Drums,
        Screen::MacroEditor => Target::Macro,
        Screen::ParameterPicker => Target::Picker,
        Screen::Overview => Target::Overview,
        Screen::Timeline | Screen::ParameterInput => timeline(a, ctx),
        _ => Target::Main,
    }
}

fn timeline(a: &App, ctx: &egui::Context) -> Target {
    // Egui's frontmost window is the keyboard recipient within the root viewport.
    let top = ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle));
    for (target, ids) in [
        (Target::Piano, ["Piano roll", "piano-roll-maximized"]),
        (Target::Tempo, ["Tempo curve", "tempo-curve-maximized"]),
        (
            Target::Parameter,
            ["parameter-curve", "parameter-curve-maximized"],
        ),
    ] {
        if valid(a, target) && top.is_some_and(|l| ids.iter().any(|id| l.id == egui::Id::new(id))) {
            return target;
        }
    }
    if a.parameter_editor.open {
        Target::Parameter
    } else if a.piano_roll.open {
        Target::Piano
    } else if a.tempo_curve.open {
        Target::Tempo
    } else {
        Target::Main
    }
}

fn valid(a: &App, target: Target) -> bool {
    match target {
        Target::Main => true,
        Target::Piano => a.piano_roll.open,
        Target::Tempo => a.tempo_curve.open,
        Target::Parameter => a.parameter_editor.open,
        Target::Overview => a.track_overview.open,
        Target::Layers => a.layer_editor.open,
        Target::Effects => a.effect_editor.is_open(),
        Target::Drums => crate::view::drums::is_open(a),
        Target::Macro => a.macro_editor.open(),
        Target::Picker => a.parameter_editor.picker_open(),
        Target::Scene => {
            crate::view::patterns::mode::workspace_open(a) && a.patterns.workspace.scene_window
        }
        Target::Song => crate::view::patterns::mode::workspace_open(a),
        Target::Views => a.command_input.views_open(),
        Target::Mml => a.mml_input.open,
        Target::Palette => a.palette.is_open(),
        Target::Recording => a.patterns.recording.is_open(),
        Target::Mixer => a.playback.mixer_focused(),
        Target::Singer => a.voicevox.singer.is_some(),
        Target::Recent => a.recent.open,
        Target::Selection => {
            crate::view::event_selection::valid(a)
                && a.event_selection.command_screen() == Screen::Selection
        }
        Target::SelectionMenu => {
            crate::view::event_selection::valid(a)
                && a.event_selection.command_screen() == Screen::SelectionMenu
        }
    }
}
