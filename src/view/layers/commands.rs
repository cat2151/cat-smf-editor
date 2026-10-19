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
    Confirm,
    Cancel,
}

impl State {
    pub(crate) fn command_screen(&self) -> Screen {
        if self.editing.is_some() {
            Screen::LayerInput
        } else if self.help {
            Screen::LayerHelp
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
            p("layer-confirm", "数値確定", &[Key::Enter], Confirm),
            p("layer-cancel", "数値取消", &[Key::Escape], Cancel),
        ];
    }
    let mut result = vec![
        p("help", "ヘルプ", &[Key::F1], Help),
        p("layers-close", "閉じる", &[Key::Escape], Close),
    ];
    if state.help {
        return result;
    }
    result.extend([
        p(
            "layer-next",
            "次のレイヤー",
            &[Key::J, Key::ArrowDown],
            Row(true),
        ),
        p(
            "layer-prev",
            "前のレイヤー",
            &[Key::K, Key::ArrowUp],
            Row(false),
        ),
        p(
            "field-prev",
            "前の項目",
            &[Key::H, Key::ArrowLeft],
            Field(false),
        ),
        p(
            "field-next",
            "次の項目",
            &[Key::L, Key::ArrowRight],
            Field(true),
        ),
        p(
            "layer-analysis",
            "Spectrum / Lissajous",
            &[Key::F],
            Analysis,
        ),
    ]);
    if patch.layers.len() < 64 {
        result.push(p("layer-add", "レイヤー複製", &[Key::A], Add));
    }
    if !patch.layers.is_empty() {
        result.extend([
            p("layer-edit", "音色選択 / 数値入力", &[Key::Enter], Edit),
            p("octave-up", "Octave +1", &[Key::O], Nudge(1, true)),
            p("octave-down", "Octave −1", &[Key::I], Nudge(1, false)),
            p(
                "layer-volume-up",
                "音量 +6 dB",
                &[Key::CloseBracket],
                Nudge(2, true),
            ),
            p(
                "layer-volume-down",
                "音量 −6 dB",
                &[Key::OpenBracket],
                Nudge(2, false),
            ),
            p("layer-mute", "Mute", &[Key::M], Mute),
            p("layer-solo", "Solo", &[Key::S], Solo),
            p("layer-remove", "レイヤー削除", &[Key::Delete], Remove),
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
        Help => {
            state.help = !state.help;
            Ok(())
        }
        Close => {
            if state.help {
                state.help = false;
            } else {
                state.open = false;
            }
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
    if !matches!(s, Screen::Layers | Screen::LayerHelp | Screen::LayerInput) {
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
        out.push(Command::new(
            s,
            spec.id,
            spec.name,
            &spec.keys,
            move |a, _| execute(a, op),
        ));
    }
}

fn execute(a: &mut App, op: Op) {
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
    if let Some(id) = choose {
        crate::player::selection::open_layer(a, id);
    }
}
