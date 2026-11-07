//! Stacked track rolls, or Song placements grouped by sound container when patterns exist.
use crate::app::{App, Loaded};
use crate::view::commands::{Command, Screen};
use cat_smf_editor::model::Event;
use cat_smf_editor::score::{musical_time::Position, note_view::note_on};
use eframe::egui::{self, Key};

mod curves;
mod drawing;
mod panes;
mod song;
pub(crate) mod song_playback;
mod window;
pub(crate) use window::show;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// Keyboard cell: displayed row and the tick where its measure begins.
    cell: (usize, u64),
    /// Scrolls the keyboard cell into view on the next frame.
    reveal: bool,
    /// Displayed row and measure the open context menu acts on.
    menu: Option<(usize, u64)>,
    viewport: crate::view::timeline_controls::Viewport,
    repeat_drag: crate::view::timeline_repeat::State,
    solo: bool,
    cache: std::cell::RefCell<Option<song::Cache>>,
}

/// What a track holds, shown as its pane background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Notes,
    /// Events but no notes, such as the conductor track.
    Events,
    /// Nothing but End of Track.
    Empty,
}

pub(crate) fn kind(events: &[Event]) -> Kind {
    if events.iter().any(note_on) {
        Kind::Notes
    } else if events
        .iter()
        .any(|e| !(e.status == 0xff && e.meta_type == 0x2f))
    {
        Kind::Events
    } else {
        Kind::Empty
    }
}

pub(crate) fn open(app: &mut App) {
    if song::enabled(app) {
        match song::Song::read(app) {
            Ok(song) => {
                let row = song
                    .rows
                    .iter()
                    .position(|r| r.track == app.track)
                    .unwrap_or(0);
                let tick = song_playback::playing(app)
                    .unwrap_or_else(|| crate::view::patterns::timing::cursor(app).unwrap_or(0));
                app.track_overview = State {
                    open: true,
                    cell: (row, song.measure_start(tick.min(song.end))),
                    reveal: true,
                    ..Default::default()
                };
            }
            Err(error) => app.load_error = Some(error),
        }
        return;
    }
    let tick = crate::play::playback::location(app).0;
    let Some(loaded) = &app.loaded else {
        return;
    };
    let start = measure_start(loaded, app.track, tick);
    app.track_overview = State {
        open: true,
        cell: (app.track, start),
        reveal: true,
        ..Default::default()
    };
}

pub(crate) fn is_song(app: &App) -> bool {
    app.track_overview.open && song::enabled(app)
}

/// Where the measure holding `tick` begins in `track`'s meter.
pub(crate) fn measure_start(loaded: &Loaded, track: usize, tick: u64) -> u64 {
    loaded
        .time
        .map(track)
        .and_then(|map| map.tick_at(&measure(map.position(tick).measure)))
        .unwrap_or(0)
}

fn measure(measure: u128) -> Position {
    Position {
        measure,
        beat: 1,
        tick_numerator: 0,
        tick_denominator: 1,
    }
}

/// The next or previous measure start, not past the song's last event.
fn step_measure(app: &mut App, forward: bool) {
    if song::enabled(app) {
        if let Ok(song) = song::Song::read(app) {
            let tick = app.track_overview.cell.1;
            let map = song.time.map(0).unwrap();
            let current = map.position(tick).measure;
            let target = if forward {
                current + 1
            } else {
                current.saturating_sub(1).max(1)
            };
            if let Some(next) = map.tick_at(&measure(target))
                && next < song.end.max(1)
            {
                app.track_overview.cell.1 = next;
                app.track_overview.reveal = true;
            }
        }
        return;
    }
    let Some(loaded) = &app.loaded else {
        return;
    };
    let (track, tick) = app.track_overview.cell;
    let Some(map) = loaded.time.map(track) else {
        return;
    };
    let current = map.position(tick).measure;
    let target = if forward {
        current + 1
    } else {
        current.saturating_sub(1).max(1)
    };
    let last = loaded
        .music
        .tracks
        .iter()
        .filter_map(|t| t.events.last())
        .map(|e| e.tick)
        .max()
        .unwrap_or(0);
    if let Some(next) = map.tick_at(&measure(target))
        && (next <= last || !forward)
    {
        app.track_overview.cell.1 = next;
        app.track_overview.reveal = true;
    }
}

fn step_track(app: &mut App, forward: bool) {
    if song::enabled(app) {
        if let Ok(song) = song::Song::read(app) {
            let row = app.track_overview.cell.0;
            app.track_overview.cell.0 = if forward {
                (row + 1).min(song.rows.len() - 1)
            } else {
                row.saturating_sub(1)
            };
            app.track_overview.reveal = true;
        }
        return;
    }
    let Some(loaded) = &app.loaded else {
        return;
    };
    let (track, tick) = app.track_overview.cell;
    let next = if forward {
        (track + 1).min(loaded.music.tracks.len() - 1)
    } else {
        track.saturating_sub(1)
    };
    app.track_overview.cell = (next, measure_start(loaded, next, tick));
    app.track_overview.reveal = true;
}

/// Every track from the measure to the song end.
pub(crate) fn play(app: &mut App, track: usize, tick: u64) {
    if song::enabled(app) {
        song_playback::play(app, track, tick, false);
        return;
    }
    let tracks: Vec<_> = (0..app.loaded.as_ref().map_or(0, |l| l.music.tracks.len())).collect();
    app.track_overview.cell = (track, tick);
    crate::play::transport::timeline::play_to_end(
        app,
        track,
        tick,
        &tracks,
        crate::play::playback::Mode::AllTracksFromMeasure,
    );
}

/// `track` alone from the measure to its end.
pub(crate) fn play_track(app: &mut App, track: usize, tick: u64) {
    if song::enabled(app) {
        song_playback::play(app, track, tick, true);
        return;
    }
    app.track_overview.cell = (track, tick);
    crate::play::transport::timeline::play_to_end(
        app,
        track,
        tick,
        &[track],
        crate::play::playback::Mode::MeasureStart,
    );
}

/// Closes the overview and puts the event list cursor on the measure.
pub(crate) fn jump(app: &mut App, track: usize, tick: u64) {
    if song::enabled(app) {
        song_playback::jump(app, track, tick);
        return;
    }
    app.track_overview.open = false;
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.parameter_editor.close();
    crate::cursor::split::go(app, track, tick);
}

pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen == Screen::Main && app.loaded.is_some() {
        out.push(Command::plain(screen, "overview.open", &[], |a, _| open(a)));
    }
    if screen != Screen::Overview {
        return;
    }
    let mut add = |id, keys: &[Key], repeat, run: fn(&mut App, &egui::Context)| {
        let mut command = Command::plain(screen, id, keys, run);
        command.repeat = repeat;
        out.push(command);
    };
    add("overview.close", &[Key::Escape], false, |a, _| {
        a.track_overview.open = false
    });
    add("app.help", &[Key::F1], false, |a, _| {
        crate::view::help::summon(a)
    });
    add(
        "overview.measure-next",
        &[Key::L, Key::ArrowRight],
        true,
        |a, _| step_measure(a, true),
    );
    add(
        "overview.measure-prev",
        &[Key::H, Key::ArrowLeft],
        true,
        |a, _| step_measure(a, false),
    );
    add(
        "overview.track-next",
        &[Key::J, Key::ArrowDown],
        true,
        |a, _| step_track(a, true),
    );
    add(
        "overview.track-prev",
        &[Key::K, Key::ArrowUp],
        true,
        |a, _| step_track(a, false),
    );
    add(
        if song::enabled(app) {
            "overview.play-song"
        } else {
            "overview.play-all"
        },
        &[Key::Enter],
        false,
        |a, _| {
            let (track, tick) = a.track_overview.cell;
            play(a, track, tick);
        },
    );
    add(
        if song::enabled(app) {
            "overview.play-container"
        } else {
            "overview.play-track"
        },
        &[Key::T],
        false,
        |a, _| {
            let (track, tick) = a.track_overview.cell;
            play_track(a, track, tick);
        },
    );
    add("overview.jump", &[Key::G], false, |a, _| {
        let (track, tick) = a.track_overview.cell;
        jump(a, track, tick);
    });
    add("play.open", &[Key::P], false, |a, ctx| {
        a.playback.viewport = ctx.viewport_id();
        if !song::enabled(a) {
            focus_cell(a);
        }
        a.playback.open = true;
    });
    add("play.last", &[Key::Space], false, |a, _| {
        if song::enabled(a) {
            let (row, tick) = a.track_overview.cell;
            song_playback::play(a, row, tick, a.track_overview.solo);
            return;
        }
        focus_cell(a);
        crate::play::playback::start(a, a.playback.last);
    });
    add("play.follow", &[Key::F], true, |a, _| {
        a.playback.follow = !a.playback.follow
    });
}

/// The play menu plays from the current track and cursor, so they follow the keyboard cell.
fn focus_cell(app: &mut App) {
    let (track, tick) = app.track_overview.cell;
    crate::cursor::split::go(app, track, tick);
}

#[cfg(test)]
mod tests;
