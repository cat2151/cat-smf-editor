use super::*;

/// Stable ordering by the midpoint of each track's editable and sounding note range;
/// empty tracks last.
pub(super) fn tracks(app: &App, loaded: &Loaded, sounding: &[Sounding]) -> Vec<usize> {
    let mut tracks = app
        .track_view
        .split
        .map_or_else(|| vec![app.track], |tracks| tracks.to_vec());
    let editable = shown(loaded);
    let midpoint = |track: usize| {
        let pitches: Vec<_> = notes_of(&editable[track])
            .chain(notes_of(&sounding[track]))
            .map(|(_, _, pitch)| u16::from(pitch))
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

pub(crate) fn fit_height(layout: &mut Layout, height: f32) {
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
