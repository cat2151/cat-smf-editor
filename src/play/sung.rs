//! VOICEVOX が歌った WAV を、note-on ごとにその note の片から鳴らす。
//!
//! 片 = 譜面の note の開始 frame から次の片の開始 frame まで。子音は前の片に入っているので、
//! note-on のたびに片の頭へ移り、次の note-on まで続けて読めば元の WAV と同じ sample になる。
use crate::play::audio_sequence::Sequence;
use cat_smf_editor::voicevox::score::{Score, FRAMES_PER_SECOND, LEAD_FRAMES};
use std::ops::Range;
use std::sync::Arc;

/// 1回の再生ぶんの、歌った WAV と note-on ごとの読み出し範囲（stereo frame 単位）。
#[derive(Default)]
pub struct Take {
    samples: Arc<Vec<f32>>,
    /// 再生開始の位置が休符の途中なら、その位置から次の note の片の前まで。
    lead: Option<Range<usize>>,
    /// `Sequence` の note-on（同じ frame の note は1つ）ごと。`None` は今の音を続ける。
    cues: Vec<Option<Range<usize>>>,
}

impl Take {
    /// 歌えない track を黙らせる。
    pub fn silent() -> Self {
        Self::default()
    }

    /// note-on のうち、歌った片から鳴らすものの数。
    #[cfg(test)]
    pub fn voiced(&self) -> usize {
        self.cues.iter().flatten().count()
    }
}

/// `samples` は `score` を `rate` Hz・stereo で歌った WAV。`sequence` はこの track の再生分で、
/// その時刻 0 が曲の `start_seconds` 秒にあたる。
pub fn take(
    score: &Score,
    samples: Arc<Vec<f32>>,
    rate: u32,
    sequence: &Sequence,
    start_seconds: f64,
) -> Take {
    let total = samples.len() / 2;
    let sample =
        |frame: f64| ((frame * f64::from(rate) / FRAMES_PER_SECOND).round() as usize).min(total);
    let song_frame = |seconds: f64| LEAD_FRAMES as f64 + seconds * FRAMES_PER_SECOND;
    // 譜面の frame は秒を丸めた値なので、半 frame までの差は同じ片とみなす。
    let piece_at = |frame: f64| {
        score
            .starts
            .partition_point(|&start| start as f64 <= frame + 0.5)
            .checked_sub(1)
    };
    let run_end = |piece: usize| {
        let next = (piece + 1..score.notes.len()).find(|&k| score.notes[k].key.is_some());
        sample(next.map_or(score.total_frames(), |k| score.starts[k]) as f64)
    };
    let lead = piece_at(song_frame(start_seconds))
        .filter(|&j| score.notes[j].key.is_none())
        .map(|j| sample(song_frame(start_seconds))..run_end(j));
    let frames = |seconds: f64| (seconds * f64::from(rate)).round().max(0.0) as u64;
    let mut cues = Vec::new();
    let mut notes = sequence.notes.iter().peekable();
    while let Some(first) = notes.next() {
        let at = frames(first.start);
        let mut duration = first.duration;
        while let Some(note) = notes.next_if(|n| frames(n.start) == at) {
            duration = duration.max(note.duration);
        }
        let seconds = start_seconds + first.start;
        let cue = piece_at(song_frame(seconds))
            .filter(|&j| score.notes[j].key.is_some())
            .map(|j| {
                let start = sample(score.starts[j] as f64);
                let note_end = (score.starts[j] + score.notes[j].frame_length - LEAD_FRAMES) as f64
                    / FRAMES_PER_SECOND;
                // 再生範囲の終わりで切られた note は、その長さで止める。
                let cut = seconds + duration < note_end - 0.5 / FRAMES_PER_SECOND;
                let end = if cut {
                    run_end(j).min(start + frames(duration) as usize)
                } else {
                    run_end(j)
                };
                start..end.max(start)
            });
        cues.push(cue);
    }
    Take {
        samples,
        lead,
        cues,
    }
}

/// 試聴の1音: 曲の `seconds` 秒に始まる note の片だけを鳴らす take。そこが歌う note でなければ `None`。
pub fn piece(score: &Score, samples: Arc<Vec<f32>>, rate: u32, seconds: f64) -> Option<Take> {
    let whole = Sequence::new(
        vec![crate::play::audio_sequence::Note {
            start: 0.0,
            duration: f64::from(u32::MAX),
            pitch: 0,
            velocity: 1,
        }],
        0.0,
    );
    let made = take(score, samples, rate, &whole, seconds);
    made.cues.first()?.as_ref()?;
    Some(made)
}

/// [`Take`] を鳴らす音源。曲ごとの take は再生開始のたびに [`Player::cue`] で届く。
#[derive(Default)]
pub struct Player {
    take: Option<Arc<Take>>,
    next: usize,
    playing: Option<Range<usize>>,
}

impl Player {
    /// 新しい再生の先頭で呼ぶ。`None` なら歌わない（track の音源が鳴る）。
    pub fn cue(&mut self, take: Option<Arc<Take>>) {
        self.playing = take.as_ref().and_then(|t| t.lead.clone());
        self.take = take;
        self.next = 0;
    }

    pub fn active(&self) -> bool {
        self.take.is_some()
    }

    /// `velocities` に音があれば note-on。`out` は `channels` 本のインターリーブ。
    pub fn render(
        &mut self,
        velocities: &[u8; 128],
        changed: i32,
        out: &mut [f32],
        channels: usize,
    ) {
        out.fill(0.0);
        let Some(take) = &self.take else {
            return;
        };
        if changed != 0
            && velocities.iter().any(|&v| v > 0)
            && let Some(cue) = take.cues.get(self.next)
        {
            if let Some(range) = cue {
                self.playing = Some(range.clone());
            }
            self.next += 1;
        }
        let Some(range) = &mut self.playing else {
            return;
        };
        for frame in out.chunks_mut(channels) {
            if range.start >= range.end {
                break;
            }
            let at = range.start * 2;
            for (c, sample) in frame.iter_mut().enumerate() {
                *sample = take.samples[at + c.min(1)];
            }
            range.start += 1;
        }
        if range.start >= range.end {
            self.playing = None;
        }
    }
}

pub mod audition;
pub mod prerender;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
pub(crate) mod tests;
