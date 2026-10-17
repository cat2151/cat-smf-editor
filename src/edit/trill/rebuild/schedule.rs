//! Forward scheduling under the exact piecewise-constant tempo that SMF exports.
//! Generated On/Off positions themselves become curve sampling positions.
use super::*;
use crate::edit::tempo_curve::microseconds;

const MAX_NOTES: usize = 1_000_000;

pub(super) fn run(music: &Music, curve: Option<&Curve>, plans: &mut [Plan]) -> Result<(), String> {
    let mut tempos = BTreeMap::from([(0, 500_000u32)]);
    let mut fixed = BTreeSet::from([0]);
    for (ti, track) in music.tracks.iter().enumerate() {
        for e in &track.events {
            if note_on(e) || note_off(e) {
                fixed.insert(e.tick);
            }
            if e.status == 0xff && e.meta_type == 0x51 {
                if curve.is_some() {
                    if ti != 0 {
                        return Err(
                            "Trill tempo curve requires tempos only in the conductor".into()
                        );
                    }
                    continue;
                }
                let [a, b, c] = e.data.as_slice() else {
                    return Err("Invalid trill tempo event".into());
                };
                let value = u32::from_be_bytes([0, *a, *b, *c]);
                if value == 0 {
                    return Err("Invalid zero trill tempo".into());
                }
                tempos.insert(e.tick, value);
                fixed.insert(e.tick);
            }
        }
    }
    for plan in plans.iter() {
        fixed.extend([plan.region.start, plan.region.end]);
    }
    let mut fixed = fixed.into_iter().peekable();
    let mut tick = 0u64;
    let mut seconds = 0.0;
    let mut count = 0;
    loop {
        while fixed.peek() == Some(&tick) {
            fixed.next();
        }
        for plan in plans.iter_mut().filter(|p| p.region.start == tick) {
            plan.start_seconds = seconds;
            plan.onsets.push(tick);
            count += 1;
        }
        let tempo = curve.map_or_else(
            || *tempos.range(..=tick).next_back().unwrap().1,
            |c| microseconds(c.bpm_at(tick as f64)),
        );
        let slope = f64::from(tempo) / 1_000_000.0 / f64::from(music.division);
        let mut next = fixed.peek().copied();
        let mut due = Vec::new();
        for (i, plan) in plans
            .iter_mut()
            .enumerate()
            .filter(|(_, p)| p.region.start <= tick && tick < p.region.end)
        {
            let mut target =
                plan.start_seconds + plan.region.metadata.onset_seconds(plan.onsets.len());
            let mut distance = (target - seconds) / slope;
            if distance.round() < 1.0 {
                // Another voice's boundary may make this tick the nearest
                // representable position under its new tempo.
                if plan.onsets.last() == Some(&tick) {
                    return Err(
                        "Trill interval is shorter than the available tick resolution".into(),
                    );
                }
                plan.onsets.push(tick);
                count += 1;
                target = plan.start_seconds + plan.region.metadata.onset_seconds(plan.onsets.len());
                distance = (target - seconds) / slope;
                if distance.round() < 1.0 {
                    return Err(
                        "Trill interval is shorter than the available tick resolution".into(),
                    );
                }
            }
            // Beyond this region there is no next onset. Avoid float->u64 overflow.
            if distance >= (plan.region.end - tick) as f64 {
                continue;
            }
            let candidate = tick + distance.round() as u64;
            if candidate >= plan.region.end {
                continue;
            }
            due.push((i, candidate));
            next = Some(next.map_or(candidate, |n| n.min(candidate)));
        }
        let Some(next) = next else {
            break;
        };
        seconds += (next - tick) as f64 * slope;
        tick = next;
        for (i, candidate) in due {
            if candidate == tick {
                plans[i].onsets.push(tick);
                count += 1;
            }
        }
        if count > MAX_NOTES {
            return Err("Trill regeneration exceeds one million notes".into());
        }
    }
    Ok(())
}
