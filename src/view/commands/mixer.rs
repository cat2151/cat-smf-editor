//! Mixer commands share the mixer operation implementation.
use super::*;
use crate::play::mixer;
pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    out.push(Command::plain(
        s,
        "mixer.unfocus",
        &[Key::Escape],
        mixer::unfocus,
    ));
    out.push(Command::plain(s, "spectrum.toggle", &[Key::F], |a, _| {
        super::super::analysis::toggle(a)
    }));
    for (id, keys, key) in [
        (
            "mixer.left",
            &[Key::H, Key::ArrowLeft][..],
            mixer::Operation::Previous,
        ),
        (
            "mixer.right",
            &[Key::L, Key::ArrowRight][..],
            mixer::Operation::Next,
        ),
        (
            "mixer.effects",
            &[Key::X, Key::Space, Key::Enter][..],
            mixer::Operation::Effects,
        ),
    ] {
        out.push(Command::plain(s, id, keys, move |a, _| {
            mixer::apply(a, key);
        }));
    }
    // When reached from Event list, enter() explicitly selects this track, not master.
    if !a.playback.mixer_focused() || !a.playback.mixer_master {
        for (id, keys, key) in [
            (
                "track.volume-down",
                &[Key::J, Key::ArrowDown][..],
                mixer::Operation::Quieter,
            ),
            (
                "track.volume-up",
                &[Key::K, Key::ArrowUp][..],
                mixer::Operation::Louder,
            ),
        ] {
            out.push(Command::plain(s, id, keys, move |a, _| {
                mixer::apply(a, key);
            }));
        }
    }
}
