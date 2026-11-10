//! A captured phrase replayed lap after lap on the audio thread, in front of the live keys.
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Phrase {
    inner: Mutex<Option<Laps>>,
}

/// Offsets within a lap of each pitch and velocity (0 for Note Off).
type Notes = Vec<(Duration, u8, u8)>;

struct Laps {
    /// Where lap 0 begins.
    start: Instant,
    length: Duration,
    /// Offsets within a lap, ordered; at one offset Note Off precedes Note On.
    notes: Vec<(Duration, u8, u8)>,
    /// Takes the place of `notes` from the first lap beginning at or after its time past
    /// `start`.
    next: Option<(Duration, Notes)>,
    /// Lap time already emitted, measured from `start`.
    done: Duration,
    sounding: [bool; 128],
}

impl Phrase {
    /// Repeats `notes` every `length` from `start`; an Off at `length` ends its lap.
    pub(crate) fn play(
        &self,
        start: Instant,
        length: Duration,
        mut notes: Vec<(Duration, u8, u8)>,
    ) {
        notes.sort_by_key(|&(at, _, velocity)| (at, velocity > 0));
        *self.inner.lock().unwrap() = (!length.is_zero()).then_some(Laps {
            start,
            length,
            notes,
            next: None,
            done: Duration::ZERO,
            sounding: [false; 128],
        });
    }

    /// From the next lap on, repeats `notes` in place of the phrase; what it left sounding
    /// stops there.
    pub(crate) fn replace(&self, notes: Vec<(Duration, u8, u8)>) {
        self.replace_from(Duration::ZERO, notes);
    }

    /// As `replace`, from the first lap beginning at or after `from` past where lap 0 begins.
    pub(crate) fn replace_from(&self, from: Duration, mut notes: Vec<(Duration, u8, u8)>) {
        notes.sort_by_key(|&(at, _, velocity)| (at, velocity > 0));
        if let Some(l) = self.inner.lock().unwrap().as_mut() {
            l.next = Some((from, notes));
        }
    }

    pub(crate) fn playing(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    /// Stops repeating; returns the pitches the phrase left sounding.
    pub(crate) fn stop(&self) -> Vec<u8> {
        self.inner.lock().unwrap().take().map_or(vec![], |l| {
            (0..128u8).filter(|&p| l.sounding[usize::from(p)]).collect()
        })
    }

    /// Writes the live words due by `now` into `out`; returns how many.
    pub(crate) fn due(&self, now: Instant, out: &mut [u32]) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let Some(l) = inner.as_mut() else {
            return 0;
        };
        let until = now.saturating_duration_since(l.start);
        if until <= l.done {
            return 0;
        }
        let length = l.length.as_nanos();
        // An Off at `length` belongs to the previous lap, so that lap is read again.
        let mut lap = (l.done.as_nanos() / length).saturating_sub(1);
        let mut count = 0;
        while lap * length < until.as_nanos() {
            let begin = lap * length;
            if begin >= l.done.as_nanos()
                && l.next
                    .as_ref()
                    .is_some_and(|(from, _)| begin >= from.as_nanos())
                && let Some((from, next)) = l.next.take()
            {
                let left: Vec<u8> = (0..128u8).filter(|&p| l.sounding[usize::from(p)]).collect();
                if count + left.len() > out.len() {
                    l.next = Some((from, next));
                    l.done = nanos(begin);
                    return count;
                }
                for pitch in left {
                    out[count] = super::live::word(pitch, 0);
                    l.sounding[usize::from(pitch)] = false;
                    count += 1;
                }
                l.notes = next;
            }
            for &(offset, pitch, velocity) in &l.notes {
                let at = lap * length + offset.as_nanos();
                if at < l.done.as_nanos() || at >= until.as_nanos() {
                    continue;
                }
                if count == out.len() {
                    l.done = nanos(at);
                    return count;
                }
                out[count] = super::live::word(pitch, velocity);
                l.sounding[usize::from(pitch)] = velocity > 0;
                count += 1;
            }
            lap += 1;
        }
        l.done = until;
        count
    }
}

fn nanos(n: u128) -> Duration {
    Duration::new((n / 1_000_000_000) as u64, (n % 1_000_000_000) as u32)
}

#[cfg(test)]
#[path = "audition_phrase_tests.rs"]
mod tests;
