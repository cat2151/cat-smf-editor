//! 再生で歌う track と、その譜面。
use super::singer;
use crate::{app::App, play::transport::Prepared};
use cat_smf_editor::voicevox::{score, score::Score, Voice};

/// `prepared` で歌う track と、その譜面。pattern・Scene・Song の再生では、鳴らす event 列と
/// tempo から作り、入れ物 track は自分の歌声で pattern の音符を全部歌う。
pub(crate) fn scores(app: &App, prepared: &Prepared) -> Vec<(usize, Score)> {
    let Some(loaded) = &app.loaded else {
        return Vec::new();
    };
    let Some(origin) = &prepared.origin else {
        return prepared
            .parts
            .iter()
            .filter(|(track, _, _)| singer(app, *track))
            .filter_map(|(track, _, _)| Some((*track, score::from_track(&loaded.music, *track)?)))
            .collect();
    };
    prepared
        .parts
        .iter()
        .filter_map(|(track, _, _)| {
            let voice = Voice::read(loaded.music.tracks.get(*track)?)?;
            let events = origin.events.get(track)?;
            Some((*track, score::from_events(events, &origin.tempo, voice)?))
        })
        .collect()
}
