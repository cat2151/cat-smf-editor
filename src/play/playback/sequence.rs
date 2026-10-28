//! Prepare note and MIDI event sequences outside the audio callback.
use super::sounding::silent;
use super::*;
use crate::play::audio_sequence::Note;
use cat_smf_editor::score::audition_timing::TempoMap;

pub(super) fn sequence_track(app: &App, track: usize, mode: Mode) -> Option<Sequence> {
    let loaded = app.loaded.as_ref()?;
    if mode == Mode::AllTracks {
        let mut result = Sequence::new(Vec::new(), 0.0);
        for track in 0..loaded.music.tracks.len() {
            let sequence = sequence_track(app, track, Mode::Track)?;
            result.end = result.end.max(sequence.end);
            result.notes.extend(sequence.notes);
            result.events.extend(sequence.events);
        }
        result.notes.sort_by(|a, b| a.start.total_cmp(&b.start));
        result.events.sort_by(|a, b| a.time.total_cmp(&b.time));
        return Some(result);
    }
    if silent(app, track) {
        return Some(Sequence::new(vec![], 0.0));
    }
    let Sounding {
        events: expanded,
        durations,
        end: reference_end,
    } = sounding(app, track, mode)?;
    let events = expanded.as_ref();
    let end_tick = cat_smf_editor::edit::parameter_curve::read(&loaded.music)
        .ok()
        .map(|collection| {
            collection
                .curves
                .keys()
                .filter(|&&curve_track| collection.owner(curve_track) == Some(track))
                .filter_map(|&curve_track| {
                    loaded.music.tracks[curve_track]
                        .events
                        .last()
                        .map(|e| e.tick)
                })
                .fold(events.last().map_or(0, |e| e.tick), u64::max)
        })
        .unwrap_or(events.last()?.tick)
        .max(reference_end);
    let (tick, position) = if mode == Mode::Track {
        (0, None)
    } else {
        location(app)
    };
    let mut start = tick;
    let mut end = end_tick;
    if mode == Mode::Track {
        start = 0;
    }
    if matches!(
        mode,
        Mode::Measure | Mode::MeasureStart | Mode::Beat | Mode::BeatOnly
    ) {
        let map = loaded.time.map(app.track)?;
        let mut p = position?;
        p.tick_numerator = 0;
        p.tick_denominator = 1;
        if matches!(mode, Mode::Measure | Mode::MeasureStart) {
            p.beat = 1;
        }
        start = map.tick_at(&p)?;
        if mode == Mode::Measure {
            p.measure += 1;
            end = map.tick_at(&p)?.min(end);
        } else if mode == Mode::BeatOnly {
            p.beat += 1;
            end = map.tick_at(&p)?.min(end);
        }
    }
    let tempo = if app.patterns.editing.as_ref().is_some_and(|c| c.standalone) {
        TempoMap::constant(loaded.music.division, app.patterns.tempo.0).ok()?
    } else {
        TempoMap::new(&loaded.music, track)
    };
    let selected_event = app.selected_event.and_then(|index| {
        let visible = &loaded.visible().tracks[app.track].events;
        let selected = visible.get(index)?;
        let same = |e: &&cat_smf_editor::model::Event| {
            e.tick == selected.tick && e.status == selected.status && e.data == selected.data
        };
        let occurrence = visible[..index].iter().filter(same).count();
        events
            .iter()
            .enumerate()
            .filter(|(_, e)| same(e))
            .nth(occurrence)
            .map(|(i, _)| i)
    });
    let selected: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            note_on(e)
                && match mode {
                    Mode::Event if track == app.track => selected_event == Some(*i),
                    Mode::Event => e.tick == tick,
                    Mode::Notes => e.tick == tick,
                    _ => e.tick >= start && e.tick < end,
                }
        })
        .map(|(i, _)| i)
        .collect();
    let notes: Vec<_> = selected
        .iter()
        .map(|&i| (i, &events[i]))
        .map(|(i, e)| {
            let duration = durations[i].unwrap_or(end.saturating_sub(e.tick));
            let duration = if matches!(mode, Mode::Notes | Mode::Event) {
                duration
            } else {
                duration.min(end.saturating_sub(e.tick))
            };
            Note {
                start: tempo.duration(start, e.tick.saturating_sub(start)),
                duration: if matches!(mode, Mode::Event | Mode::Notes) && durations[i].is_none() {
                    f64::INFINITY
                } else {
                    tempo.duration(e.tick, duration)
                },
                pitch: e.data[0],
                velocity: e.data[1],
            }
        })
        .collect();
    let end = if matches!(mode, Mode::Event | Mode::Notes) {
        notes
            .iter()
            .map(|n| n.start + n.duration)
            .fold(0.0, f64::max)
    } else {
        tempo.duration(start, end.saturating_sub(start))
    };
    Some(Sequence::from_source(
        notes, end, events, &selected, &tempo, start,
    ))
}
