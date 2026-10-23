use crate::app::App;
use cat_smf_editor::{score::display_rows::DisplayRow, score::note_view::note_on};
use eframe::egui;

pub fn sync_mode(app: &mut App) {
    app.track_view.cursor_tick = None;
    app.cursor_position = None;
    app.cursor_row = app.loaded.as_ref().and_then(|loaded| {
        app.selected_event.and_then(|i| {
            if app.note_mode {
                loaded.display_rows[app.track].event_row(i)
            } else {
                Some(i)
            }
        })
    });
}

/// Places the cursor at an exact tick, including gaps and the insertion end.
pub fn at_tick(app: &mut App, tick: u64) {
    let Some(loaded) = &app.loaded else { return };
    let row = crate::cursor::split::row_sounding_loaded(loaded, app.note_mode, app.track, tick);
    let count = if app.note_mode {
        loaded.display_rows[app.track].len()
    } else {
        loaded.music.tracks[app.track].events.len()
    };
    app.cursor_row = (count > 0).then_some(row.min(count.saturating_sub(1)));
    app.selected_event = None;
    app.cursor_position = loaded.time.map(app.track).map(|m| m.position(tick));
    app.track_view.cursor_tick = Some(tick);
    app.reveal_cursor = true;
}

/// Selects the row and auditions the chord at its tick (click, beat moves).
pub fn select(app: &mut App, row: usize) {
    place(app, row);
    audition(app, true);
}

pub fn place(app: &mut App, row: usize) {
    app.playback.pending = None;
    app.playback.active = None;
    app.audition.stop();
    let Some(loaded) = &app.loaded else {
        return;
    };
    app.cursor_row = Some(row);
    app.track_view.cursor_tick = None;
    app.cursor_position = None;
    app.selected_event = if app.note_mode {
        match loaded.display_rows[app.track].get(row) {
            DisplayRow::Event(i) => Some(i),
            DisplayRow::Beat(_) => None,
        }
    } else {
        Some(row)
    };
}

fn audition(app: &mut App, chord: bool) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    let events = &loaded.music.tracks[app.track].events;
    let pitches = app.selected_event.map_or_else(Vec::new, |i| {
        let tick = events[i].tick;
        let range = if !chord {
            i..i + 1
        } else if note_on(&events[i]) || !loaded.note_views[app.track].keyboard_notes[i].is_empty()
        {
            // The keyboard is grouped at the end of this tick, even when the cursor
            // reaches an earlier note row of the chord.
            let start = events.partition_point(|e| e.tick < tick);
            start..events.partition_point(|e| e.tick <= tick)
        } else {
            i..i
        };
        range
            .filter(|&index| note_on(&events[index]))
            .map(|index| {
                let duration = loaded.note_views[app.track].durations[index].map(|d| {
                    cat_smf_editor::score::audition_timing::duration_seconds(
                        &loaded.music,
                        app.track,
                        tick,
                        d,
                    )
                });
                (events[index].data[0], events[index].data[1], duration)
            })
            .collect()
    });
    if pitches.is_empty() {
        app.audition.stop();
        return;
    }
    let tick = app.selected_event.map(|i| events[i].tick);
    let take = tick.and_then(|tick| crate::play::sung::audition::cursor(app, app.track, tick));
    if crate::play::automation::audition_notes(
        app,
        app.track,
        tick.unwrap_or(0),
        &pitches,
        take.clone(),
    ) {
        return;
    }
    let loaded = app.loaded.as_ref().unwrap();
    app.audition
        .play_track(app.track, &loaded.patches[app.track], &pitches, take);
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

pub(crate) fn move_rows(app: &mut App, step: isize) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    let count = if app.note_mode {
        loaded.display_rows[app.track].len()
    } else {
        loaded.music.tracks[app.track].events.len()
    };
    if count == 0 {
        return;
    }
    let current = app.cursor_row.or_else(|| {
        app.selected_event.and_then(|i| {
            if app.note_mode {
                app.loaded.as_ref()?.display_rows[app.track].event_row(i)
            } else {
                Some(i)
            }
        })
    });
    let first = if step == isize::MAX { count - 1 } else { 0 };
    let next = current.map_or(first, |i| i.saturating_add_signed(step).min(count - 1));
    // Playback follow and manual scrolling can hide an unchanged cursor row.
    // Reapply placement even at an edge to clear exact-tick overrides and playback.
    place(app, next);
    audition(app, false);
    app.reveal_cursor = true;
}

pub(crate) fn boundary(app: &mut App, forward: bool, measure: bool) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    let Some(map) = loaded.time.map(app.track) else {
        return;
    };
    let events = &loaded.music.tracks[app.track].events;
    let Some(last) = events.last() else {
        return;
    };
    let (_, Some(current)) = crate::play::playback::location(app) else {
        return;
    };
    let target = if measure {
        cat_smf_editor::score::musical_time::Position {
            measure: if forward {
                current.measure + 1
            } else {
                current.measure.saturating_sub(1).max(1)
            },
            beat: 1,
            tick_numerator: 0,
            tick_denominator: 1,
        }
    } else {
        let Some(target) = map.adjacent_beat(&current, forward, last.tick) else {
            if !forward {
                move_rows(app, 0);
            }
            return;
        };
        target
    };
    if (target.measure, target.beat) > {
        let p = map.position(last.tick);
        (p.measure, p.beat)
    } {
        return;
    }
    let count = if app.note_mode {
        loaded.display_rows[app.track].len()
    } else {
        events.len()
    };
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = low + (high - low) / 2;
        let position = if app.note_mode {
            match loaded.display_rows[app.track].get(mid) {
                DisplayRow::Beat(p) => p,
                DisplayRow::Event(i) => map.position(events[i].tick),
            }
        } else {
            map.position(events[mid].tick)
        };
        if (position.measure, position.beat) < (target.measure, target.beat) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    if low < count {
        let exact = app.note_mode || map.position(events[low].tick) == target;
        place(app, low);
        if !exact {
            app.cursor_position = Some(target);
            app.selected_event = None;
        }
        if measure {
            // Only the current track's landing measure, without changing Space's method.
            app.playback.pending =
                crate::play::transport::prepare(app, crate::play::playback::Mode::Measure);
        } else {
            audition(app, true);
        }
        app.reveal_cursor = true;
    }
}

#[cfg(test)]
mod tests;
