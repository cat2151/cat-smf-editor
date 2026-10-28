use super::*;
use crate::score::musical_time::{MeterMap, MusicalTime, Position};

pub struct Measure {
    pub number: u64,
    pub start: u64,
    pub end: u64,
    pub steps: Vec<u64>,
}
impl Measure {
    pub fn new(map: &MeterMap, division: u16, number: u64) -> Result<Self, String> {
        tick(division, 0)?;
        if number == 0 {
            return Err("Measure must be positive".into());
        }
        let at = |n: u64| {
            map.tick_at(&Position {
                measure: u128::from(n),
                beat: 1,
                tick_numerator: 0,
                tick_denominator: 1,
            })
            .ok_or_else(|| "Measure exceeds tick range".to_string())
        };
        let start = at(number)?;
        let end = at(number.checked_add(1).ok_or("Measure overflow")?)?;
        let run = map
            .beat_runs(start)
            .into_iter()
            .rev()
            .find(|r| r.position(0).measure <= u128::from(number))
            .ok_or("Missing measure meter")?;
        let count = (run.beats_per_measure() * 16).div_ceil(run.denominator);
        if count == 0 || count > 4096 {
            return Err("Unsupported measure length".into());
        }
        let beats = (u128::from(number) - run.position(0).measure) * run.beats_per_measure();
        let base = beats
            .checked_mul(run.step)
            .and_then(|v| v.checked_mul(4))
            .ok_or("Drum measure overflow")?;
        let denominator = run.denominator * 4;
        let steps = (0..count)
            .map(|step| {
                let offset = base
                    .checked_add(step * u128::from(division) * run.denominator)
                    .ok_or("Drum step overflow")?
                    .div_ceil(denominator);
                run.tick
                    .checked_add(u64::try_from(offset).map_err(|_| "Drum step overflow")?)
                    .ok_or("Drum step overflow".into())
            })
            .collect::<Result<Vec<_>, String>>()?
            .into_iter()
            .take_while(|&tick| tick < end)
            .collect();
        Ok(Self {
            number,
            start,
            end,
            steps,
        })
    }
    pub fn active(&self, track: &Track, row: usize, step: usize) -> bool {
        let Some(&start) = self.steps.get(step) else {
            return false;
        };
        let end = self.steps.get(step + 1).copied().unwrap_or(self.end);
        track
            .events
            .iter()
            .any(|e| note_on(e) && e.data[0] == NOTES[row] && (start..end).contains(&e.tick))
    }
}

pub fn toggle_measure(
    music: &mut crate::model::Music,
    track: usize,
    row: usize,
    number: u64,
    step: usize,
) -> Result<(), String> {
    let time = MusicalTime::new(music);
    let measure = Measure::new(
        time.map(track).ok_or("Missing measure map")?,
        music.division,
        number,
    )?;
    let start = *measure.steps.get(step).ok_or("Invalid drum step")?;
    let end = measure.steps.get(step + 1).copied().unwrap_or(measure.end);
    let pitch = *NOTES.get(row).ok_or("Invalid drum row")?;
    let source = music.tracks.get_mut(track).ok_or("Missing drum track")?;
    let remove: Vec<_> = source
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            (note_on(e) && e.data[0] == pitch && (start..end).contains(&e.tick)).then_some(i)
        })
        .collect();
    if remove.is_empty() {
        edit::insert(
            source,
            Event {
                tick: start,
                delta: 0,
                status: 0x99,
                meta_type: 0,
                data: vec![pitch, 100],
            },
        );
        edit::insert(
            source,
            Event {
                tick: end,
                delta: 0,
                status: 0x89,
                meta_type: 0,
                data: vec![pitch, 0],
            },
        );
    } else {
        edit::event_range::delete(source, &remove, true)?;
    }
    rebuild(music)
}

/// GM onsets and velocities are authoritative; releases follow the next hit in that row or bar end.
pub fn rebuild(music: &mut crate::model::Music) -> Result<(), String> {
    let time = MusicalTime::new(music);
    let bodies = (0..music.tracks.len())
        .filter(|&t| {
            crate::score::pattern::is_source(music, t)
                || crate::score::performance::is_owner(&music.tracks[t])
        })
        .collect::<std::collections::BTreeSet<_>>();
    for (track, source) in music.tracks.iter_mut().enumerate() {
        if bodies.contains(&track) || read(source)?.is_none() {
            continue;
        }
        let map = time.map(track).ok_or("Missing drum measure map")?;
        let pairs = edit::relocate::pairs(source.events.iter());
        let mut releases = Vec::new();
        for pitch in NOTES {
            let ons: Vec<_> = source
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| note_on(e) && e.data[0] == pitch)
                .collect();
            for (position, (index, on)) in ons.iter().enumerate() {
                let number =
                    u64::try_from(map.position(on.tick).measure).map_err(|_| "Measure overflow")?;
                let measure = Measure::new(map, music.division, number)?;
                let end = ons[position + 1..]
                    .iter()
                    .find(|(_, e)| e.tick > on.tick)
                    .map_or(measure.end, |(_, e)| e.tick.min(measure.end));
                releases.push((pairs[*index].ok_or("Drum note has no Note Off")?, end));
            }
        }
        for (off, end) in releases {
            source.events[off].tick = end;
        }
        let eot = source
            .events
            .pop_if(|e| e.status == 0xff && e.meta_type == 0x2f);
        source.events.sort_by_key(|e| {
            (
                e.tick,
                if crate::score::note_view::note_off(e)
                    && e.data.first().is_some_and(|p| NOTES.contains(p))
                {
                    0
                } else if note_on(e) && NOTES.contains(&e.data[0]) {
                    2
                } else {
                    1
                },
            )
        });
        if let Some(mut eot) = eot {
            eot.tick = eot.tick.max(source.events.last().map_or(0, |e| e.tick));
            source.events.push(eot);
        }
        edit::refresh_deltas(source)?;
    }
    Ok(())
}
