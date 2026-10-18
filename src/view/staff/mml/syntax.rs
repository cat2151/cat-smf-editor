use super::{Note, Token, Tokens, Tone};
use std::collections::BTreeMap;

/// Only ordinary note values and tuplets, optionally dotted. MIDI timing is
/// approximated for reading: never emit foreign dialects' tick lengths or ties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Length {
    pub(super) denominator: u16,
    pub(super) dotted: bool,
}

pub(super) fn length(ticks: u64, ppqn: u16) -> Length {
    approximate(ticks, ppqn, false)
}

fn rest_length(ticks: u64, ppqn: u16) -> Length {
    approximate(ticks, ppqn, true)
}

fn approximate(ticks: u64, ppqn: u16, rest: bool) -> Length {
    let duration = |value: &Length| {
        f64::from(ppqn.max(1)) * 4.0 / f64::from(value.denominator)
            * if value.dotted { 1.5 } else { 1.0 }
    };
    [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128]
        .into_iter()
        .flat_map(|denominator| {
            [false, true].map(move |dotted| Length {
                denominator,
                dotted,
            })
        })
        // A partial-measure rest must not round up to a whole rest before a note.
        .filter(|value| !rest || duration(value) <= ticks as f64)
        .min_by(|a, b| {
            let error = |value: &Length| (duration(value) - ticks as f64).abs();
            error(a).total_cmp(&error(b)).then(a.dotted.cmp(&b.dotted))
        })
        .unwrap()
}

pub(super) fn cell(notes: &[&Note], start: u64, end: u64, ppqn: u16) -> Tokens {
    if notes.is_empty() {
        return Vec::new();
    }
    let rest_min = u64::from(ppqn.max(1)).div_ceil(2);
    let mut parts = Vec::new();
    let mut counts = BTreeMap::new();
    let mut tick = start;
    let rest = |tick, ticks| {
        (
            vec![Token {
                text: "r".into(),
                tone: None,
                tick,
            }],
            rest_length(ticks, ppqn),
            false,
        )
    };
    for group in notes.chunk_by(|a, b| a.tick == b.tick && a.end == b.end) {
        let note = group[0];
        let onset = note.tick.max(start);
        if onset.saturating_sub(tick) >= rest_min {
            parts.push(rest(tick, onset - tick));
        }
        let mut names = std::collections::BTreeSet::new();
        let distinct: Vec<_> = group
            .iter()
            .filter(|n| names.insert((n.spelled.step.rem_euclid(7), n.spelled.alter)))
            .collect();
        let chord = distinct.len() >= 3;
        let mut tokens = Vec::new();
        let mut octave = distinct[0].spelled.step.div_euclid(7);
        for (i, note) in distinct.iter().enumerate() {
            let mut text = String::new();
            if chord && i == 0 {
                text.push(' ');
                text.push('\'');
            }
            if chord {
                let next = note.spelled.step.div_euclid(7);
                let delta = next - octave;
                text.extend(std::iter::repeat_n(
                    if delta > 0 { '<' } else { '>' },
                    delta.unsigned_abs() as usize,
                ));
                octave = next;
            }
            text.push(b"cdefgab"[note.spelled.step.rem_euclid(7) as usize] as char);
            text.push_str(match note.spelled.alter {
                -1 => "-",
                1 => "+",
                _ => "",
            });
            tokens.push(Token {
                text,
                tone: Some(Tone {
                    color: note.color,
                    start: note.tick,
                    end: note.end,
                }),
                tick: onset,
            });
        }
        tick = note.end.min(end);
        let value = length(tick - onset, ppqn);
        // Put a following rest after the displayed note value. A shortened
        // performance gate must not put the next beat's rest in this beat.
        let displayed = u64::from(ppqn.max(1)) * 4 / u64::from(value.denominator);
        let displayed = if value.dotted {
            displayed * 3 / 2
        } else {
            displayed
        };
        tick = tick.max(onset.saturating_add(displayed).min(end));
        if !chord {
            *counts.entry(value).or_insert(0) += 1;
        }
        parts.push((tokens, value, chord));
    }
    if end.saturating_sub(tick) >= rest_min {
        parts.push(rest(tick, end - tick));
    }
    let default = counts
        .into_iter()
        .max_by_key(|&(value, count)| (count, value.denominator, !value.dotted))
        .map(|(value, _)| value.denominator)
        .filter(|default| {
            let digits = default.to_string().len();
            let matching = parts
                .iter()
                .filter(|(_, value, chord)| !chord && value.denominator == *default)
                .count();
            matching * digits >= digits + 2
        });
    let mut tokens = Vec::new();
    if let Some(default) = default {
        tokens.push(Token {
            text: format!("l{default} "),
            tone: None,
            tick: start,
        });
    }
    for (mut part, value, chord) in parts {
        let text = &mut part.last_mut().unwrap().text;
        if chord || Some(value.denominator) != default {
            text.push_str(&value.denominator.to_string());
        }
        if value.dotted {
            text.push('.');
        }
        if chord {
            text.push('\'');
            text.push(' ');
        }
        tokens.extend(part);
    }
    tokens
}
