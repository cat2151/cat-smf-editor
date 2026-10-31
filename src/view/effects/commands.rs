use super::*;
use crate::view::commands::{Command, Screen, Spec};
use egui::{Key, Modifiers};

#[derive(Clone, Copy)]
pub(super) enum Op {
    Help,
    Close,
    Row(bool),
    Move(bool),
    Add,
    Replace,
    Bypass,
    Remove,
}
impl State {
    pub(crate) fn command_screen(&self) -> Screen {
        Screen::Effects
    }
    pub(crate) fn command_target(&self) -> (Option<Target>, usize) {
        (self.target, self.selected)
    }
}
pub(super) fn specs(state: &State, count: usize) -> Vec<Spec<Op>> {
    use Op::*;
    let p = Spec::plain;
    let mut r = vec![
        p("help", "ヘルプ", &[Key::F1], Help),
        p("effects-close", "閉じる", &[Key::Escape, Key::Enter], Close),
    ];
    r.extend([
        p("effect-add", "エフェクト追加", &[Key::A], Add),
        p("effect-replace", "エフェクト差し替え", &[Key::X], Replace),
    ]);
    if count > 0 {
        r.extend([
            p(
                "effect-next",
                "次のエフェクト",
                &[Key::J, Key::ArrowDown],
                Row(true),
            ),
            p(
                "effect-prev",
                "前のエフェクト",
                &[Key::K, Key::ArrowUp],
                Row(false),
            ),
            p("effect-bypass", "Bypass", &[Key::B], Bypass),
            p("effect-remove", "エフェクト削除", &[Key::Delete], Remove),
        ]);
        for (id, name, keys, down, allowed) in [
            (
                "effect-move-down",
                "順番を下へ",
                [Key::J, Key::ArrowDown],
                true,
                state.selected + 1 < count,
            ),
            (
                "effect-move-up",
                "順番を上へ",
                [Key::K, Key::ArrowUp],
                false,
                state.selected > 0,
            ),
        ] {
            if allowed {
                r.push(Spec {
                    id,
                    name,
                    keys: keys.iter().map(|k| (*k, Modifiers::SHIFT)).collect(),
                    action: Move(down),
                });
            }
        }
    }
    r
}
pub(super) fn apply(state: &mut State, count: usize, op: Op) -> Option<Action> {
    use Op::*;
    match op {
        Help => None,

        Close => Some(Action::Close),
        Row(down) => {
            state.selected = if down {
                (state.selected + 1).min(count.saturating_sub(1))
            } else {
                state.selected.saturating_sub(1)
            };
            state.reveal = true;
            None
        }
        Move(down) => Some(Action::Move {
            from: state.selected,
            to: if down {
                state.selected + 1
            } else {
                state.selected.saturating_sub(1)
            },
        }),
        Add => Some(Action::Browse(count)),
        Replace => Some(Action::Browse(state.selected)),
        Bypass => Some(Action::Bypass(state.selected)),
        Remove => Some(Action::Remove(state.selected)),
    }
}
pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if !matches!(s, Screen::Effects) {
        return;
    }
    let target = a.effect_editor.target.unwrap_or_else(|| {
        if a.playback.mixer_focused() {
            if a.playback.mixer_master {
                Target::Master
            } else {
                Target::Track(a.playback.mixer_track)
            }
        } else {
            Target::Track(a.track)
        }
    });
    let Some(chain) = effects(a, target) else {
        return;
    };
    let closed = State {
        target: Some(target),
        selected: chain.len().saturating_sub(1),
        ..Default::default()
    };
    let state = if a.effect_editor.is_open() {
        &a.effect_editor
    } else {
        &closed
    };
    for spec in specs(state, chain.len()) {
        let op = spec.action;
        out.push(Command::new(
            s,
            spec.id,
            spec.name,
            &spec.keys,
            move |a, _| {
                if matches!(op, Op::Help) {
                    crate::view::help::summon(a);
                    return;
                }
                let Some(chain) = effects(a, target) else {
                    return;
                };
                if let Some(action) = apply(&mut a.effect_editor, chain.len(), op)
                    && let Err(e) = super::apply(a, target, action)
                {
                    a.player_selection.error = Some(e);
                }
            },
        ));
    }
}
