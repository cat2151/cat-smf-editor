//! Mixer commands share the mixer operation implementation.
use super::*;
use crate::play::mixer;
pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    out.push(Command::plain(
        s,
        "mixer-unfocus",
        "Event listへ戻る",
        &[Key::Escape],
        mixer::unfocus,
    ));
    out.push(Command::plain(
        s,
        "spectrum",
        "Spectrum / Lissajous",
        &[Key::F],
        |a, _| super::super::analysis::toggle(a),
    ));
    for (id, name, keys, key) in [
        (
            "mixer-left",
            "前のtrack / master",
            &[Key::H, Key::ArrowLeft][..],
            mixer::Operation::Previous,
        ),
        (
            "mixer-right",
            "次のtrack",
            &[Key::L, Key::ArrowRight][..],
            mixer::Operation::Next,
        ),
        (
            "mixer-effects",
            "選択対象のエフェクト",
            &[Key::X, Key::Space, Key::Enter][..],
            mixer::Operation::Effects,
        ),
    ] {
        out.push(Command::plain(s, id, name, keys, move |a, _| {
            mixer::apply(a, key);
        }));
    }
    // When reached from Event list, enter() explicitly selects this track, not master.
    if !a.playback.mixer_focused() || !a.playback.mixer_master {
        for (id, name, keys, key) in [
            (
                "volume-down",
                "現在のtrackを減音 −3 dB",
                &[Key::J, Key::ArrowDown][..],
                mixer::Operation::Quieter,
            ),
            (
                "volume-up",
                "現在のtrackを増音 +3 dB",
                &[Key::K, Key::ArrowUp][..],
                mixer::Operation::Louder,
            ),
        ] {
            out.push(Command::plain(s, id, name, keys, move |a, _| {
                mixer::apply(a, key);
            }));
        }
    }
}
