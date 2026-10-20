//! The palette's one-line lyric input for one measure: the measure's lyrics are replaced as they
//! are typed (`edit::lyric::measure`).
use crate::app::App;
use cat_smf_editor::model::Music;
use eframe::egui;

pub struct Editing {
    song: u64,
    generation: u64,
    track: usize,
    /// The cursor row's lyric cell, captured while the score draws.
    pub(crate) rect: Option<egui::Rect>,
    pub(crate) text: String,
    error: Option<String>,
    focus: bool,
    pub(crate) replacing: Replacing,
    pub(super) combined: bool,
    repeat_active: bool,
    repeat_seen: bool,
    /// The IME holds unconverted text, so Enter converts it instead of confirming.
    composing: bool,
}

/// The measure whose lyrics follow the input, and the song as it was when the input opened.
pub(crate) struct Replacing {
    pub(crate) range: (u64, u64),
    base: Music,
}

/// The cursor's measure loops, and the input replaces its lyrics.
pub(crate) fn begin(app: &mut App) {
    let Some(range) = crate::view::mml_input::measure::range(app) else {
        return;
    };
    crate::cursor::moves::at_tick(app, range.0);
    crate::play::transport::repeat::start(app, range);
    let base = app.loaded.as_ref().unwrap().music.clone();
    app.voicevox.lyrics = Some(Editing {
        song: app.song_id,
        generation: app.generation,
        track: app.track,
        rect: None,
        text: String::new(),
        error: None,
        focus: true,
        replacing: Replacing { range, base },
        combined: false,
        repeat_active: true,
        repeat_seen: app.playback.repeat.is_some(),
        composing: false,
    });
}

/// The opening song with the measure's lyrics replaced by `editing.text`.
fn replaced(editing: &Editing, replacing: &Replacing) -> Result<Music, String> {
    let mut music = replacing.base.clone();
    let text = if editing.combined {
        super::combined::replace(&mut music, editing.track, replacing.range, &editing.text)?
    } else {
        &editing.text
    };
    cat_smf_editor::edit::lyric::measure::replace(
        &mut music,
        editing.track,
        replacing.range,
        text,
    )?;
    cat_smf_editor::edit::regenerate(&mut music)?;
    Ok(music)
}

/// Shows `music` unless it is already shown; true when the song changed.
fn install(app: &mut App, music: Music) -> Result<bool, String> {
    let current = &app.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    let encode = cat_smf_editor::smf_io::encode;
    if encode(current)? == encode(&music)? {
        return Ok(false);
    }
    app.apply_music(music)?;
    Ok(true)
}

/// Each change of the measure input is heard in the loop, and the singer starts on it.
fn preview(app: &mut App, editing: &mut Editing) {
    editing.error = None;
    let result = replaced(editing, &editing.replacing).and_then(|music| install(app, music));
    if editing.combined && result.is_err() {
        let _ = install(app, editing.replacing.base.clone());
    }
    editing.generation = app.generation;
    match result {
        Ok(true) => {
            crate::play::sung::prerender::ahead(app, editing.track);
            if editing.combined
                && editing.repeat_active
                && app.playback.repeat != Some(editing.replacing.range)
            {
                crate::play::transport::repeat::start(app, editing.replacing.range);
            }
        }
        Ok(false) => {}
        Err(error) => editing.error = Some(error),
    }
}

/// The score calls this with the cursor row's lyric cell; the input stays where it first appeared.
pub(crate) fn place(app: &mut App, rect: egui::Rect) {
    if let Some(editing) = &mut app.voicevox.lyrics {
        editing.rect.get_or_insert(rect);
    }
}

pub(crate) fn cancel(app: &mut App) {
    let Some(editing) = app.voicevox.lyrics.take() else {
        return;
    };
    if let Err(error) = install(app, editing.replacing.base) {
        app.voicevox.error = Some(error);
    }
}

/// The measure input's song becomes one undo step from the opening song.
fn confirm_measure(app: &mut App, editing: &Editing) -> Result<(), String> {
    let replacing = &editing.replacing;
    let music = replaced(editing, replacing)?;
    let encode = cat_smf_editor::smf_io::encode;
    if encode(&music)? == encode(&replacing.base)? {
        return install(app, music).map(drop);
    }
    install(app, music)?;
    app.undo.push(replacing.base.clone());
    Ok(())
}

pub(crate) fn confirm(app: &mut App, _: &egui::Context) {
    let Some(mut editing) = app.voicevox.lyrics.take() else {
        return;
    };
    match confirm_measure(app, &editing) {
        Ok(()) => {
            crate::play::sung::prerender::ahead(app, editing.track);
            // The measure keeps looping while the cursor waits on the next one.
            crate::cursor::moves::at_tick(app, editing.replacing.range.1);
        }
        Err(error) => {
            editing.error = Some(error);
            editing.focus = true;
            app.voicevox.lyrics = Some(editing);
        }
    }
}

/// Removes this frame's Enter when it reaches the IME: while text is unconverted, or with the
/// conversion it commits. Commands run after this, once per frame.
pub(crate) fn keep_ime_enter(app: &mut App, ctx: &egui::Context) {
    use egui::{Event, ImeEvent};
    let Some(Editing { composing, .. }) = &mut app.voicevox.lyrics else {
        return;
    };
    let events = ctx.input(|i| i.events.clone());
    let mut ime = *composing;
    for event in &events {
        match event {
            Event::Ime(ImeEvent::Preedit(text)) => {
                *composing = !text.is_empty();
                ime |= *composing;
            }
            Event::Ime(ImeEvent::Commit(_)) => {
                *composing = false;
                ime = true;
            }
            Event::Ime(ImeEvent::Disabled) => *composing = false,
            _ => {}
        }
    }
    if ime {
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: egui::Key::Enter,
                        ..
                    }
                )
            })
        });
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.voicevox.lyrics.is_some() {
        crate::view::commands::route(app, ctx);
    }
    let Some(mut editing) = app.voicevox.lyrics.take() else {
        return;
    };
    if editing.song != app.song_id || editing.generation != app.generation {
        return;
    }
    if app.playback.repeat.is_some() {
        editing.repeat_seen = true;
    } else if editing.repeat_seen && !crate::play::transport::timeline::running(app) {
        editing.repeat_active = false;
    }
    let mut area = egui::Area::new(egui::Id::new("voicevox-lyrics")).order(egui::Order::Foreground);
    area = match editing.rect {
        Some(rect) => area.fixed_pos(rect.min),
        None => area.anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO),
    };
    let mut close = false;
    area.show(ctx, |ui| {
        let mut edit =
            egui::TextEdit::singleline(&mut editing.text).font(egui::TextStyle::Monospace);
        if let Some(rect) = editing.rect {
            edit = edit.desired_width(rect.width().max(240.0));
        }
        let response = ui.add(edit);
        if editing.focus {
            response.request_focus();
            editing.focus = false;
        }
        if response.changed() {
            preview(app, &mut editing);
        }
        if response.lost_focus() && !app.palette.is_open() {
            close = true;
        }
        if let Some(error) = &editing.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    });
    app.voicevox.lyrics = Some(editing);
    if close {
        cancel(app);
    }
}
