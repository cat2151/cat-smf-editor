//! Restart the running playback range at its current position, e.g. with new patches.
use crate::{
    app::App,
    play::audio_sequence::Sequence,
    play::transport::{Active, Prepared},
};

pub fn suspend(app: &mut App) -> Option<Prepared> {
    if let Some(prepared) = app.playback.pending.take() {
        return Some(prepared);
    }
    let active = app.playback.active.take()?;
    let elapsed = app.audition.seconds()?;
    skip(app, active, elapsed)
}

/// `active` without its first `elapsed` seconds; None once nothing remains.
pub fn skip(app: &App, mut active: Active, elapsed: f64) -> Option<Prepared> {
    let elapsed = elapsed.min(active.end);
    if elapsed >= active.end {
        return None;
    }
    active.sources = active
        .sources
        .into_iter()
        .map(|(track, sequence)| (track, remaining(sequence, elapsed)))
        .collect();
    active.start_seconds += elapsed;
    active.end -= elapsed;
    let loaded = app.loaded.as_ref()?;
    let parts = active
        .sources
        .iter()
        .map(|(track, sequence)| (*track, loaded.patches[*track].clone(), sequence.clone()))
        .collect();
    Some(Prepared { parts, active })
}

fn remaining(mut sequence: Sequence, elapsed: f64) -> Sequence {
    sequence.skip(elapsed);
    sequence
}

pub fn restore(app: &mut App, mut prepared: Prepared) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    for (track, patch, _) in &mut prepared.parts {
        let Some(current) = loaded.patches.get(*track) else {
            return;
        };
        *patch = current.clone();
    }
    // Patch changes can remove track-head macro values. Never resume the old fixed-value plan.
    let selected: Vec<_> = prepared.parts.iter().map(|(track, _, _)| *track).collect();
    prepared.active.automation =
        super::automation::Automation::scoped(app, &prepared.parts, false, &selected);
    // Audio setup happens on the next frame, after the returned UI is painted.
    app.playback.pending = Some(prepared);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::audio_sequence::Note;

    #[test]
    fn remaining_keeps_held_note_and_future_events_with_original_end() {
        let sequence = remaining(
            Sequence::new(
                vec![
                    Note {
                        start: 0.0,
                        duration: 0.5,
                        pitch: 59,
                        velocity: 40,
                    },
                    Note {
                        start: 0.5,
                        duration: 2.0,
                        pitch: 60,
                        velocity: 80,
                    },
                    Note {
                        start: 3.0,
                        duration: 0.5,
                        pitch: 62,
                        velocity: 70,
                    },
                ],
                4.0,
            ),
            1.5,
        );
        assert_eq!(sequence.notes.len(), 2);
        assert_eq!(
            (
                sequence.notes[0].pitch,
                sequence.notes[0].start,
                sequence.notes[0].duration
            ),
            (60, 0.0, 1.0)
        );
        assert_eq!(sequence.notes[1].start, 1.5);
        assert_eq!(sequence.end, 2.5);
    }
}
