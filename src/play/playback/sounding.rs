//! The events a track plays, shared by playback and the views that draw the performance.
use super::Mode;
use crate::app::{App, Loaded};
use cat_smf_editor::{
    model::{Event, Music},
    score::note_view::NoteView,
};
use std::borrow::Cow;

/// One track's events as they sound, with the length of each Note On.
#[derive(Default)]
pub(crate) struct Sounding<'a> {
    pub events: Cow<'a, [Event]>,
    /// Indexed like `events`.
    pub durations: Cow<'a, [Option<u64>]>,
    /// Last tick of the arrangement the events come from.
    pub end: u64,
}

/// A pattern source plays only while it is edited on its own.
pub(crate) fn silent(app: &App, track: usize) -> bool {
    app.loaded.as_ref().is_some_and(|loaded| {
        cat_smf_editor::score::pattern::is_source(&loaded.music, track)
            && !app
                .patterns
                .editing
                .as_ref()
                .is_some_and(|c| c.standalone && c.track == track)
    })
}

/// Scene and standalone edits play what they show, except when playing whole tracks.
fn plays_shown(app: &App, mode: Mode) -> bool {
    !matches!(mode, Mode::AllTracks | Mode::Track)
        && app
            .patterns
            .editing
            .as_ref()
            .is_some_and(|c| c.scene.is_some() || c.standalone)
}

/// What `track` sounds when played in `mode`: nothing for a silent pattern source,
/// otherwise its events with same-measure references expanded.
pub(crate) fn sounding(app: &App, track: usize, mode: Mode) -> Option<Sounding<'_>> {
    let loaded = app.loaded.as_ref()?;
    if silent(app, track) {
        return Some(Sounding::default());
    }
    let (events, end) = if plays_shown(app, mode) {
        let events = &loaded.visible().tracks.get(track)?.events;
        (
            Cow::Borrowed(events.as_slice()),
            events.last().map_or(0, |e| e.tick),
        )
    } else {
        cat_smf_editor::score::same_measure::expand(&loaded.music, track).ok()?
    };
    let durations = if matches!(events, Cow::Owned(_)) || loaded.display_music.is_some() {
        Cow::Owned(NoteView::new(&events).durations)
    } else {
        Cow::Borrowed(loaded.note_views[track].durations.as_slice())
    };
    Some(Sounding {
        events,
        durations,
        end,
    })
}

/// One track of `Loaded::music` with same-measure references expanded, built when the song changes.
pub(crate) enum Expansion {
    /// No references: the stored events, with their lengths.
    Plain { durations: Vec<Option<u64>> },
    Expanded {
        events: Vec<Event>,
        durations: Vec<Option<u64>>,
        end: u64,
    },
    /// The references are invalid, so the track does not sound.
    Invalid,
}

#[cfg(test)]
thread_local! {
    pub(crate) static EXPANSIONS_BUILT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Expands every track of `music`; `note_views` are `music`'s own.
pub(crate) fn expand_all(music: &Music, note_views: &[NoteView]) -> Vec<Expansion> {
    #[cfg(test)]
    EXPANSIONS_BUILT.with(|n| n.set(n.get() + 1));
    (0..music.tracks.len())
        .map(
            |track| match cat_smf_editor::score::same_measure::expand(music, track) {
                Err(_) => Expansion::Invalid,
                Ok((Cow::Borrowed(_), _)) => Expansion::Plain {
                    durations: note_views[track].durations.clone(),
                },
                Ok((Cow::Owned(events), end)) => Expansion::Expanded {
                    durations: NoteView::new(&events).durations,
                    events,
                    end,
                },
            },
        )
        .collect()
}

/// Every track as it sounds when played from the cursor, so views never show
/// stored records that playback expands or skips.
pub(crate) fn performance(app: &App) -> Vec<Sounding<'_>> {
    let Some(loaded) = app.loaded.as_ref() else {
        return Vec::new();
    };
    (0..loaded.music.tracks.len())
        .map(|track| performed(app, loaded, track).unwrap_or_default())
        .collect()
}

/// `sounding(app, track, Mode::Position)`, borrowing what `Loaded` keeps.
fn performed<'a>(app: &App, loaded: &'a Loaded, track: usize) -> Option<Sounding<'a>> {
    if silent(app, track) {
        return Some(Sounding::default());
    }
    if plays_shown(app, Mode::Position) {
        // `note_views` describe the shown song whenever `display_music` is set.
        let events = &loaded.visible().tracks.get(track)?.events;
        return Some(Sounding {
            events: Cow::Borrowed(events.as_slice()),
            durations: Cow::Borrowed(loaded.note_views.get(track)?.durations.as_slice()),
            end: events.last().map_or(0, |e| e.tick),
        });
    }
    match loaded.expansions.get(track)? {
        Expansion::Invalid => None,
        Expansion::Plain { durations } => {
            let events = &loaded.music.tracks[track].events;
            Some(Sounding {
                events: Cow::Borrowed(events.as_slice()),
                durations: Cow::Borrowed(durations.as_slice()),
                end: events.last().map_or(0, |e| e.tick),
            })
        }
        Expansion::Expanded {
            events,
            durations,
            end,
        } => Some(Sounding {
            events: Cow::Borrowed(events.as_slice()),
            durations: Cow::Borrowed(durations.as_slice()),
            end: *end,
        }),
    }
}

#[cfg(test)]
mod tests;
