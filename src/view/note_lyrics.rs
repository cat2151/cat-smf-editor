//! Lyrics attached to drawable notes of a singing track, shared by staff and roll.
use crate::app::Loaded;
use cat_smf_editor::{edit::lyric::LYRIC, score::note_view::note_on, voicevox};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn read(loaded: &Loaded, track: usize) -> BTreeMap<u64, String> {
    let source = &loaded.music.tracks[track];
    if !voicevox::is_singer(source) {
        return BTreeMap::new();
    }
    let ticks: BTreeSet<_> = source
        .events
        .iter()
        .enumerate()
        .filter(|(i, event)| note_on(event) && loaded.note_views[track].durations[*i].is_some())
        .map(|(_, event)| event.tick)
        .collect();
    let mut lyrics = BTreeMap::new();
    for event in &source.events {
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
pub(super) mod tests;
