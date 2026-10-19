use super::*;

/// Stable ordering by the midpoint of each track's playable note range; empty tracks last.
pub(super) fn tracks(app: &App, loaded: &Loaded) -> Vec<usize> {
    let mut tracks = app
        .track_view
        .split
        .map_or_else(|| vec![app.track], |tracks| tracks.to_vec());
    let midpoint = |track: usize| {
        let pitches: Vec<_> = loaded.music.tracks[track]
            .events
            .iter()
            .enumerate()
            .filter(|(i, e)| note_on(e) && loaded.note_views[track].durations[*i].is_some())
            .map(|(_, e)| u16::from(e.data[0]))
            .collect();
        pitches
            .iter()
            .min()
            .zip(pitches.iter().max())
            .map(|(low, high)| low + high)
    };
    tracks.sort_by_key(|&track| std::cmp::Reverse(midpoint(track)));
    tracks
}

pub(super) fn fit_height(layout: &mut Layout, height: f32) {
    let factor = height / layout.size.y;
    layout.size.y = height;
    for y in &mut layout.staff {
        *y *= factor;
    }
    for note in layout
        .notes
        .iter_mut()
        .map(|(_, note)| note)
        .chain(&mut layout.references)
    {
        note.min.y *= factor;
        note.max.y *= factor;
    }
}
