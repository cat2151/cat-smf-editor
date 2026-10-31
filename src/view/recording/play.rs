//! Playing a session: pads sound and record, Esc commits or closes.
use super::*;
use crate::play::audition::click::Click;
use cat_smf_editor::score::pattern::recording::{Layout, Mode, Pad, Range, Start};

/// Opens the instrument and starts playing; errors keep the session preparing.
pub(super) fn start(a: &mut App, s: &mut Session) -> bool {
    let sound = &s.sounds[s.selected];
    s.settings.drums = sound.patch.drums.is_some();
    let recorder = match Recorder::new(s.division, s.settings) {
        Ok(r) => r,
        Err(error) => {
            s.error = Some(error);
            return false;
        }
    };
    (s.queue, s.click) = a.audition.start_live(sound.track, &sound.patch).unzip();
    s.error = s.queue.is_none().then(|| {
        a.audition
            .error()
            .unwrap_or("Cannot open recording instrument")
            .to_owned()
    });
    if s.queue.is_some() {
        s.recorder = Some(recorder);
    }
    s.recorder.is_some()
}

/// Closes the instrument before another container, keeping changes made while playing.
pub(super) fn stop(a: &mut App, s: &mut Session) {
    if let Some(r) = s.recorder.take() {
        s.settings = *r.settings();
    }
    a.audition.stop_live();
    s.queue = None;
    s.click = None;
}

fn send(s: &mut Session, result: Result<Vec<u8>, String>, on: bool) {
    match result {
        Ok(pitches) => {
            if let Some(q) = &s.queue {
                for pitch in pitches {
                    live::push(q, pitch, on);
                }
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
        window::Button::Stop if armed(s) => close(a, s, at),
        window::Button::Sound(i) if !armed(s) => {
            setup::select(a, s, i);
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
    let base = if t.layout == Layout::Rows && t.mode != Mode::Note {
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

/// Sounds or stops a pad; with `Start::Key`, a loop's first press asks for the count-in.
fn sound(s: &mut Session, pad: Pad, pressed: bool, at: std::time::Duration) {
    let Some(r) = s.recorder.as_mut() else {
        return;
    };
    // Free starts inside the recorder; a loop needs the metronome's count-in.
    if pressed
        && r.settings().start == Start::Key
        && matches!(r.settings().range, Range::Loop { .. })
    {
        arm(r, s.click.as_deref(), at);
    }
    let result = if pressed {
        r.press(pad, at)
    } else {
        r.release(pad, at)
    };
    send(s, result, pressed);
}

/// Moves the mouse of one window onto `target`, releasing the pad it left.
pub(super) fn point(s: &mut Session, from: window::Pointer, target: Option<Pad>) {
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

/// Commits what was recorded; `true` closes the window.
pub(super) fn close(a: &mut App, s: &mut Session, at: std::time::Duration) -> bool {
    let Some(r) = s.recorder.as_mut() else {
        return true;
    };
    let released = r.release_all(at);
    let recorded = r.recorded();
    send(s, released, false);
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
        Key::F1 | Key::Questionmark => super::super::help::summon(a),
        Key::Comma if ctrl => s.settings_open = !s.settings_open,
        Key::Period if ctrl => s.piano_open = !s.piano_open,
        _ => return setup::shortcut(a, s, key, modifiers),
    }
    true
}

/// `true` closes the window.
pub(super) fn process(a: &mut App, ctx: &egui::Context, s: &mut Session) -> bool {
    follow_click(s);
    let at = s.clock.elapsed();
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
    for event in ctx.input(|i| i.events.clone()) {
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
                Key::Escape if commands::matches(modifiers, Modifiers::SHIFT) => return true,
                Key::Escape => return close(a, s, at),
                Key::Space if plain => {
                    if let Some(r) = s.recorder.as_mut().filter(|r| !r.started()) {
                        arm(r, s.click.as_deref(), at);
                    }
                }
                Key::ArrowUp | Key::ArrowDown if plain => {
                    if let Some(r) = s.recorder.as_mut() {
                        octave(r, key == Key::ArrowUp);
                    }
                }
                _ if window_key(a, s, key, modifiers) => continue,
                _ => {}
            }
        }
        let Some(r) = s.recorder.as_ref() else {
            continue;
        };
        let Some(pad) = pads::pad(
            r.settings().layout,
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
    false
}
