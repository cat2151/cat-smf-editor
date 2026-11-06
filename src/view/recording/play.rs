//! Playing a session: pads sound and record, Esc commits or closes.
use super::*;
use crate::play::audition::click::Click;
use cat_smf_editor::score::pattern::recording::{Pad, Range};

/// Every container as the audio thread plays it.
pub(super) fn sounds(s: &Session) -> Vec<(usize, Patch, live::Live)> {
    s.sounds
        .iter()
        .map(|x| (x.track, x.patch.clone(), x.live.clone()))
        .collect()
}

/// Opens the instruments of every container once, so that another container plays without
/// cutting the phrases, and starts playing the selected one; errors keep the session preparing.
pub(super) fn start(a: &mut App, s: &mut Session) -> bool {
    s.settings.drums = s.sound().patch.drums.is_some();
    let recorder = match Recorder::new(s.division, s.settings) {
        Ok(r) => r,
        Err(error) => {
            s.error = Some(error);
            return false;
        }
    };
    if s.click.is_none() {
        let click = std::sync::Arc::new(Click::new());
        s.click = a.audition.start_live(&sounds(s), &click).then_some(click);
    }
    s.error = s.click.is_none().then(|| {
        a.audition
            .error()
            .unwrap_or(message::CANNOT_OPEN_RECORDING_INSTRUMENT)
            .to_owned()
    });
    if s.click.is_some() {
        s.recorder = Some(recorder);
    }
    s.recorder.is_some()
}

/// Leaves the playing container with its keys released, keeping changes made while playing;
/// its phrase goes on.
pub(super) fn leave(s: &mut Session) {
    let at = s.clock.elapsed();
    if let Some(r) = s.recorder.as_mut() {
        let released = r.release_all(at);
        send(s, released, 0, at);
    }
    s.mouse = [None; 2];
    if let Some(r) = s.recorder.take() {
        s.settings = *r.settings();
    }
}

/// Pitches that started at `velocity`, or stopped for 0.
fn send(s: &mut Session, result: Result<Vec<u8>, String>, velocity: u8, at: std::time::Duration) {
    match result {
        Ok(pitches) => {
            // Before the keys, so a replaced phrase's Note Off cannot cut a key of its pitch.
            super::phrase::heard(s, &pitches, velocity, at);
            super::capture::record(s, &pitches, velocity, at);
            if let Some(q) = s.queue() {
                live::push(q, &pitches, velocity);
            }
        }
        Err(error) => s.error = Some(error),
    }
}

/// Free starts the clock now. Loop asks the audio thread's first click for one measure of
/// count-in, then the loop's first lap.
fn arm(r: &mut Recorder, click: Option<&Click>, at: std::time::Duration) {
    match (r.settings().range, click) {
        (Range::Free, _) => r.start(at),
        (Range::Loop { .. }, Some(click)) => click.request(r.settings().tempo),
        (Range::Loop { .. }, None) => {}
    }
}

/// Counting in or recording.
pub(super) fn armed(s: &Session) -> bool {
    s.recorder.as_ref().is_some_and(Recorder::started)
        || s.click.as_ref().is_some_and(|c| c.requested())
}

/// A clicked transport button; `true` closes the window.
pub(super) fn button(a: &mut App, s: &mut Session, button: window::Button) -> bool {
    let at = s.clock.elapsed();
    match button {
        window::Button::Rec if !armed(s) => {
            if let Some(r) = s.recorder.as_mut() {
                arm(r, s.click.as_deref(), at);
            }
            false
        }
        window::Button::Stop if armed(s) => end(a, s, at),
        window::Button::Sound(i) if !armed(s) => {
            setup::select(a, s, i);
            false
        }
        window::Button::NewSound if !armed(s) && s.kit_creation.is_none() => {
            let released = s.recorder.as_mut().map(|r| r.release_all(at));
            if let Some(released) = released {
                send(s, released, 0, at);
            }
            s.mouse = [None; 2];
            s.new_sound = Some(0);
            false
        }
        window::Button::NewKit if !armed(s) => {
            super::choices::create_kit(a, s);
            false
        }
        window::Button::CreateSound(drums) if !armed(s) => {
            match super::super::patterns::library::new_container(a, drums) {
                Ok(id) => {
                    if super::phrase::adopt(a, s) {
                        super::phrase::retune(a, s);
                    }
                    if let Some(i) = s.sounds.iter().position(|x| x.container == Some(id)) {
                        setup::select(a, s, i);
                    }
                }
                Err(error) => s.error = Some(error),
            }
            false
        }
        window::Button::Velocity(v) => {
            s.settings_mut().velocity = v;
            false
        }
        window::Button::Modulation(m) => {
            setup::modulate(a, s, m);
            false
        }
        window::Button::Phrase => {
            super::phrase::toggle(a, s);
            false
        }
        window::Button::Capture => {
            super::capture::toggle(a, s);
            false
        }
        window::Button::ToSong => {
            if let Err(error) = super::phrase::to_song(a, s) {
                s.error = Some(error);
            }
            false
        }
        _ => false,
    }
}

/// Takes the loop origin from the metronome once it has started.
pub(super) fn follow_click(s: &mut Session) {
    let (Some(r), Some(origin)) = (
        s.recorder.as_mut(),
        s.click.as_ref().and_then(|c| c.origin()),
    ) else {
        return;
    };
    if !r.started() {
        r.start(origin.saturating_duration_since(s.clock));
    }
}

fn octave(r: &mut Recorder, up: bool) {
    let t = r.settings_mut();
    let base = if t.mode.chord_notes().is_some() {
        &mut t.chord_base
    } else {
        &mut t.note_base
    };
    let moved = if up {
        base.checked_add(12).filter(|&p| p < 128)
    } else {
        base.checked_sub(12)
    };
    *base = moved.unwrap_or(*base);
}

/// Sounds or stops a pad.
fn sound(s: &mut Session, pad: Pad, pressed: bool, at: std::time::Duration) {
    let Some(r) = s.recorder.as_mut() else {
        return;
    };
    let (result, velocity) = if pressed {
        let result = r.press(pad, at);
        (result, r.velocity())
    } else {
        (r.release(pad, at), 0)
    };
    send(s, result, velocity, at);
}

/// Moves the mouse of one window onto `target`, releasing the pad it left.
pub(super) fn point(s: &mut Session, from: window::Pointer, target: Option<Pad>) {
    let target = target.filter(|_| s.new_sound.is_none());
    let previous = std::mem::replace(&mut s.mouse[from as usize], target);
    if previous == target {
        return;
    }
    let at = s.clock.elapsed();
    if let Some(pad) = previous {
        sound(s, pad, false, at);
    }
    if let Some(pad) = target {
        sound(s, pad, true, at);
    }
}

/// The Song window keeps the keyboard; only a song without patterns lets it close.
pub(super) fn kept(a: &App) -> bool {
    crate::view::patterns::mode::song_window(a)
}

/// Commits what was recorded, then records anew while the keyboard is `kept`; `true` closes
/// the window.
pub(super) fn end(a: &mut App, s: &mut Session, at: std::time::Duration) -> bool {
    if !close(a, s, at) {
        return false;
    }
    if !kept(a) {
        return true;
    }
    leave(s);
    start(a, s);
    false
}

/// Drops the take and records anew; the keys played so far stay in the Capture.
fn discard(a: &mut App, s: &mut Session, at: std::time::Duration) {
    if let Some(r) = s.recorder.as_mut() {
        let released = r.release_all(at);
        send(s, released, 0, at);
    }
    if let Err(error) = super::capture::flush(a, s, true) {
        a.load_error = Some(error);
    }
    leave(s);
    start(a, s);
}

/// Commits what was recorded; `true` closes the window.
pub(super) fn close(a: &mut App, s: &mut Session, at: std::time::Duration) -> bool {
    let Some(r) = s.recorder.as_mut() else {
        return true;
    };
    let released = r.release_all(at);
    let recorded = r.recorded();
    send(s, released, 0, at);
    // Before the take, so undoing the take keeps the keys.
    if let Err(error) = super::capture::flush(a, s, true) {
        a.load_error = Some(error);
    }
    if !recorded {
        return true;
    }
    match finish(a, s) {
        Ok(()) => true,
        Err(error) => {
            s.error = Some(error);
            false
        }
    }
}

/// Window keys shared by the keyboard and settings windows; `false` when the key is not one.
pub(super) fn window_key(a: &mut App, s: &mut Session, key: Key, modifiers: Modifiers) -> bool {
    let ctrl = commands::matches(modifiers, Modifiers::CTRL);
    match key {
        Key::N if commands::matches(modifiers, Modifiers::CTRL | Modifiers::SHIFT) => {
            button(a, s, window::Button::NewSound);
        }
        Key::F1 | Key::Questionmark => super::super::help::summon(a),
        Key::Comma if ctrl => s.settings_open = !s.settings_open,
        Key::Period if ctrl => s.piano_open = !s.piano_open,
        Key::E if ctrl => super::phrase::toggle(a, s),
        Key::T if ctrl => s.clicked = Some(window::Button::Browse),
        Key::J if ctrl => super::capture::toggle(a, s),
        Key::Enter if ctrl => s.clicked = Some(window::Button::ToSong),
        _ => return setup::shortcut(a, s, key, modifiers),
    }
    true
}

/// `true` closes the window.
pub(super) fn process(a: &mut App, ctx: &egui::Context, s: &mut Session) -> bool {
    super::choices::poll_kit(a, s);
    follow_click(s);
    super::phrase::follow(a, s);
    if super::phrase::follow_patch(a, s) {
        return true;
    }
    let at = s.clock.elapsed();
    // An ended Capture phrase takes the sweep it was played under.
    if let Err(error) = super::capture::flush(a, s, false) {
        s.error = Some(error);
    }
    super::sweep::follow(s, at);
    let focused = ctx.input(|i| {
        i.focused
            && !i
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false)))
    });
    s.raise |= ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::WindowFocused(true)))
    });
    if !focused {
        // A key release goes to the focused window; a piano key still follows its mouse.
        let keys: Vec<Pad> = s.recorder.as_ref().map_or(vec![], |r| {
            r.held_pads()
                .filter(|p| !s.mouse.contains(&Some(*p)))
                .collect()
        });
        for pad in keys {
            sound(s, pad, false, at);
        }
        return false;
    }
    if super::choices::creation_keys(ctx, s) {
        return false;
    }
    for event in ctx.input(|i| i.events.clone()) {
        if s.new_sound.is_some() {
            break;
        }
        let egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
        } = event
        else {
            continue;
        };
        let plain = commands::matches(modifiers, Modifiers::NONE);
        if pressed && !repeat {
            match key {
                Key::Escape if commands::matches(modifiers, Modifiers::SHIFT) => {
                    if !kept(a) {
                        return true;
                    }
                    discard(a, s, at);
                }
                Key::Escape => return end(a, s, at),
                Key::Space if plain => {
                    if let Some(r) = s.recorder.as_mut().filter(|r| !r.started()) {
                        arm(r, s.click.as_deref(), at);
                    }
                }
                // The Stop button's key; unlike Esc it never closes an unarmed window.
                Key::Space if commands::matches(modifiers, Modifiers::SHIFT) => {
                    if armed(s) && button(a, s, window::Button::Stop) {
                        return true;
                    }
                }
                Key::ArrowUp | Key::ArrowDown if plain => {
                    if let Some(r) = s.recorder.as_mut() {
                        octave(r, key == Key::ArrowUp);
                    }
                }
                Key::ArrowLeft | Key::ArrowRight if plain => {
                    let step = if key == Key::ArrowLeft { -1 } else { 1 };
                    setup::change(a, s, setup::Item::Sound, step, false);
                }
                _ if window_key(a, s, key, modifiers) => continue,
                _ => {}
            }
        }
        let Some(r) = s.recorder.as_ref() else {
            continue;
        };
        let Some(pad) = pads::pad(
            r.settings().mode,
            key,
            physical_key,
            pressed,
            |p| r.held(p),
            s.down,
        ) else {
            continue;
        };
        if pressed && (repeat || !plain) {
            continue;
        }
        sound(s, pad, pressed, at);
    }
    super::sweep::follow(s, at);
    false
}
