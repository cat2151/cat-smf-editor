//! Lossless body extraction: actual note offs, release velocity and equal-tick order.
use super::*;
use crate::message;
use crate::{
    model::Track,
    score::{
        musical_time::MusicalTime,
        note_view::{note_off, note_on},
        same_measure,
    },
};
use std::collections::{BTreeMap, VecDeque};

pub fn length(meters: &[Meter]) -> Result<u64, String> {
    if meters.is_empty() {
        return Err("patternには長さの指定が必要です".into());
    }
    meters.iter().try_fold(0u64, |n, m| {
        n.checked_add(m.ticks)
            .ok_or("patternの長さが上限を超えました".into())
    })
}
pub fn meter_music(meters: &[Meter], division: u16) -> Result<Music, String> {
    length(meters)?;
    let mut tick = 0;
    let mut events = vec![];
    for m in meters {
        if m.numerator == 0 || m.exponent > 63 || m.ticks == 0 {
            return Err("patternの拍子が不正です".into());
        }
        events.push(Event {
            tick,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![m.numerator, m.exponent, 24, 8],
        });
        tick += m.ticks;
    }
    let music = Music {
        format: 1,
        division,
        tracks: vec![Track { events }],
    };
    let time = MusicalTime::new(&music);
    let map = time.map(0).ok_or(message::PATTERN_REQUIRES_PPQN_TIMING)?;
    let mut start = 0;
    for (i, m) in meters.iter().enumerate() {
        if same_measure::bounds(map, i as u128 + 1)? != (start, start + m.ticks) {
            return Err("patternの拍子が長さと合いません".into());
        }
        start += m.ticks;
    }
    Ok(music)
}
/// Body events include channel expressions and opaque text/meta, never track settings.
pub fn expression(e: &Event) -> bool {
    super::automation::owned(e)
        || e.status < 0xf0
        || matches!(e.status, 0xf0 | 0xf7)
        || (e.status == 0xff && matches!(e.meta_type, 1 | 5 | 6 | 7))
}
pub fn extract(events: &[Event], (start, end): (u64, u64)) -> Result<Vec<Event>, String> {
    if start >= end {
        return Err("本体の範囲が不正です".into());
    }
    let mut pending = BTreeMap::<(u8, u8), VecDeque<u64>>::new();
    let mut out = vec![];
    for e in events {
        if note_on(e) || note_off(e) {
            let pitch = *e.data.first().ok_or("patternのnoteが壊れています")?;
            let key = (e.status & 15, pitch);
            if note_on(e) {
                let inside = (start..end).contains(&e.tick);
                pending.entry(key).or_default().push_back(e.tick);
                if inside {
                    out.push(e.clone());
                }
            } else {
                let onset = pending.get_mut(&key).and_then(VecDeque::pop_front);
                if onset.is_some_and(|tick| (start..end).contains(&tick)) {
                    if e.tick > end {
                        return Err("noteがpatternの境界をまたいでいます".into());
                    }
                    out.push(e.clone());
                } else if (start..end).contains(&e.tick)
                    || onset.is_some_and(|on| on < start && e.tick > start)
                {
                    return Err("patternに対応の無いnote offか持ち越したnote offがあります".into());
                }
            }
        } else if expression(e) && (start..end).contains(&e.tick) {
            out.push(e.clone());
        }
    }
    if pending.values().any(|q| {
        q.iter()
            .any(|tick| (start..end).contains(tick) || *tick < start)
    }) {
        return Err("patternに終わりの無いnoteがあります".into());
    }
    Ok(out)
}

pub fn parse_meters(text: &str, division: u16) -> Result<Vec<Meter>, String> {
    let mut meters = vec![];
    for word in text.split_whitespace() {
        let (a, b) = word
            .split_once('/')
            .ok_or("拍子を空白区切りの分数で入力してください")?;
        let numerator = a
            .parse::<u8>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("分子が不正です")?;
        let denominator = b
            .parse::<u64>()
            .ok()
            .filter(|n| n.is_power_of_two())
            .ok_or("分母が不正です")?;
        let units = u64::from(division) * 4 * u64::from(numerator);
        if units % denominator != 0 || units / denominator == 0 {
            return Err("拍子を整数のtickで表せません".into());
        }
        meters.push(Meter {
            numerator,
            exponent: denominator.trailing_zeros() as u8,
            ticks: units / denominator,
        });
    }
    if meters.is_empty() || meters.len() > 100_000 {
        return Err("小節数が不正です".into());
    }
    Ok(meters)
}
