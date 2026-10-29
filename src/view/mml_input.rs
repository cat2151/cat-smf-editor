//! Note insertion overlay for MML and chord progressions. Input changes paint a preparation message before
//! updating the song and opening plugins for cursor audition or A–B playback.
use crate::app::App;
use cat_smf_editor::{
    mml::{Action, CursorPreview},
    model::Music,
};
use eframe::egui;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub buffer: String,
    confirm_ready: bool,
    command: Option<Action>,
    pending_preview: Option<(u64, usize, bool)>,
    preview_cursor: Option<usize>,
    error: Option<String>,
    focus: bool,
    preview: CursorPreview,
    song: u64,
    track: usize,
    tick: u64,
    channel: u8,
    key: Option<String>,
    /// The song as it was when the overlay opened; previews are rebuilt from it.
    base: Option<Music>,
    saved_base: Option<Music>,
    pattern: Option<cat_smf_editor::score::pattern::editing::Context>,
    session: Option<crate::store::config::Session>,
    cursor_tick: Option<u64>,
    /// A repeat editing session never replaces transport audio with cursor audition.
    repeat_session: bool,
    /// The measure whose notes the phrase overwrites; without one, it pushes them later.
    pub(crate) measure: Option<(u64, u64)>,
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

pub fn open(app: &mut App, ctx: &egui::Context, overwrite: bool) {
    let Some(loaded) = app.loaded.as_ref() else {
        return;
    };
    if !app.can_edit_notes(app.track) {
        app.load_error = Some(
            if cat_smf_editor::score::chord_guide::track(&loaded.music) == Some(app.track) {
                "Select a note track".into()
            } else {
                "Edit the pattern from its destination track".into()
            },
        );
        return;
    }
    let (tick, _) = crate::play::playback::location(app);
    let channel =
        cat_smf_editor::edit::mml::channel(&loaded.visible().tracks[app.track], app.selected_event);
    app.mml_input = State {
        open: true,
        focus: true,
        song: app.song_id,
        track: app.track,
        tick,
        channel,
        key: loaded
            .keys
            .map(app.track)
            .and_then(|keys| keys.at(tick))
            .map(|key| {
                let name = key.name();
                let tonic = name.split_whitespace().next().unwrap();
                format!("{tonic}{}", if key.minor() { " Aeolian" } else { "" })
            }),
        base: Some(loaded.visible().clone()),
        saved_base: Some(loaded.music.clone()),
        pattern: app.patterns.editing.clone(),
        session: app.session(),
        cursor_tick: app.track_view.cursor_tick,
        repeat_session: app.playback.repeat.is_some()
            && crate::play::transport::timeline::running(app),
        measure: overwrite.then(|| app.mml_input.measure.take()).flatten(),
        ..Default::default()
    };
    if !app.mml_input.repeat_session {
        crate::play::transport::stop(app);
    }
    // The text event may arrive with the key or one pass later.
    swallow_typed_i(ctx);
}

fn swallow_typed_i(ctx: &egui::Context) {
    ctx.input_mut(|i| {
        i.events
            .retain(|e| !matches!(e, egui::Event::Text(t) if t == "i" || t == "I"))
    });
}

/// Called only after the waiting overlay has had a frame to paint.
pub fn ready_to_confirm(app: &mut App) {
    app.mml_input.confirm_ready = true;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.mml_input.open {
        return;
    }
    crate::play::transport::stop_shortcut(app, ctx);
    if super::busy::pending(app) {
        super::comma_menu::swallow(ctx);
        return;
    }
    let mut state = std::mem::take(&mut app.mml_input);
    if state.song != app.song_id
        || state.track != app.track
        || app.loaded.is_none()
        || state.pattern != app.patterns.editing
    {
        if !state.repeat_session {
            app.audition.stop();
        }
        return;
    }
    const RATE: u32 = 48000;
    if let Some((pass, mut cursor, changed)) = state.pending_preview
        && ctx.cumulative_pass_nr() > pass
        && ctx.output(|o| o.num_completed_passes == 0)
    {
        state.pending_preview = None;
        if changed {
            if let Some(prefixed) = key_prefix::apply(ctx, &mut state.buffer, state.key.as_deref())
            {
                cursor = prefixed;
            }
            preview(app, &mut state);
        }
        let request = state.preview.refresh(&state.buffer, cursor, RATE, changed);
        audition(app, &state, request);
        ctx.request_repaint();
    }
    let ready = std::mem::take(&mut state.confirm_ready);
    let action = if ready {
        ctx.request_repaint();
        Action::Confirm
    } else {
        editor::show(ctx, &mut state, app)
    };
    if matches!(action, Action::Confirm) && !ready {
        app.mml_input = state;
        super::busy::request(app, ctx, super::busy::Task::MmlConfirm);
        super::comma_menu::swallow(ctx);
        return;
    }
    match action {
        Action::Cancel => {
            if !state.repeat_session {
                app.audition.stop();
            }
            match app.apply_music(
                state
                    .saved_base
                    .as_ref()
                    .unwrap_or_else(|| state.base.as_ref().unwrap())
                    .clone(),
            ) {
                Ok(()) => {
                    if let Some(session) = &state.session {
                        app.restore_cursor(session);
                    }
                    app.track_view.cursor_tick = state.cursor_tick;
                    state.open = false;
                }
                Err(error) => state.error = Some(error),
            }
        }
        Action::Confirm => {
            if !state.repeat_session {
                app.audition.stop();
            }
            match inserted(&state) {
                Ok((music, end)) => {
                    if let Err(error) = insertion::apply(app, &state, music) {
                        state.error = Some(error);
                        app.mml_input = state;
                        super::comma_menu::swallow(ctx);
                        return;
                    }
                    app.undo.push(
                        state
                            .saved_base
                            .as_ref()
                            .unwrap_or_else(|| state.base.as_ref().unwrap())
                            .clone(),
                    );
                    state.open = false;
                    if !state.repeat_session {
                        crate::play::transport::repeat::play_once(app, (state.tick, end));
                        app.playback.follow = false;
                    }
                    let cursor = if state
                        .pattern
                        .as_ref()
                        .and_then(|p| p.edit_range(state.saved_base.as_ref()?).ok())
                        .is_some_and(|r| end >= r.1)
                    {
                        state.tick
                    } else {
                        end
                    };
                    crate::cursor::moves::at_tick(app, cursor);
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

fn audition(app: &mut App, state: &State, request: Option<Option<cat_smf_editor::mml::Notes>>) {
    if state.repeat_session {
        return;
    }
    const RATE: u32 = 48000;
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
            if let Some((track, patch)) = super::patterns::preview::edit_sound(app, state.track) {
                app.audition.play_track(track, &patch, &notes, None);
                return;
            }
            // A singer sings only the top note, as in playback.
            let take = notes.last().and_then(|&(pitch, _, seconds)| {
                crate::play::sung::audition::note(app, state.track, pitch, seconds?)
            });
            let tick = crate::play::playback::location(app).0;
            if !crate::play::automation::audition_notes(
                app,
                state.track,
                tick,
                &notes,
                take.clone(),
            ) {
                let patch = &app.loaded.as_ref().unwrap().patches[state.track];
                app.audition.play_track(state.track, patch, &notes, take);
            }
        } else {
            app.audition.stop();
        }
    }
}

/// Unusable input restores the opening snapshot, including its pattern length.
fn preview(app: &mut App, state: &mut State) {
    let fallback = if state.pattern.is_some() {
        state.saved_base.as_ref()
    } else {
        state.base.as_ref()
    };
    let Some(base) = fallback else { return };
    let music = inserted(state).map_or_else(|_| base.clone(), |(music, _)| music);
    let current = &app.loaded.as_ref().unwrap().music;
    if cat_smf_editor::smf_io::encode(current) == cat_smf_editor::smf_io::encode(&music) {
        return;
    }
    match insertion::apply(app, state, music) {
        Ok(()) => measure::follow(app, state),
        Err(error) => state.error = Some(error),
    }
}

use insertion::inserted;
pub(crate) mod commands;
mod editor;
mod insertion;
mod key_prefix;
pub(crate) mod measure;
#[cfg(test)]
mod tests;
