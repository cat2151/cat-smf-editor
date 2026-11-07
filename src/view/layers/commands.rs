use super::*;
use crate::view::commands::{Command, Screen, Spec};
use egui::Key;

#[derive(Clone, Copy)]
pub(super) enum Op {
    Help,
    Close,
    Row(bool),
    Field(bool),
    Edit,
    Nudge(usize, bool),
    Mute,
    Solo,
    Add,
    Remove,
    Analysis,
    Automation,
    Confirm,
    Cancel,
}

impl State {
    pub(crate) fn command_screen(&self) -> Screen {
        if self.editing.is_some() {
            Screen::LayerInput
        } else {
            Screen::Layers
        }
    }
    pub(crate) fn command_target(&self) -> (Option<u64>, usize) {
        (self.selected, self.field)
    }
}

pub(super) fn specs(state: &State, patch: &Patch) -> Vec<Spec<Op>> {
    use Op::*;
    let p = Spec::plain;
    if state.editing.is_some() {
        return vec![
            p("layer.confirm", &[Key::Enter], Confirm),
            p("layer.cancel", &[Key::Escape], Cancel),
        ];
    }
    let mut result = vec![
        p("app.help", &[Key::F1], Help),
        p("layer.close", &[Key::Escape], Close),
    ];
    result.extend([
        p("layer.next", &[Key::J, Key::ArrowDown], Row(true)),
        p("layer.prev", &[Key::K, Key::ArrowUp], Row(false)),
        p("layer.field-prev", &[Key::H, Key::ArrowLeft], Field(false)),
        p("layer.field-next", &[Key::L, Key::ArrowRight], Field(true)),
        p("layer.analysis", &[Key::F], Analysis),
    ]);
    if patch.layers.len() < 64 {
        result.push(p("layer.add", &[Key::A], Add));
    }
    if !patch.layers.is_empty() {
        result.extend([
            p("layer.automation", &[Key::U], Automation),
            p("layer.edit", &[Key::Enter], Edit),
            p("layer.octave-up", &[Key::O], Nudge(1, true)),
            p("layer.octave-down", &[Key::I], Nudge(1, false)),
            p("layer.volume-up", &[Key::CloseBracket], Nudge(2, true)),
            p("layer.volume-down", &[Key::OpenBracket], Nudge(2, false)),
            p("layer.mute", &[Key::M], Mute),
            p("layer.solo", &[Key::S], Solo),
            p("layer.remove", &[Key::Delete], Remove),
        ]);
    }
    result
}

pub(super) fn apply(
    state: &mut State,
    patch: &mut Patch,
    op: Op,
    choose: &mut Option<u64>,
    error: &mut Option<String>,
) -> bool {
    use Op::*;
    let row = state.row(patch);
    let result = match op {
        Help => Ok(()),
        Close => {
            state.open = false;
            Ok(())
        }
        Row(down) => {
            let i = if down {
                (row + 1).min(patch.layers.len().saturating_sub(1))
            } else {
                row.saturating_sub(1)
            };
            state.selected = patch.layers.get(i).map(|l| l.settings.id);
            state.reveal = true;
            Ok(())
        }
        Field(next) => {
            state.field = if next {
                (state.field + 1).min(2)
            } else {
                state.field.saturating_sub(1)
            };
            Ok(())
        }
        Edit => {
            if state.field == 0 {
                *choose = state.selected;
            } else {
                state.begin(patch);
            }
            Ok(())
        }
        Nudge(field, up) => super::nudge(patch, row, field, up),
        Mute | Solo => {
            let mut candidate = patch.clone();
            if let Some(layer) = candidate.layers.get_mut(row) {
                let v = if matches!(op, Mute) {
                    &mut layer.settings.mute
                } else {
                    &mut layer.settings.solo
                };
                *v = !*v;
            }
            candidate.validate().map(|_| *patch = candidate)
        }
        Add => state.add(patch),
        Remove => {
            if let Some(id) = state.selected {
                state.remove(patch, id);
            }
            Ok(())
        }
        Automation => {
            state.automation = state.selected;
            Ok(())
        }
        Analysis => return true,
        Confirm => state.commit(patch),
        Cancel => {
            state.editing = None;
            Ok(())
        }
    };
    if let Err(e) = result {
        *error = Some(e);
    }
    false
}

pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if !matches!(s, Screen::Layers | Screen::LayerInput) {
        return;
    }
    let Some(patch) = a.loaded.as_ref().and_then(|l| l.patches.get(a.track)) else {
        return;
    };
    let closed = State {
        open: true,
        selected: patch.layers.first().map(|l| l.settings.id),
        ..Default::default()
    };
    let state = if a.layer_editor.open {
        &a.layer_editor
    } else {
        &closed
    };
    for spec in specs(state, patch) {
        let op = spec.action;
        out.push(Command::new(s, spec.id, &spec.keys, move |a, _| {
            execute(a, op)
        }));
    }
}

fn execute(a: &mut App, op: Op) {
    if matches!(op, Op::Help) {
        crate::view::help::summon(a);
        return;
    }
    let Some(mut patch) = a
        .loaded
        .as_ref()
        .and_then(|l| l.patches.get(a.track))
        .cloned()
    else {
        return;
    };
    let before = patch.clone();
    let mut choose = None;
    let mut error = None;
    if apply(&mut a.layer_editor, &mut patch, op, &mut choose, &mut error) {
        crate::view::analysis::toggle(a);
    }
    if patch != before
        && let Err(e) = crate::player::selection::store_track(a, a.track, &patch)
    {
        error = Some(e);
    }
    if let Some(e) = error {
        a.player_selection.error = Some(e);
    }
    if let Some(id) = a.layer_editor.automation.take() {
        crate::view::parameter_editor::choose(a, id);
    }
    if let Some(id) = choose {
        crate::player::selection::open_layer(a, id);
    }
}
