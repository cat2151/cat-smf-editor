//! 試聴（カーソル移動・MML 入力・鍵盤・Recent の先頭小節）で、歌う track を歌声の片で鳴らす。
//! 歌声がまだ無ければ `None` を返し、呼び出し側は track の音色で鳴らす。
use super::{piece, prerender, take, Take};
use crate::{app::App, play::audio_sequence::Sequence};
use cat_smf_editor::{
    model::Music,
    score::{audition_timing::TempoMap, instrument::Patch},
    voicevox::{
        cache, is_singer,
        score::{self, one_note, FRAMES_PER_SECOND},
        Voice,
    },
};
use std::path::Path;
use std::sync::Arc;

/// `track` の `tick` に始まる note の片。曲がまだ歌われていなければ裏で歌わせ始める。
pub fn cursor(app: &mut App, track: usize, tick: u64) -> Option<Arc<Take>> {
    let (score, samples) = prerender::song(app, track)?;
    let music = &app.loaded.as_ref()?.music;
    let seconds = TempoMap::new(music, track).duration(0, tick);
    piece(&score, samples, app.audition.rate(), seconds).map(Arc::new)
}

/// 歌う `track` で `pitch` を `seconds` 秒歌う1音。まだ歌っていなければ裏で歌わせる。
pub fn note(app: &mut App, track: usize, pitch: u8, seconds: f64) -> Option<Arc<Take>> {
    let loaded = app.loaded.as_ref()?;
    if !loaded.music.tracks.get(track).is_some_and(is_singer) {
        return None;
    }
    let frames = ((seconds * FRAMES_PER_SECOND).round() as u64).max(1);
    let voice = Voice::read(loaded.music.tracks.get(track)?)?;
    let samples = prerender::note(app, voice, (pitch, frames))?;
    piece(&one_note(pitch, frames), samples, app.audition.rate(), 0.0).map(Arc::new)
}

/// 鍵盤の1音。押している長さは先に分からないので、cursor の位置の tempo で1拍歌う。
pub fn key(app: &mut App, track: usize, pitch: u8) -> Option<Arc<Take>> {
    let loaded = app.loaded.as_ref()?;
    let (tick, _) = crate::play::playback::location(app);
    let beat = TempoMap::new(&loaded.music, track).duration(tick, u64::from(loaded.music.division));
    note(app, track, pitch, beat)
}

/// 保存済みの歌った WAV がある歌う track の take。engine は起こさない。
pub fn stored(
    music: &Music,
    parts: &[(usize, Patch, Sequence)],
    start_seconds: f64,
    rate: u32,
    directory: &Path,
) -> Vec<(usize, Arc<Take>)> {
    parts
        .iter()
        .filter(|(track, _, _)| music.tracks.get(*track).is_some_and(is_singer))
        .filter_map(|(track, _, sequence)| {
            let score = score::from_track(music, *track)?;
            let samples =
                cache::load_or_sing(directory, &score, rate, |_, _| Err(String::new())).ok()?;
            let made = take(&score, Arc::new(samples), rate, sequence, start_seconds);
            Some((*track, Arc::new(made)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
