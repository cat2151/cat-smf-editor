//! Every key the virtual keyboard sounds, grouped into phrases and appended to its
//! container's Capture pattern.
use super::{control, fill, note, operations};
use crate::edit;
use crate::message;
use crate::model::{Event, Music};
use crate::score::{
    note_view::{note_off, note_on},
    pattern::{body, Catalog, Kind, Meter},
};

/// Milliseconds from the phrase start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub start: u64,
    pub length: u64,
    pub pitch: u8,
    pub velocity: u8,
}

impl Phrase {
    /// Milliseconds from the phrase start to the end of its last note.
    pub fn end(&self) -> u64 {
        self.notes
            .iter()
            .map(|n| n.start + n.length)
            .max()
            .unwrap_or(0)
    }
}

/// Keys played on one container without a silence of the gap between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phrase {
    /// Milliseconds from the keyboard session start.
    pub start: u64,
    /// `None` for the fallback sound of a song without a container.
    pub container: Option<u64>,
    /// Ordered by start.
    pub notes: Vec<Note>,
}

#[derive(Clone)]
struct Open {
    phrase: Phrase,
    /// Start and velocity of each sounding pitch.
    sounding: [Option<(u64, u8)>; 128],
    last: u64,
}

impl Open {
    fn stop(&mut self, pitch: u8, at: u64) {
        if let Some((start, velocity)) = self.sounding[usize::from(pitch)].take() {
            self.phrase.notes.push(Note {
                start: start - self.phrase.start,
                length: at - start,
                pitch,
                velocity,
            });
        }
    }
    fn silent(&self) -> bool {
        self.sounding.iter().all(Option::is_none)
    }
    /// Notes left sounding end at the phrase's last key.
    fn close(mut self) -> Phrase {
        for pitch in 0..128u8 {
            self.stop(pitch, self.last);
        }
        self.phrase.notes.sort_by_key(|n| (n.start, n.pitch));
        self.phrase
    }
}

/// The keys of one keyboard session. A phrase ends at a silence of the gap or another
/// container's key.
#[derive(Clone, Default)]
pub struct Keys {
    open: Option<Open>,
    closed: Vec<Phrase>,
}

impl Keys {
    /// `at` is session-relative milliseconds, never earlier than the previous key; a `velocity` of 0
    /// releases the key.
    pub fn key(&mut self, at: u64, pitch: u8, velocity: u8, container: Option<u64>, gap: u64) {
        let on = velocity > 0;
        if let Some(o) = &self.open
            && (o.phrase.container != container || (o.silent() && on && at - o.last >= gap))
        {
            self.end();
        }
        if on && self.open.is_none() {
            self.open = Some(Open {
                phrase: Phrase {
                    start: at,
                    container,
                    notes: vec![],
                },
                sounding: [None; 128],
                last: at,
            });
        }
        // A Note Off before any Note On belongs to a phrase already left.
        let Some(o) = self.open.as_mut() else {
            return;
        };
        o.stop(pitch, at);
        if on {
            o.sounding[usize::from(pitch)] = Some((at, velocity));
        }
        o.last = at;
    }
    fn end(&mut self) {
        let phrase = self.open.take().unwrap().close();
        if !phrase.notes.is_empty() {
            self.closed.push(phrase);
        }
    }
    /// A phrase is open that `gap` milliseconds of silence up to `now` have not ended.
    pub fn playing(&self, now: u64, gap: u64) -> bool {
        self.open
            .as_ref()
            .is_some_and(|o| !o.silent() || now.saturating_sub(o.last) < gap)
    }
    /// Phrases ended by another container's key, or by `gap` milliseconds of silence up to `now`.
    pub fn take(&mut self, now: u64, gap: u64) -> Vec<Phrase> {
        if self.open.is_some() && !self.playing(now, gap) {
            self.end();
        }
        std::mem::take(&mut self.closed)
    }
    /// Every phrase not taken yet, the open one included.
    pub fn finish(&mut self) -> Vec<Phrase> {
        if self.open.is_some() {
            self.end();
        }
        std::mem::take(&mut self.closed)
    }
}

/// Pattern-relative CC1 `controls` and note events of `phrase` from tick 0 at `tempo`
/// microseconds per quarter, and the tick after its last note.
fn events(
    division: u16,
    tempo: u32,
    phrase: &Phrase,
    controls: &[(u64, u8)],
) -> Result<(Vec<Event>, u64), String> {
    if division == 0 || division & 0x8000 != 0 {
        return Err(message::RECORDING_REQUIRES_PPQN_TIMING.into());
    }
    if tempo == 0 {
        return Err(message::RECORDING_REQUIRES_A_POSITIVE_TEMPO.into());
    }
    if phrase.notes.is_empty() {
        return Err(message::RECORDING_IS_EMPTY.into());
    }
    let per_quarter = u128::from(tempo);
    let division = u128::from(division);
    let tick = |ms: u64| {
        u64::try_from((u128::from(ms) * 1000 * division + per_quarter / 2) / per_quarter)
            .map_err(|_| message::RECORDING_TIME_OVERFLOW.to_owned())
    };
    let mut released = [0u64; 128];
    let mut events = vec![];
    let mut end = 0;
    for n in &phrase.notes {
        let p = usize::from(n.pitch);
        // Rounding may pull a repeated key into the note before it.
        let on = tick(n.start)?.max(released[p]);
        let off = tick(n.start + n.length)?.max(on + 1);
        released[p] = off;
        end = end.max(off);
        events.push(note(on, n.pitch, n.velocity.max(1)));
        events.push(note(off, n.pitch, 0));
    }
    for &(at, value) in controls {
        let at = tick(at)?;
        if at < end {
            control(&mut events, at, value);
        }
    }
    // CC1 leads its tick; a Note Off and the next Note On of its pitch on one tick keep that order.
    events.sort_by_key(|e| (e.tick, e.status != 0xb0, e.status == 0x90));
    Ok((events, end))
}

fn source(music: &mut Music, source: Option<usize>) -> Result<usize, String> {
    match source {
        Some(track) => Ok(track),
        None => operations::create_source(music),
    }
}

/// Appends `phrase` and its CC1 `controls` (milliseconds from the phrase start) to the Capture
/// pattern of the container on `source`, creating the pattern (and, for a `None` source, the
/// container). The phrase starts at a bar, one empty bar after the previous one, behind a
/// marker with a song-wide sequence number. Returns the Capture pattern.
pub fn append(
    music: &mut Music,
    source: Option<usize>,
    tempo: u32,
    phrase: &Phrase,
    controls: &[(u64, u8)],
) -> Result<u64, String> {
    let sequence = captured(music)?
        .iter()
        .map(|p| p.sequence)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(message::STORAGE_OVERFLOW)?;
    let (notes, end) = events(music.division, tempo, phrase, controls)?;
    let source = self::source(music, source)?;
    if notes.iter().any(|e| e.status == 0xb0) {
        super::cc1_baseline(music, source)?;
    }
    let measure = u64::from(music.division) * 4;
    let bar = Meter {
        numerator: 4,
        exponent: 2,
        ticks: measure,
    };
    let existing = Catalog::read(music)?
        .patterns
        .values()
        .find(|p| p.track == source && p.kind == Kind::Capture)
        .map(|p| Ok::<_, String>((p.id, p.meters.clone(), p.length()?)))
        .transpose()?;
    let (id, mut meters, offset) = match existing {
        Some((id, mut meters, length)) => {
            meters.push(bar.clone());
            (id, meters, length + measure)
        }
        None => (
            operations::create_as(music, source, Kind::Capture)?,
            vec![],
            0,
        ),
    };
    let count = end.div_ceil(measure).max(1);
    if meters.len() as u64 + count > 100_000 {
        return Err(message::RECORDING_EXCEEDS_THE_PATTERN_MEASURE_LIMIT.into());
    }
    meters.extend(std::iter::repeat_n(bar, count as usize));
    let mut events = vec![Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: MARKER,
        data: format!("Capture {sequence}").into_bytes(),
    }];
    events.extend(notes);
    fill(music, id, meters, offset, events)?;
    Ok(id)
}

const MARKER: u8 = 6;

fn marker(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == MARKER
}

/// Ticks from the start of a phrase in a Capture pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickNote {
    pub start: u64,
    pub length: u64,
    pub pitch: u8,
}

/// One phrase of a Capture pattern: the bars from its marker to its last note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captured {
    pub pattern: u64,
    pub container: u64,
    /// Song-wide capture order, independent of wall-clock time.
    pub sequence: u64,
    /// Ticks from the pattern start: the phrase's first bar and the bar after its last note.
    pub range: (u64, u64),
    /// The meters of those bars.
    pub meters: Vec<Meter>,
    /// Ordered by start.
    pub notes: Vec<TickNote>,
}

impl Captured {
    /// The tick after the last note, from the phrase start.
    pub fn end(&self) -> u64 {
        self.notes
            .iter()
            .map(|n| n.start + n.length)
            .max()
            .unwrap_or(0)
    }
}

/// The phrases of every Capture pattern, newest first.
pub fn captured(music: &Music) -> Result<Vec<Captured>, String> {
    let mut out = vec![];
    for p in Catalog::read(music)?
        .patterns
        .values()
        .filter(|p| p.kind == Kind::Capture)
    {
        let (start, end) = p.range()?;
        let events = &music.tracks[p.track].events;
        let markers: Vec<_> = body::extract(events, (start, end))?
            .into_iter()
            .filter(marker)
            .collect();
        let mut bars = vec![start];
        for m in &p.meters {
            bars.push(bars.last().unwrap() + m.ticks);
        }
        for (i, m) in markers.iter().enumerate() {
            let next = markers.get(i + 1).map_or(end, |n| n.tick);
            let body = body::extract(events, (m.tick, next))?;
            let last = body.iter().map(|e| e.tick).max().unwrap_or(m.tick);
            let first = bars.partition_point(|&b| b <= m.tick).saturating_sub(1);
            let after = bars
                .iter()
                .position(|&b| b > m.tick && b >= last)
                .unwrap_or(bars.len() - 1);
            let mut sounding: Vec<Option<u64>> = vec![None; 128 * 16];
            let mut notes = vec![];
            for e in &body {
                let at = e.tick - bars[first];
                let key = usize::from(e.status & 15) * 128 + usize::from(e.data[0] & 127);
                if note_on(e) {
                    sounding[key] = Some(at);
                } else if note_off(e)
                    && let Some(on) = sounding[key].take()
                {
                    notes.push(TickNote {
                        start: on,
                        length: at - on,
                        pitch: e.data[0],
                    });
                }
            }
            notes.sort_by_key(|n| (n.start, n.pitch));
            out.push(Captured {
                pattern: p.id,
                container: p.container,
                sequence: std::str::from_utf8(&m.data)
                    .ok()
                    .and_then(|s| s.strip_prefix("Capture "))
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|&n| n > 0)
                    .ok_or("Invalid Capture sequence marker")?,
                range: (bars[first] - start, bars[after] - start),
                meters: p.meters[first..after].to_vec(),
                notes,
            });
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse((p.sequence, p.pattern, p.range.0)));
    Ok(out)
}

/// A new pattern of `phrase`'s bars in its container, without the marker.
pub fn take(music: &mut Music, phrase: &Captured) -> Result<u64, String> {
    let c = Catalog::read(music)?;
    let p = c
        .patterns
        .get(&phrase.pattern)
        .filter(|p| p.kind == Kind::Capture)
        .ok_or("phraseのCapture patternが無くなりました")?;
    let (start, end) = (p.tick + phrase.range.0, p.tick + phrase.range.1);
    let events = body::extract(&music.tracks[p.track].events, (start, end))?
        .into_iter()
        .filter(|e| !marker(e))
        .map(|mut e| {
            e.tick -= start;
            e
        })
        .collect();
    let id = operations::create(music, p.track, None)?;
    fill(music, id, phrase.meters.clone(), 0, events)?;
    Ok(id)
}

mod carry;
pub use carry::carry;
mod clear;
pub use clear::clear;

#[cfg(test)]
#[path = "capture_sequence_tests.rs"]
mod sequence_tests;
#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
