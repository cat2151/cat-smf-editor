//! Note insertion overlay. The shared player widget owns text editing; every
//! input change is applied to the song so all views follow it.
use crate::app::App;
use cat_smf_editor::{
    mml::{self, Action, CursorPreview},
    model::Music,
};
use eframe::egui;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub buffer: String,
    error: Option<String>,
    focus: bool,
    preview: CursorPreview,
    song: u64,
    track: usize,
    tick: u64,
    channel: u8,
    /// The song as it was when the overlay opened; previews are rebuilt from it.
    base: Option<Music>,
    /// One undo step restores `base`, however many previews were applied.
    recorded: bool,
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if app.mml_input.open
        || super::busy::pending(app)
        || app.loaded.is_none()
        || ctx.wants_keyboard_input()
        || app.help_open
        || app.comma_menu.open
        || app.edit_menu.open
        || app.note_edit.is_some()
        || app.trill_edit.is_some()
        || app.layer_editor.open
        || app.effect_editor.is_open()
        || app.track_view.go_open
        || app.track_view.split_open
        || app.playback.open
        || app.playback.mixer_focused()
        || app.player_selection.open
    {
        return;
    }
    #[cfg(windows)]
    if crate::player::resident_link::loading(app) {
        return;
    }
    if !super::consume_plain(ctx, egui::Key::I) {
        return;
    }
    swallow_typed_i(ctx);
    super::busy::request(app, ctx, super::busy::Task::MmlInput);
}

pub fn open(app: &mut App, ctx: &egui::Context) {
    let Some(loaded) = app.loaded.as_ref() else {
        return;
    };
    let (tick, _) = crate::play::playback::location(app);
    let channel =
        cat_smf_editor::edit::mml::channel(&loaded.music.tracks[app.track], app.selected_event);
    app.mml_input = State {
        open: true,
        focus: true,
        song: app.song_id,
        track: app.track,
        tick,
        channel,
        base: Some(loaded.music.clone()),
        ..Default::default()
    };
    crate::play::transport::stop(app);
    // The text event may arrive with the key or one pass later.
    swallow_typed_i(ctx);
}

fn swallow_typed_i(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events
            .retain(|e| !matches!(e, egui::Event::Text(t) if t == "i" || t == "I"))
    });
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.mml_input.open {
        return;
    }
    let mut state = std::mem::take(&mut app.mml_input);
    if state.song != app.song_id || state.track != app.track || app.loaded.is_none() {
        app.audition.stop();
        return;
    }
    const RATE: u32 = 48000;
    let typed = state.buffer.clone();
    let (action, request) = mml::editor(
        ctx,
        &mut state.buffer,
        &mut state.focus,
        &mut state.preview,
        state.error.as_deref(),
        RATE,
        true,
    );
    if state.buffer != typed {
        preview(app, &mut state);
    }
    if let Some(notes) = request {
        if let Some(notes) = notes {
            let notes: Vec<_> = notes
                .pitches
                .iter()
                .enumerate()
                .filter(|(_, n)| **n != 0)
                .map(|(pitch, &n)| {
                    (
                        pitch as u8,
                        n as u8,
                        Some((n >> 8) as f64 / f64::from(RATE)),
                    )
                })
                .collect();
            let patch = &app.loaded.as_ref().unwrap().patches[state.track];
            app.audition.play_track(state.track, patch, &notes);
        } else {
            app.audition.stop();
        }
    }
    match action {
        Action::Cancel => {
            app.audition.stop();
            state.open = false;
        }
        Action::Confirm => {
            app.audition.stop();
            match inserted(&state) {
                Ok((_, end)) => {
                    state.open = false;
                    crate::play::transport::repeat::play_once(app, (state.tick, end));
                }
                Err(error) => {
                    state.error = Some(error);
                    state.focus = true;
                }
            }
        }
        Action::None => {}
    }
    app.mml_input = state;
    // Text and navigation belong to the modal, even on its closing frame.
    super::comma_menu::swallow(ctx);
}

/// The song with the current input inserted into the overlay's opening state,
/// and the tick where the insertion ends.
fn inserted(state: &State) -> Result<(Music, u64), String> {
    let mut music = state.base.clone().ok_or("No SMF is loaded")?;
    let end = cat_smf_editor::edit::mml::insert(
        &mut music,
        state.track,
        state.tick,
        state.channel,
        &state.buffer,
    )?;
    cat_smf_editor::edit::regenerate(&mut music)?;
    Ok((music, end))
}

/// Unusable input shows the opening state, so deleting the text undoes the preview.
fn preview(app: &mut App, state: &mut State) {
    let Some(base) = &state.base else {
        return;
    };
    let music = inserted(state).map_or_else(|_| base.clone(), |(music, _)| music);
    let current = &app.loaded.as_ref().unwrap().music;
    if cat_smf_editor::smf_io::encode(current) == cat_smf_editor::smf_io::encode(&music) {
        return;
    }
    if let Err(error) = app.apply_music(music) {
        state.error = Some(error);
        return;
    }
    if !state.recorded {
        app.undo.push(base.clone());
        state.recorded = true;
    }
}

#[cfg(test)]
mod tests;
