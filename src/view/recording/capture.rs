//! Every key the keyboard sounds goes to its container's Capture pattern, recording or not;
//! the Capture window lists those phrases to audition and take as patterns.
use super::*;
use cat_smf_editor::message;
use cat_smf_editor::score::pattern::recording::capture::{self, Captured, Keys, Phrase};
use std::time::Duration;
mod filter;
mod playback;
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
}

impl Pending {
    pub(super) fn new() -> Self {
        Self {
            keys: Keys::default(),
        }
    }
}

/// Notes that started at `velocity`, or stopped for 0, by the keys.
pub(super) fn record(s: &mut Session, pitches: &[u8], velocity: u8, at: Duration) {
    let at = at.as_millis() as u64;
    let container = s.sounds[s.selected].container;
    for &pitch in pitches {
        s.pending.keys.key(at, pitch, velocity, container, GAP);
    }
}

/// A phrase goes on that silence has not ended yet.
pub(super) fn playing(s: &Session, at: Duration) -> bool {
    s.pending.keys.playing(at.as_millis() as u64, GAP)
}

/// Puts the ended phrases, or with `all` every phrase, in the song without an undo step.
pub(super) fn flush(a: &mut App, s: &mut Session, all: bool) -> Result<(), String> {
    let phrases = if all {
        s.pending.keys.finish()
    } else {
        let now = s.clock.elapsed().as_millis() as u64;
        s.pending.keys.take(now, GAP)
    };
    for phrase in phrases {
        append(a, s, &phrase)?;
    }
    Ok(())
}

/// Puts `phrase` in the song with the CC1 the keyboard played over it.
fn append(a: &mut App, s: &mut Session, phrase: &Phrase) -> Result<(), String> {
    let source = super::take::source(a, phrase.container)?;
    let tempo = s.settings().tempo;
    let from = Duration::from_millis(phrase.start);
    let controls: Vec<(u64, u8)> = s
        .settings()
        .modulation
        .changes(
            s.sweep_start(),
            from,
            from + Duration::from_millis(phrase.end()),
        )
        .into_iter()
        .map(|(at, value)| ((at - from).as_millis() as u64, value))
        .collect();
    let mut id = 0;
    a.append_tracks(|music| {
        id = capture::append(music, source, tempo, phrase, &controls)?;
        Ok(())
    })?;
    super::take::adopt(a, s, id)
}

/// The open Capture window.
pub(super) struct Browser {
    all_phrases: Vec<Captured>,
    container: Option<u64>,
    /// Newest first.
    phrases: Vec<Captured>,
    /// The `App::generation` `phrases` were read at; `None` until the first read.
    generation: Option<u64>,
    selected: usize,
    /// The editor's view before the first audition.
    back: Option<shown::Back>,
    place: Option<egui::Pos2>,
    focus: bool,
    caption: String,
    playback: Option<playback::Playing>,
}

/// Opens the window on the newest phrase, or closes it.
pub(super) fn toggle(a: &mut App, s: &mut Session) {
    if s.capture.is_some() {
        close(a, s);
        return;
    }
    s.capture = Some(Browser {
        all_phrases: vec![],
        container: None,
        phrases: vec![],
        generation: None,
        selected: 0,
        back: None,
        place: None,
        focus: true,
        caption: TITLE.into(),
        playback: None,
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
    let Some(generation) = s.capture.as_ref().map(|b| b.generation) else {
        return Ok(());
    };
    if generation == Some(a.generation) {
        return Ok(());
    }
    if generation.is_none() {
        flush(a, s, true)?;
    }
    let phrases = capture::captured(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?;
    let b = s.capture.as_mut().unwrap();
    b.all_phrases = phrases;
    filter::refresh(b);
    b.generation = Some(a.generation);
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
    b.playback = None;
    match playback::start(a, &p, &b.phrases) {
        Ok(playing) => b.playback = Some(playing),
        Err(error) => s.error = Some(error),
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
    super::reveal(a, s, id)
}

/// Clear persisted and pending captures; ordinary patterns and sounds stay in place.
fn clear(a: &mut App, s: &mut Session) -> Result<(), String> {
    a.append_tracks(capture::clear)?;
    s.pending = Pending::new();
    if let Some(back) = s.capture.as_mut().and_then(|b| b.back.take()) {
        back.restore(a);
    }
    read(a, s)
}

pub(super) fn position(a: &App, s: &Session) -> Option<(usize, u64)> {
    s.capture
        .as_ref()
        .and_then(|b| shown::position(a, &b.phrases))
}

pub(super) fn title(a: &App, s: &Session) -> String {
    s.capture
        .as_ref()
        .and_then(|b| {
            b.playback
                .as_ref()
                .and_then(|p| p.caption(a, &b.phrases))
                .or_else(|| shown::caption(a, &b.phrases))
        })
        .map_or_else(|| TITLE.into(), |caption| format!("{TITLE} · {caption}"))
}

/// Advance before the transport clears a finished phrase.
pub(super) fn advance(a: &mut App, s: &mut Session) {
    let Some(b) = s.capture.as_mut() else { return };
    let Some(playing) = b.playback.take() else {
        return;
    };
    match playing.advance(a, &b.phrases) {
        Ok(next) => b.playback = next,
        Err(error) => s.error = Some(error),
    }
}

pub(crate) fn advance_app(a: &mut App) {
    let mut state = std::mem::take(&mut a.patterns.recording);
    if let Some(s) = state.session.as_mut() {
        advance(a, s);
    }
    a.patterns.recording = state;
}

pub(super) fn native_title(s: &Session) -> &str {
    s.capture.as_ref().map_or(TITLE, |b| b.caption.as_str())
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
            Key::Delete if commands::matches(modifiers, Modifiers::CTRL | Modifiers::SHIFT) => {
                if let Err(error) = clear(a, s) {
                    s.error = Some(error);
                }
            }
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
            .map(|at| (at, s.companions.settings_size()));
        companions::place(a, ctx, keyboard, other, SIZE)
    });
    let visible = super::super::presentation::Native::Keyboard.visible(a, ctx, viewport());
    let b = s.capture.as_mut().unwrap();
    b.place = Some(at);
    if visible && std::mem::take(&mut b.focus) {
        ctx.send_viewport_cmd_to(viewport(), egui::ViewportCommand::Focus);
    }
    let title = title(a, s);
    s.capture.as_mut().unwrap().caption.clone_from(&title);
    let builder = egui::ViewportBuilder::default()
        .with_title(&title)
        .with_position(at)
        .with_inner_size(SIZE)
        .with_active(false);
    let mut clicked = None;
    let _window_span = crate::frame_trace::span("capture");
    super::super::presentation::Native::Keyboard.show(
        a,
        ctx,
        viewport(),
        builder,
        |a, ctx, class| {
            let _span = crate::frame_trace::span("capture/ui");
            if auditioning(a, s) {
                ctx.request_repaint();
            }
            if class == egui::ViewportClass::Embedded {
                let position = position(a, s);
                let tempo = super::super::patterns::timing::cursor_map(a);
                egui::Window::new(&title)
                    .id(egui::Id::new(PLACE))
                    .show(ctx, |ui| {
                        clicked = rows::list(ui, s, position, tempo.as_ref())
                    });
                return;
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                close(a, s);
                return;
            }
            if let Some(at) = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min) {
                companions::remember(a, PLACE, at);
            }
            super::super::help::observe_recording(a, ctx);
            let input = super::modal::begin(a, ctx);
            if s.first_read(ctx) && !filter::owns_keys(ctx) {
                keys(a, s, &ctx.input(|i| i.events.clone()));
            }
            if s.capture.is_none() {
                ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Focus);
            } else {
                let position = position(a, s);
                let tempo = super::super::patterns::timing::cursor_map(a);
                egui::CentralPanel::default().show(ctx, |ui| {
                    clicked = rows::list(ui, s, position, tempo.as_ref())
                });
            }
            super::modal::end(a, ctx, input);
        },
    );
    match clicked {
        Some(rows::Click::Filter(container)) => filter::set(a, s, container),
        Some(rows::Click::Take(i)) => {
            if let Some(b) = s.capture.as_mut() {
                b.selected = i;
            }
            if let Err(error) = take(a, s) {
                s.error = Some(error);
            }
        }
        Some(rows::Click::Listen(i)) => listen(a, s, i),
        Some(rows::Click::Clear) => {
            if let Err(error) = clear(a, s) {
                s.error = Some(error);
            }
        }
        None => {}
    }
}

#[cfg(test)]
pub(super) fn test_ui(a: &mut App, s: &mut Session, ctx: &egui::Context) {
    if !filter::owns_keys(ctx) {
        keys(a, s, &ctx.input(|i| i.events.clone()));
    }
    let mut clicked = None;
    egui::CentralPanel::default().show(ctx, |ui| {
        let tempo =
            cat_smf_editor::score::audition_timing::TempoMap::constant(s.division, 120.0).unwrap();
        clicked = rows::list(ui, s, None, Some(&tempo));
    });
    if let Some(rows::Click::Filter(id)) = clicked {
        filter::set(a, s, id);
    }
}

#[cfg(test)]
pub(super) fn phrases<'s>(a: &App, s: &'s Session) -> Option<(&'s [Captured], usize, bool)> {
    let sounding = auditioning(a, s);
    s.capture
        .as_ref()
        .map(|b| (b.phrases.as_slice(), b.selected, sounding))
}
