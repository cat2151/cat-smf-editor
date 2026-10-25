//! Command palette `Track Overview`: every track's piano roll stacked, for playing or jumping to any track's measure.
use crate::app::{App, Loaded};
use crate::view::commands::{Command, Screen};
use cat_smf_editor::score::{musical_time::Position, note_view::note_on};
use eframe::egui::{self, Key};

mod curves;
mod window;
pub(crate) use window::show;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// Keyboard cell: track and the tick where its measure begins.
    cell: (usize, u64),
    /// Scrolls the keyboard cell into view on the next frame.
    reveal: bool,
    /// Track and measure the open context menu acts on.
    menu: Option<(usize, u64)>,
    viewport: crate::view::piano_roll::viewport::Viewport,
    repeat_drag: crate::view::timeline_repeat::State,
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

pub(crate) fn kind(loaded: &Loaded, track: usize) -> Kind {
    let events = &loaded.music.tracks[track].events;
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
    app.track_overview.open = false;
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.parameter_editor.close();
    crate::cursor::split::go(app, track, tick);
}

pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen == Screen::Main && app.loaded.is_some() {
        out.push(Command::plain(
            screen,
            "track-overview",
            "Track Overview",
            &[],
            |a, _| open(a),
        ));
    }
    if screen != Screen::Overview {
        return;
    }
    let mut add = |id, name, keys: &[Key], repeat, run: fn(&mut App, &egui::Context)| {
        let mut command = Command::plain(screen, id, name, keys, run);
        command.repeat = repeat;
        out.push(command);
    };
    add(
        "overview-close",
        "閉じる",
        &[Key::Escape],
        false,
        |a, _| a.track_overview.open = false,
    );
    add("help", "ヘルプ", &[Key::F1], false, |a, _| {
        a.help_open = !a.help_open
    });
    add(
        "overview-measure-next",
        "次の小節",
        &[Key::L, Key::ArrowRight],
        true,
        |a, _| step_measure(a, true),
    );
    add(
        "overview-measure-prev",
        "前の小節",
        &[Key::H, Key::ArrowLeft],
        true,
        |a, _| step_measure(a, false),
    );
    add(
        "overview-track-next",
        "次のtrack",
        &[Key::J, Key::ArrowDown],
        true,
        |a, _| step_track(a, true),
    );
    add(
        "overview-track-prev",
        "前のtrack",
        &[Key::K, Key::ArrowUp],
        true,
        |a, _| step_track(a, false),
    );
    add(
        "overview-play",
        "全trackをこの小節から演奏",
        &[Key::Enter],
        false,
        |a, _| {
            let (track, tick) = a.track_overview.cell;
            play(a, track, tick);
        },
    );
    add(
        "overview-track-play",
        "track play",
        &[Key::T],
        false,
        |a, _| {
            let (track, tick) = a.track_overview.cell;
            play_track(a, track, tick);
        },
    );
    add("overview-jump", "jump", &[Key::G], false, |a, _| {
        let (track, tick) = a.track_overview.cell;
        jump(a, track, tick);
    });
    add("play-menu", "再生menu", &[Key::P], false, |a, _| {
        focus_cell(a);
        a.playback.open = true;
    });
    add(
        "play-last",
        "前回の方法で再生",
        &[Key::Space],
        false,
        |a, _| {
            focus_cell(a);
            crate::play::playback::start(a, a.playback.last);
        },
    );
    add("follow", "再生位置追従", &[Key::F], true, |a, _| {
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
