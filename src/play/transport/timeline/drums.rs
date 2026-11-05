//! Rebuild at the original origin so the audio clock and scope stay intact.
use super::*;

pub(crate) fn reload_drums(app: &mut App, track: usize) {
    let active = app
        .playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(app.playback.active.as_ref());
    let Some(active) = active else { return };
    let Some(map) = active.tempos.get(active.clock_track) else {
        return;
    };
    let (patch, sequence) = if matches!(active.mode, Mode::Event | Mode::Notes) {
        // These auditions contain only the selected hit(s). Rebuilding a whole
        // time range would add other rows and later hits to a held-note preview.
        let Some((_, source)) = active.sources.iter().find(|s| s.0 == track) else {
            return;
        };
        let Some(loaded) = &app.loaded else { return };
        let patch = loaded.patches[track].clone();
        let mut sequence = source.clone();
        if let Some(drums) = &patch.drums {
            for note in &mut sequence.notes {
                if let Some(row) = cat_smf_editor::score::drums::NOTES
                    .iter()
                    .position(|&p| p == note.pitch)
                {
                    note.velocity = drums.rows[row].velocity;
                }
            }
            for event in &mut sequence.events {
                if (event.word >> 20) & 15 == 9
                    && event.word & 127 != 0
                    && let Some(row) = cat_smf_editor::score::drums::NOTES
                        .iter()
                        .position(|&p| u32::from(p) == (event.word >> 8) & 127)
                {
                    event.word = (event.word & !127) | u32::from(drums.rows[row].velocity);
                }
            }
        }
        (patch, sequence)
    } else {
        let start = map.tick_after(0, active.start_seconds + 1e-9);
        let end = map.tick_after(0, active.start_seconds + active.end + 1e-9);
        let tracks: Vec<_> = active.sources.iter().map(|s| s.0).collect();
        let Some(mut prepared) = prepare(app, start, end, &tracks, active.mode, active.clock_track)
        else {
            return;
        };
        let Some(index) = prepared.parts.iter().position(|p| p.0 == track) else {
            return;
        };
        let (_, patch, sequence) = prepared.parts.swap_remove(index);
        (patch, sequence)
    };
    if let Some(pending) = &mut app.playback.pending {
        if let Some(part) = pending.parts.iter_mut().find(|p| p.0 == track) {
            part.1 = patch;
            part.2 = sequence.clone();
        }
        if let Some(source) = pending.active.sources.iter_mut().find(|s| s.0 == track) {
            source.1 = sequence;
        }
    } else if let Some(active) = &mut app.playback.active {
        app.audition
            .reload_drums(track, &patch, &sequence, active.start_seconds);
        if let Some(source) = active.sources.iter_mut().find(|s| s.0 == track) {
            source.1 = sequence;
        }
    }
}
