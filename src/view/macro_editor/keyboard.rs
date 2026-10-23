use super::*;
use crate::view::commands::{Command, Screen};
use egui::Key;

pub(super) struct State {
    pub step: f64,
    pub count: Option<usize>,
    pub step_toast_until: Option<f64>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            step: 0.125,
            count: None,
            step_toast_until: None,
        }
    }
}

pub(super) fn collect(screen: Screen, out: &mut Vec<Command>) {
    for (digit, key) in [
        Key::Num0,
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ]
    .into_iter()
    .enumerate()
    {
        out.push(Command::plain(
            screen,
            format!("macro-count-{digit}"),
            format!("反復回数 {digit}"),
            &[key],
            move |a, _| {
                let Some(s) = &mut a.macro_editor.session else {
                    return;
                };
                if digit != 0 || s.keyboard.count.is_some() {
                    s.keyboard.count = Some(
                        s.keyboard
                            .count
                            .unwrap_or(0)
                            .saturating_mul(10)
                            .saturating_add(digit),
                    );
                }
            },
        ));
    }
    for (id, name, key, factor) in [
        ("macro-step-less", "変化量を半分", Key::OpenBracket, 0.5),
        ("macro-step-more", "変化量を2倍", Key::CloseBracket, 2.0),
    ] {
        out.push(
            Command::plain(screen, id, name, &[key], move |a, ctx| {
                if let Some(s) = &mut a.macro_editor.session {
                    s.keyboard.count = None;
                    s.keyboard.step = (s.keyboard.step * factor).clamp(f64::EPSILON, 1.0);
                    s.keyboard.step_toast_until = Some(ctx.input(|i| i.time) + 0.5);
                }
            })
            .repeating(),
        );
    }
    for (id, name, keys) in [
        ("macro-prev", "前のMacro", &[Key::K, Key::ArrowUp][..]),
        ("macro-next", "次のMacro", &[Key::J, Key::ArrowDown][..]),
        ("macro-less", "値を下げる", &[Key::H, Key::ArrowLeft][..]),
        ("macro-more", "値を上げる", &[Key::L, Key::ArrowRight][..]),
        ("macro-min", "最小", &[Key::Home][..]),
        ("macro-max", "最大", &[Key::End][..]),
    ] {
        out.push(
            Command::plain(screen, id, name, keys, move |a, _| {
                let Some(s) = &mut a.macro_editor.session else {
                    return;
                };
                let count = s.keyboard.count.take().unwrap_or(1);
                match keys[0] {
                    Key::K => s.selected = s.selected.saturating_sub(count),
                    Key::J => s.selected = s.selected.saturating_add(count).min(s.rows.len() - 1),
                    Key::H => s.rows[s.selected]
                        .set(s.rows[s.selected].get() - s.keyboard.step * count as f64),
                    Key::L => s.rows[s.selected]
                        .set(s.rows[s.selected].get() + s.keyboard.step * count as f64),
                    Key::Home => s.rows[s.selected].set(0.0),
                    Key::End => s.rows[s.selected].set(1.0),
                    _ => {}
                }
            })
            .repeating(),
        );
    }
}
