//! One expansion and control-occupancy contract for pattern, Scene, Song and export.
use super::*;
use crate::edit::parameter_curve::{Curve, Target};
use automation::Baseline;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug)]
pub struct ParameterInterval {
    pub start: u64,
    pub end: u64,
    pub baseline: f64,
    pub curve: Curve,
}
#[derive(Default)]
pub struct Expansion {
    pub events: BTreeMap<usize, Vec<Event>>,
    pub parameters: BTreeMap<usize, Vec<ParameterInterval>>,
    pub end: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Control {
    Parameter(Target),
    Midi(u64, u8, Option<u8>),
}
fn midi_control(e: &Event, container: u64) -> Result<Option<Control>, String> {
    let status = e.status & 0xf0;
    if matches!(status, 0xa0 | 0xd0) {
        return Err("patternのaftertouch演奏には対応していません。内容はそのまま残します".into());
    }
    if !matches!(status, 0xb0 | 0xc0 | 0xe0) {
        return Ok(None);
    }
    if status == 0xc0 {
        return Err("Program Changeではpattern containerのsoundを変えられません".into());
    }
    if e.data.len() != 2 || e.data.iter().any(|v| *v > 127) {
        return Err("patternのMIDI controlが壊れています".into());
    }
    let controller = if status == 0xb0 {
        let cc = e.data[0];
        if !automation::supported_cc(cc) {
            return Err(format!(
                "CC {cc}は対応していない複合/channelの状態遷移を要します。内容はそのまま残します"
            ));
        }
        Some(cc)
    } else {
        None
    };
    Ok(Some(Control::Midi(container, e.status & 15, controller)))
}
fn midi_event(tick: u64, channel: u8, controller: Option<u8>, value: u16) -> Event {
    let (status, data) = if let Some(cc) = controller {
        (0xb0 | channel, vec![cc, value as u8])
    } else {
        (
            0xe0 | channel,
            vec![(value & 127) as u8, (value >> 7) as u8],
        )
    };
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data,
    }
}
pub fn expand(
    m: &Music,
    c: &Catalog,
    occurrences: &[(u64, u64)],
    end: u64,
) -> Result<Expansion, String> {
    let mut result = Expansion {
        end,
        ..Default::default()
    };
    let mut occurrences = occurrences.to_vec();
    occurrences.sort_by_key(|&(id, start)| (start, id));
    let mut occupied = BTreeMap::<Control, Vec<(u64, u64)>>::new();
    let mut midi_intervals = vec![];
    for (id, start) in occurrences {
        let p = c
            .patterns
            .get(&id)
            .ok_or("patternの出現箇所が見つかりません")?;
        let until = start
            .checked_add(p.length()?)
            .ok_or("patternの出現箇所が上限を超えました")?;
        if until > end {
            return Err("patternの出現箇所が宣言した長さを超えています".into());
        }
        let body = body::extract(&m.tracks[p.track].events, p.range()?)?;
        let baselines = automation::baselines(m, p.track)?;
        let mut controls = BTreeSet::new();
        for curve in automation::curves(m, p)? {
            let baseline = baselines
                .iter()
                .find_map(|b| match b {
                    Baseline::Parameter { target, value, .. } if target == &curve.target => {
                        Some(*value)
                    }
                    _ => None,
                })
                .ok_or("containerのparameter基準値が見つかりません")?;
            controls.insert(Control::Parameter(curve.target.clone()));
            result
                .parameters
                .entry(p.track)
                .or_default()
                .push(ParameterInterval {
                    start,
                    end: until,
                    baseline,
                    curve,
                });
        }
        let output = result.events.entry(p.track).or_default();
        for e in &body {
            if let Some(control) = midi_control(e, p.container)? {
                controls.insert(control);
            }
            if matches!(e.status, 0xf0 | 0xf7) {
                return Err(
                    "patternのSysEx演奏には対応していません。内容はそのまま残します".into(),
                );
            }
            if automation::owned(e) {
                continue;
            }
            let mut e = e.clone();
            e.tick = start
                .checked_add(e.tick - p.tick)
                .ok_or("patternのeventが上限を超えました")?;
            output.push(e);
        }
        for control in controls {
            let intervals = occupied.entry(control.clone()).or_default();
            if intervals.iter().any(|&(a, b)| start < b && until > a) {
                return Err(format!("patternのcontrolが重なっています {control:?}"));
            }
            intervals.push((start, until));
            if let Control::Midi(_, channel, controller) = control {
                let value = baselines
                    .iter()
                    .find_map(|b| match b {
                        Baseline::Midi {
                            channel: c,
                            controller: cc,
                            value,
                        } if *c == channel && *cc == controller => Some(*value),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        format!(
                            "channel {} {}のcontainer基準値が見つかりません",
                            channel + 1,
                            controller.map_or("pitch bend".into(), |cc| format!("CC {cc}"))
                        )
                    })?;
                let at_start = body.iter().any(|e| {
                    e.tick == p.tick
                        && midi_control(e, p.container).ok().flatten()
                            == Some(Control::Midi(p.container, channel, controller))
                });
                midi_intervals.push((
                    p.track,
                    p.container,
                    start,
                    until,
                    channel,
                    controller,
                    value,
                    at_start,
                ));
            }
        }
    }
    for &(track, container, start, end, channel, controller, value, at_start) in &midi_intervals {
        let output = result.events.entry(track).or_default();
        if !at_start {
            output.insert(0, midi_event(start, channel, controller, value));
        }
        let followed = midi_intervals
            .iter()
            .any(|&(_, owner, next, _, ch, cc, _, _)| {
                owner == container && next == end && ch == channel && cc == controller
            });
        if !followed {
            output.push(midi_event(end, channel, controller, value));
        }
    }
    for events in result.events.values_mut() {
        events.sort_by_key(|e| e.tick);
    }
    Ok(result)
}
