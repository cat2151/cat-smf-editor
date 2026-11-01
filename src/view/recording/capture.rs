//! Every key the keyboard sounds goes to its container's Capture pattern, recording or not;
//! the Capture window lists those phrases to audition and take as patterns.
use super::*;
use cat_smf_editor::message;
use cat_smf_editor::score::pattern::recording::capture::{self, Captured, Keys, Phrase};
use std::time::Duration;
mod rows;
mod shown;

/// Silence that separates two phrases.
const GAP: u64 = 2_000;
pub(super) const TITLE: &str = "Virtual MIDI keyboard · Capture";
/// Key of `config.keyboard_windows`.
const PLACE: &str = "Capture";
const SIZE: egui::Vec2 = egui::vec2(560.0, 420.0);

pub(crate) fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("virtual-midi-keyboard-capture")
}

/// The keys of one keyboard session not yet in the song.
pub(super) struct Pending {
    keys: Keys,
    /// Wall-clock milliseconds at the session clock's zero.
    base: u64,
}

impl Pending {
    pub(super) fn new() -> Self {
        Self {
            keys: Keys::default(),
            base: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
        }
    }
}

/// Notes that started or stopped sounding by the keys.
pub(super) fn record(s: &mut Session, pitches: &[u8], on: bool, at: Duration) {
    let at = s.pending.base + at.as_millis() as u64;
    let container = s.sounds[s.selected].container;
    for &pitch in pitches {
        s.pending.keys.key(at, pitch, on, container, GAP);
    }
}

/// Puts the ended phrases, or with `all` every phrase, in the song without an undo step.
pub(super) fn flush(a: &mut App, s: &mut Session, all: bool) -> Result<(), String> {
    let phrases = if all {
        s.pending.keys.finish()
    } else {
        let now = s.pending.base + s.clock.elapsed().as_millis() as u64;
        s.pending.keys.take(now, GAP)
    };
    for phrase in phrases {
        append(a, s, &phrase)?;
    }
    Ok(())
}

fn append(a: &mut App, s: &mut Session, phrase: &Phrase) -> Result<(), String> {
    let music = &a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    let source = match phrase.container {
        Some(c) => Some(
            *Catalog::read(music)?
                .tracks
                .get(&c)
                .ok_or("phraseのcontainerが無くなりました")?,
        ),
        None => None,
    };
    let tempo = s.settings().tempo;
    let stamp = rows::stamp(phrase.start);
    let mut id = 0;
    a.replace_tracks(|music| {
        id = capture::append(music, source, tempo, phrase, &stamp)?;
        Ok(())
    })?;
    // The keyboard's own edit keeps it open.
    s.song = a.song_id;
    if phrase.container.is_none() {
        let c = Catalog::read(&a.loaded.as_ref().unwrap().music)?;
        let container = c.patterns[&id].container;
        // The fallback sound now plays the container it created.
        for sound in s.sounds.iter_mut().filter(|s| s.container.is_none()) {
            sound.container = Some(container);
            sound.track = c.tracks[&container];
        }
    }
    Ok(())
}

/// The open Capture window.
pub(super) struct Browser {
    /// Newest first.
    phrases: Vec<Captured>,
    /// The song `phrases` were read from; `None` until the first read.
    song: Option<u64>,
    selected: usize,
    /// The editor's view before the first audition.
    back: Option<shown::Back>,
    place: Option<egui::Pos2>,
    focus: bool,
}

/// Opens the window on the newest phrase, or closes it.
pub(super) fn toggle(a: &mut App, s: &mut Session) {
    if s.capture.is_some() {
        close(a, s);
        return;
    }
    s.capture = Some(Browser {
        phrases: vec![],
        song: None,
        selected: 0,
        back: None,
        place: None,
        focus: true,
    });
}

/// Closes the window, putting back the editor's view of before the first audition.
pub(super) fn close(a: &mut App, s: &mut Session) {
    if let Some(back) = s.capture.take().and_then(|b| b.back) {
        back.restore(a);
    }
}

/// Rereads the phrases after the song changed, keeping the selected one; the first read
/// puts the open phrase in the song so it is listed too.
fn read(a: &mut App, s: &mut Session) -> Result<(), String> {
    let Some(song) = s.capture.as_ref().map(|b| b.song) else {
        return Ok(());
    };
    if song == Some(a.song_id) {
        return Ok(());
    }
    if song.is_none() {
        flush(a, s, true)?;
    }
    let phrases = capture::captured(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?;
    let b = s.capture.as_mut().unwrap();
    let kept = b.phrases.get(b.selected).map(|p| (p.pattern, p.range.0));
    b.selected = kept
        .and_then(|k| phrases.iter().position(|p| (p.pattern, p.range.0) == k))
        .unwrap_or(0);
    b.phrases = phrases;
    b.song = Some(a.song_id);
    Ok(())
}

fn auditioning(a: &App, s: &Session) -> bool {
    s.capture
        .as_ref()
        .is_some_and(|b| shown::auditioning(a, &b.phrases))
}

/// Shows the selected phrase in the editor and plays it from its bar.
fn audition(a: &mut App, s: &mut Session) {
    let Some(b) = s.capture.as_mut() else {
        return;
    };
    let Some(p) = b.phrases.get(b.selected).cloned() else {
        return;
    };
    b.back.get_or_insert_with(|| shown::Back::of(a));
    if let Err(error) = shown::audition(a, &p) {
        s.error = Some(error);
    }
}

fn select(a: &mut App, s: &mut Session, index: usize) {
    let Some(b) = s.capture.as_mut() else {
        return;
    };
    if index >= b.phrases.len() || index == b.selected {
        return;
    }
    b.selected = index;
    // Browsing while listening moves the listening along.
    if auditioning(a, s) {
        audition(a, s);
    }
}

fn listen(a: &mut App, s: &mut Session, index: usize) {
    if let Some(b) = s.capture.as_mut().filter(|b| index < b.phrases.len()) {
        b.selected = index;
        audition(a, s);
    }
}

/// The selected phrase's bars as a new pattern of its container, opened as a take is.
fn take(a: &mut App, s: &mut Session) -> Result<(), String> {
    let Some(phrase) = s
        .capture
        .as_ref()
        .and_then(|b| b.phrases.get(b.selected))
        .cloned()
    else {
        return Ok(());
    };
    let mut id = 0;
    a.edit_tracks(|music| {
        id = capture::take(music, &phrase)?;
        Ok(())
    })?;
    // The new pattern replaces the view to go back to.
    if let Some(b) = s.capture.as_mut() {
        b.back = None;
    }
    super::reveal(a, s, id, s.settings().tempo)
}

/// Keys of the focused Capture window.
pub(super) fn keys(a: &mut App, s: &mut Session, events: &[egui::Event]) {
    for event in events {
        let &egui::Event::Key {
            key,
            pressed: true,
            repeat,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        let plain = commands::matches(modifiers, Modifiers::NONE);
        let Some(selected) = s.capture.as_ref().map(|b| b.selected) else {
            return;
        };
        match key {
            Key::ArrowUp if plain => select(a, s, selected.saturating_sub(1)),
            Key::ArrowDown if plain => select(a, s, selected + 1),
            _ if repeat => {}
            Key::Escape if plain => close(a, s),
            Key::Space if plain => {
                if auditioning(a, s) {
                    crate::play::transport::stop(a);
                } else {
                    audition(a, s);
                }
            }
            Key::Enter if plain => {
                if let Err(error) = take(a, s) {
                    s.error = Some(error);
                }
            }
            _ => {
                play::window_key(a, s, key, modifiers);
            }
        }
    }
}

/// The Capture window, opened beside the keyboard and focused so its keys work at once.
pub(super) fn show(a: &mut App, ctx: &egui::Context, s: &mut Session, keyboard: egui::Pos2) {
    if s.capture.is_none() {
        return;
    }
    if let Err(error) = read(a, s) {
        s.error = Some(error);
    }
    let b = s.capture.as_ref().unwrap();
    let at = b.place.or(companions::saved(a, PLACE)).unwrap_or_else(|| {
        let other = s
            .companions
            .settings
            .map(|at| (at, companions::SETTINGS_SIZE));
        companions::place(a, ctx, keyboard, other, SIZE)
    });
    let b = s.capture.as_mut().unwrap();
    b.place = Some(at);
    if std::mem::take(&mut b.focus) {
        ctx.send_viewport_cmd_to(viewport(), egui::ViewportCommand::Focus);
    }
    let builder = egui::ViewportBuilder::default()
        .with_title(TITLE)
        .with_position(at)
        .with_inner_size(SIZE);
    let mut clicked = None;
    ctx.show_viewport_immediate(viewport(), builder, |ctx, class| {
        if class != egui::ViewportClass::Embedded {
            if ctx.input(|i| i.viewport().close_requested()) {
                close(a, s);
                return;
            }
            if let Some(at) = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min) {
                companions::remember(a, PLACE, at);
            }
            super::super::help::observe_recording(a, ctx);
            if s.first_read(ctx) {
                keys(a, s, &ctx.input(|i| i.events.clone()));
                if s.capture.is_none() {
                    ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Focus);
                    return;
                }
            }
        }
        let sounding = auditioning(a, s);
        let bpm = a.patterns.tempo.0;
        let body = |ui: &mut egui::Ui| clicked = rows::list(ui, s, sounding, bpm);
        if class == egui::ViewportClass::Embedded {
            egui::Window::new(TITLE).show(ctx, body);
        } else {
            egui::CentralPanel::default().show(ctx, body);
        }
    });
    match clicked {
        Some(rows::Click::Take(i)) => {
            if let Some(b) = s.capture.as_mut() {
                b.selected = i;
            }
            if let Err(error) = take(a, s) {
                s.error = Some(error);
            }
        }
        Some(rows::Click::Listen(i)) => listen(a, s, i),
        None => {}
    }
}

#[cfg(test)]
pub(super) fn phrases<'s>(a: &App, s: &'s Session) -> Option<(&'s [Captured], usize, bool)> {
    let sounding = auditioning(a, s);
    s.capture
        .as_ref()
        .map(|b| (b.phrases.as_slice(), b.selected, sounding))
}
