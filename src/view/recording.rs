//! The keyboard windows sit above the editor beside the windows that stay usable.
use super::{
    commands::{self, Command, Screen},
    modal,
    patterns::workspace::Pane,
};
use crate::{app::App, play::audition::live};
use cat_smf_editor::message;
use cat_smf_editor::score::{
    instrument::Patch,
    key_signature::KeySignature,
    pattern::{
        recording::{self, Recorder, Settings},
        Catalog,
    },
    scene::arrangement::PatternOrScene,
};
use eframe::egui::{self, Key, Modifiers};
use std::time::Instant;
mod capture;
mod choices;
mod companions;
mod following;
mod pads;
mod panel;
mod phrase;
mod piano;
mod play;
mod resume;
mod setup;
mod sweep;
mod take;
pub(crate) use following::pattern as follow_pattern;
#[cfg(test)]
mod tests;
pub(crate) mod window;
pub(crate) use capture::advance_app as advance_capture;
#[cfg(test)]
pub(crate) use capture::viewport as capture_viewport;
pub(crate) use companions::reset as reset_windows;
pub(crate) use companions::{piano_viewport, settings_viewport};
pub(crate) use panel::LOOPING;
pub(crate) use resume::begin as resume;
pub(crate) use resume::{close_for_exit, restore};
use take::{finish, reveal};

/// Ticks into pattern `id` that a Phrase loop repeating it plays now.
pub(crate) fn phrase_tick(a: &App, id: u64) -> Option<u64> {
    let s = a.patterns.recording.session.as_ref()?;
    phrase::tick(s, id, a.loaded.as_ref()?.music.division)
}

#[derive(Default)]
pub struct State {
    session: Option<Session>,
    follow: Option<u64>,
    /// The keyboard reopens at startup once the Song window is up.
    resuming: bool,
    /// The song whose keyboard could not open by itself; it is not retried every frame.
    failed: Option<u64>,
}
impl State {
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }
    /// Minimized by its own button; the Song window offers it back.
    pub fn minimized(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.minimized)
    }
    #[cfg(test)]
    pub(crate) fn preparing(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.recorder.is_none())
    }
}
struct Sound {
    container: Option<u64>,
    track: usize,
    patch: Patch,
    name: String,
    /// Its keys and phrase, playing beside the other containers' phrases.
    live: live::Live,
    looper: phrase::Looper,
    /// Full-keyboard kit page, counted upward from the bottom of the editor.
    kit_bank: usize,
    /// Runtime chromatic range, independent of the full-keyboard page.
    kit_base: u8,
}
struct Session {
    /// The window the keyboard opened from; it gets the takes and the focus back.
    origin: egui::ViewportId,
    /// The editor window that owns the keyboard windows, keeping them above it.
    owner: Option<isize>,
    /// Titles of the open windows `owner` already owns.
    owned: Vec<&'static str>,
    division: u16,
    clock: Instant,
    sounds: Vec<Sound>,
    selected: usize,
    /// Used while no instrument plays; the recorder owns a copy while playing.
    settings: Settings,
    /// Loop length, also remembered while the range is free.
    measures: u16,
    /// Selected row of the settings window.
    item: setup::Item,
    settings_open: bool,
    piano_open: bool,
    /// The keyboard came to the front, so its companions follow it there.
    raise: bool,
    /// Of the keyboard and `origin`, the keyboard had the focus last.
    focused: bool,
    minimized: bool,
    /// Where each open companion window opened, fixed while it is open so it can be moved.
    companions: companions::Places,
    /// The pad the mouse holds in each `window::Pointer` window.
    mouse: [Option<recording::Pad>; 2],
    /// Present while playing.
    recorder: Option<Recorder>,
    /// Session time the CC1 sweep started, while the keys are recorded.
    sweep: Option<std::time::Duration>,
    /// The recording voice's metronome, present while the instruments play; its first click
    /// fixes the loop origin.
    click: Option<std::sync::Arc<crate::play::audition::click::Click>>,
    /// The app generation whose patches `sounds` hold.
    generation: u64,
    /// Keys sounded, recording or not, not yet in the song.
    pending: capture::Pending,
    /// Present while the Capture window is open.
    capture: Option<capture::Browser>,
    error: Option<String>,
    /// A transport button clicked while drawing, handled after it.
    clicked: Option<window::Button>,
    new_sound: Option<usize>,
    kit_creation: Option<choices::KitCreation>,
    /// The last frame of each window whose keys were read.
    handled: Vec<(egui::ViewportId, u64)>,
    /// Where the keyboard window opens on the screen.
    position: egui::Pos2,
    /// Reads a Win32 virtual key; replaced by tests.
    down: fn(u8) -> bool,
}
impl Session {
    fn sound(&self) -> &Sound {
        &self.sounds[self.selected]
    }
    /// The keys of the playing container, while the instruments play.
    fn queue(&self) -> Option<&live::Queue> {
        self.click.as_ref().map(|_| &self.sound().live.queue)
    }
    fn settings(&self) -> &Settings {
        self.recorder
            .as_ref()
            .map_or(&self.settings, Recorder::settings)
    }
    fn settings_mut(&mut self) -> &mut Settings {
        self.recorder
            .as_mut()
            .map_or(&mut self.settings, Recorder::settings_mut)
    }
    /// `false` when this frame of the window already read its keys.
    fn first_read(&mut self, ctx: &egui::Context) -> bool {
        let token = modal::frame_token(ctx);
        let fresh = !self.handled.contains(&token);
        if fresh {
            self.handled.retain(|(id, _)| *id != token.0);
            self.handled.push(token);
        }
        fresh
    }
}

#[path = "recording/entry_commands.rs"]
mod entry_commands;
pub(crate) use entry_commands::collect;

/// Moves the session clock as if `by` had passed.
#[cfg(test)]
pub(crate) fn rewind(a: &mut App, by: std::time::Duration) {
    a.patterns.recording.session.as_mut().unwrap().clock -= by;
}

#[cfg(test)]
pub(crate) fn sounding(a: &App, pitch: u8) -> bool {
    a.patterns
        .recording
        .session
        .as_ref()
        .is_some_and(|s| s.recorder.as_ref().is_some_and(|r| r.sounding(pitch)))
}

/// Plays one audio block of the recording voice's metronome, as the output stream would.
#[cfg(test)]
pub(crate) fn render_click(a: &mut App) {
    let s = a.patterns.recording.session.as_mut().unwrap();
    s.click
        .as_ref()
        .unwrap()
        .render(&mut [0.0; 2048], 2, 48000, Instant::now());
    play::follow_click(s);
}

mod opening;
use opening::{open, open_from};
mod kit;

/// The keyboard or one of its companion windows. While they draw, the session is out of `App`.
pub(crate) fn owns(viewport: egui::ViewportId) -> bool {
    [
        window::viewport(),
        settings_viewport(),
        piano_viewport(),
        capture::viewport(),
    ]
    .contains(&viewport)
}

/// Shows the keyboard windows; the other windows keep their own input.
pub(crate) fn show(a: &mut App, ctx: &egui::Context) {
    resume::follow(a, ctx);
    following::apply(a);
    let mut state = std::mem::take(&mut a.patterns.recording);
    let Some(s) = state.session.as_mut() else {
        a.patterns.recording = state;
        return;
    };
    let visible = super::presentation::Native::Keyboard.visible(a, ctx, window::viewport());
    let suspended_done = if !visible {
        play::suspend(s);
        play::advance(a, s)
    } else {
        false
    };
    let builder = egui::ViewportBuilder::default()
        .with_title(window::TITLE)
        .with_position(s.position)
        .with_inner_size(window::SIZE);
    let mut moved = None;
    let song = a.song_id;
    let window_span = crate::frame_trace::span("keyboard");
    let done = super::presentation::Native::Keyboard.show(
        a,
        ctx,
        window::viewport(),
        builder,
        |a, ctx, class| {
            let _span = crate::frame_trace::span("keyboard/ui");
            let native = class != egui::ViewportClass::Embedded;
            if native {
                moved = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min);
                s.minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
                if ctx.input(|i| i.viewport().close_requested()) {
                    if play::kept(a) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    }
                    return play::end(a, s, s.clock.elapsed());
                }
            }
            super::help::observe_recording(a, ctx);
            // Embedded, this shares the root input whose modal already ran; a second one eats the palette's keys.
            let input = native.then(|| modal::begin(a, ctx)).flatten();
            let done = (s.first_read(ctx) && play::process(a, ctx, s)) || {
                window::show(ctx, class, s);
                match s.clicked.take() {
                    Some(window::Button::Browse) => {
                        phrase::browse(a, ctx, s);
                        false
                    }
                    b => b.is_some_and(|b| play::button(a, s, b)),
                }
            };
            if native {
                modal::end(a, ctx, input);
            }
            done
        },
    );
    drop(window_span);
    if let Some(at) = moved {
        companions::remember(a, companions::KEYBOARD, at);
    }
    if ctx.input_for(window::viewport(), |i| i.focused) {
        s.focused = true;
    } else if ctx.input_for(s.origin, |i| i.focused) {
        s.focused = false;
    }
    let done = suspended_done || done.unwrap_or(false);
    if !done {
        if let Err(error) = capture::flush(a, s, false) {
            s.error = Some(error);
        }
        let keyboard = moved.unwrap_or(s.position);
        companions::show(a, ctx, s, keyboard);
        capture::show(a, ctx, s, keyboard);
        companions::adopt(ctx, s);
    }
    if done {
        // A closing that commits a take has put the keys in the song already.
        if a.song_id == song {
            if let Err(error) = capture::flush(a, s, true) {
                a.load_error = Some(error);
            }
            // A take reveals its pattern instead.
            capture::close(a, s);
        }
        ctx.send_viewport_cmd_to(s.origin, egui::ViewportCommand::Focus);
        // Only closing the app leaves the switch on for the next start.
        a.config.select_phrase_loop(false);
        resume::shut(a, &mut state);
    }
    a.patterns.recording = state;
    ctx.request_repaint_after(std::time::Duration::from_millis(8));
}
