//! SMF の1 track を VOICEVOX の譜面（`notes` の列）へ変換する。
use crate::edit::lyric::text_at;
use crate::model::{Event, Music};
use crate::score::audition_timing::TempoMap;
use crate::score::note_view::{note_on, NoteView};
use serde::Serialize;

/// 譜面の 1 frame は 256 / 24000 秒。
pub const FRAMES_PER_SECOND: f64 = 24_000.0 / 256.0;
/// 先頭の休符。engine は休符で始まらない譜面を拒み、最初の子音もこの枠に入る。
pub const LEAD_FRAMES: u64 = 15;
/// 末尾の休符。最後の音の減衰をこの枠に入れる。
pub const TAIL_FRAMES: u64 = 15;
/// 歌詞の無い note を歌う音。
pub const DEFAULT_LYRIC: &str = "ら";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScoreNote {
    pub key: Option<u8>,
    pub frame_length: u64,
    pub lyric: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub voice: super::Voice,
    pub notes: Vec<ScoreNote>,
    /// `notes[i]` の wav 上の開始 frame（それより前の `frame_length` の総和）。
    pub starts: Vec<u64>,
    /// `notes[i]` の元の Note On の event index。休符は `None`。
    pub sources: Vec<Option<usize>>,
}

impl Score {
    /// engine の `/sing_frame_audio_query` へそのまま送る body。
    pub fn json(&self) -> String {
        #[derive(Serialize)]
        struct Body<'a> {
            notes: &'a [ScoreNote],
        }
        serde_json::to_string(&Body { notes: &self.notes }).expect("score is serializable")
    }

    pub fn total_frames(&self) -> u64 {
        self.notes.iter().map(|n| n.frame_length).sum()
    }

    fn push(&mut self, key: Option<u8>, frames: u64, lyric: String, source: Option<usize>) {
        self.starts.push(self.total_frames());
        self.notes.push(ScoreNote {
            key,
            frame_length: frames,
            lyric,
        });
        self.sources.push(source);
    }
}

/// `track` が鳴らす note（同じ小節の参照を展開した後）のうち、歌う note を
/// 単音の譜面にする。歌う note が無ければ `None`。
pub fn from_track(music: &Music, track: usize) -> Option<Score> {
    let voice = super::Voice::read(music.tracks.get(track)?).unwrap_or_default();
    let (events, _) = crate::score::same_measure::expand(music, track).ok()?;
    let tempo = TempoMap::new(music, track);
    from_events(&events, &tempo, voice)
}

/// 鳴らす event 列の note を、`tempo` の時刻で単音の譜面にする。歌う note が無ければ `None`。
///
/// 同じ tick の note は最も高い音だけを歌い、後から始まる note は前の note を切る。
pub fn from_events(events: &[Event], tempo: &TempoMap, voice: super::Voice) -> Option<Score> {
    let durations = NoteView::new(events).durations;
    let end_tick = events.last()?.tick;
    let mut melody: Vec<usize> = Vec::new();
    for (i, event) in events.iter().enumerate().filter(|(_, e)| note_on(e)) {
        match melody.last() {
            Some(&last) if events[last].tick == event.tick => {
                if event.data[0] > events[last].data[0] {
                    *melody.last_mut().unwrap() = i;
                }
            }
            _ => melody.push(i),
        }
    }
    if melody.is_empty() {
        return None;
    }
    let frame =
        |tick: u64| LEAD_FRAMES + (tempo.duration(0, tick) * FRAMES_PER_SECOND).round() as u64;
    let mut score = Score {
        voice,
        notes: Vec::new(),
        starts: Vec::new(),
        sources: Vec::new(),
    };
    let mut cursor = 0;
    for (n, &i) in melody.iter().enumerate() {
        let start_tick = events[i].tick;
        let own_end = start_tick.saturating_add(durations[i].unwrap_or(end_tick - start_tick));
        let end_tick = melody
            .get(n + 1)
            .map_or(own_end, |&next| own_end.min(events[next].tick));
        let (start, end) = (frame(start_tick), frame(end_tick));
        if end <= start {
            continue;
        }
        if start > cursor {
            score.push(None, start - cursor, String::new(), None);
        }
        let lyric = text_at(events, start_tick).unwrap_or_else(|| DEFAULT_LYRIC.to_owned());
        score.push(Some(events[i].data[0]), end - start, lyric, Some(i));
        cursor = end;
    }
    score.push(None, TAIL_FRAMES, String::new(), None);
    Some(score)
}

/// 歌詞 [`DEFAULT_LYRIC`] の1音だけの譜面。試聴で鳴らす音を、曲と別に歌わせる。
pub fn one_note(key: u8, frames: u64) -> Score {
    let mut score = Score {
        voice: Default::default(),
        notes: Vec::new(),
        starts: Vec::new(),
        sources: Vec::new(),
    };
    score.push(None, LEAD_FRAMES, String::new(), None);
    score.push(Some(key), frames.max(1), DEFAULT_LYRIC.to_owned(), None);
    score.push(None, TAIL_FRAMES, String::new(), None);
    score
}

/// Fixed voice-picker sample: どんぐりころころ, G G E E F E D C, eighth notes at 120 BPM.
pub fn demo(voice: super::Voice) -> Score {
    let mut score = Score {
        voice,
        notes: Vec::new(),
        starts: Vec::new(),
        sources: Vec::new(),
    };
    score.push(None, LEAD_FRAMES, String::new(), None);
    for (i, (key, lyric)) in [67, 67, 64, 64, 65, 64, 62, 60]
        .into_iter()
        .zip(["ど", "ん", "ぐ", "り", "こ", "ろ", "こ", "ろ"])
        .enumerate()
    {
        let start = (i as f64 * FRAMES_PER_SECOND / 4.0).round() as u64;
        let end = ((i + 1) as f64 * FRAMES_PER_SECOND / 4.0).round() as u64;
        score.push(Some(key), end - start, lyric.into(), None);
    }
    score.push(None, TAIL_FRAMES, String::new(), None);
    score
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
