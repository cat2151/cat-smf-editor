//! Capture auditions follow song-wide sequence order across containers.
use super::*;

pub(super) struct Playing {
    phrase: Captured,
    start: f64,
    duration: f64,
    end: f64,
}

fn next<'a>(p: &Captured, phrases: &'a [Captured]) -> Option<&'a Captured> {
    phrases
        .iter()
        .filter(|n| n.sequence > p.sequence)
        .min_by_key(|n| n.sequence)
}

pub(super) fn start(a: &mut App, p: &Captured, phrases: &[Captured]) -> Result<Playing, String> {
    let song_tick = crate::view::patterns::timing::pattern_tick(a, p.pattern)?;
    let tempo = crate::view::patterns::timing::pattern(a, p.pattern)?;
    let start = tempo.seconds_at(p.range.0);
    let duration = tempo.duration(p.range.0, p.range.1 - p.range.0);
    let gap = if next(p, phrases).is_some() {
        tempo.duration(
            p.range.1,
            u64::from(a.loaded.as_ref().unwrap().music.division) * 4,
        )
    } else {
        0.0
    };
    let mut prepared = crate::view::patterns::preview::remaining(a, p.pattern, start, duration)
        .ok_or("Capture phrase cannot be played")?;
    // Cut the source at this phrase, then retain one silent bar before the next number.
    for (_, _, sequence) in &mut prepared.parts {
        sequence.end = duration + gap;
    }
    prepared.active.end = duration + gap;
    prepared.active.sources = prepared
        .parts
        .iter()
        .map(|(t, _, s)| (*t, s.clone()))
        .collect();
    crate::view::patterns::library::edit(a, p.pattern)?;
    crate::play::transport::stop(a);
    a.playback.last = crate::play::playback::Mode::Pattern(p.pattern);
    a.playback.follow = false;
    a.playback.pending = Some(prepared);
    a.patterns.timing.playing = Some((
        cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern(p.pattern),
        song_tick,
    ));
    a.patterns.preview = Some(p.pattern);
    crate::cursor::moves::at_tick(a, p.range.0);
    Ok(Playing {
        phrase: p.clone(),
        start,
        duration,
        end: duration + gap,
    })
}

impl Playing {
    // A stop, seek or playback started elsewhere cancels this chain.
    fn elapsed(&self, a: &App) -> Option<f64> {
        let pending = a.playback.pending.as_ref();
        let active = pending.map(|p| &p.active).or(a.playback.active.as_ref())?;
        if active.mode != crate::play::playback::Mode::Pattern(self.phrase.pattern)
            || active.start_seconds != self.start
            || active.end != self.end
        {
            return None;
        }
        if pending.is_some() {
            Some(0.0)
        } else {
            a.audition.seconds()
        }
    }

    pub(super) fn caption(&self, a: &App, phrases: &[Captured]) -> Option<String> {
        let elapsed = self.elapsed(a)?;
        if elapsed < self.duration {
            return Some(format!("#{}", self.phrase.sequence));
        }
        let next = next(&self.phrase, phrases)?;
        let wait = (self.end - elapsed).ceil().max(1.0) as u64;
        Some(format!(
            "#{} to #{} wait {wait}",
            self.phrase.sequence, next.sequence
        ))
    }

    pub(super) fn advance(self, a: &mut App, phrases: &[Captured]) -> Result<Option<Self>, String> {
        let Some(elapsed) = self.elapsed(a) else {
            return Ok(None);
        };
        if a.audition.sequence_failed() {
            return Ok(None);
        }
        if elapsed < self.end {
            return Ok(Some(self));
        }
        next(&self.phrase, phrases)
            .map(|p| start(a, p, phrases))
            .transpose()
    }
}
