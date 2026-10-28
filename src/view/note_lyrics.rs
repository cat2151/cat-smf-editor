//! Lyrics attached to drawable notes of a singing track, shared by staff and roll.
use crate::play::playback::Sounding;
use cat_smf_editor::{edit::lyric::LYRIC, model::Music, score::note_view::note_on, voicevox};
use std::collections::{BTreeMap, BTreeSet};

/// Lyric at each note onset of `source`, the events `track` of `music` sounds,
/// when the track has a voice.
pub(super) fn read(music: &Music, track: usize, source: &Sounding) -> BTreeMap<u64, String> {
    if !music.tracks.get(track).is_some_and(voicevox::is_singer) {
        return BTreeMap::new();
    }
    let ticks: BTreeSet<_> = source
        .events
        .iter()
        .enumerate()
        .filter(|(i, event)| note_on(event) && source.durations[*i].is_some())
        .map(|(_, event)| event.tick)
        .collect();
    let mut lyrics = BTreeMap::new();
    for event in source.events.iter() {
        if event.status == 0xff
            && event.meta_type == LYRIC
            && !event.data.is_empty()
            && ticks.contains(&event.tick)
        {
            lyrics
                .entry(event.tick)
                .or_insert_with(|| String::from_utf8_lossy(&event.data).into_owned());
        }
    }
    lyrics
}

#[cfg(test)]
mod pattern_edit_tests;
#[cfg(test)]
pub(super) mod tests;
