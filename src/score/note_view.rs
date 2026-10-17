//! A presentation index; original MIDI events are never modified.
use crate::model::Event;
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct NoteView {
    pub rows: Vec<usize>,
    pub durations: Vec<Option<u64>>,
    /// Note On source indices, grouped on the last visible row at each tick.
    pub keyboard_notes: Vec<Vec<usize>>,
    /// Note Ons still sounding at the next later Note On.
    pub overlaps: Vec<bool>,
    /// The first Note On, when every Note On shares its velocity.
    pub uniform_velocity_head: Option<usize>,
}

pub fn note_on(event: &Event) -> bool {
    event.status & 0xf0 == 0x90 && event.data.get(1).is_some_and(|v| *v > 0)
}

pub fn note_off(event: &Event) -> bool {
    event.status & 0xf0 == 0x80 || (event.status & 0xf0 == 0x90 && event.data.get(1) == Some(&0))
}

pub fn note_name(note: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        names[usize::from(note % 12)],
        i16::from(note) / 12 - 1
    )
}

impl NoteView {
    pub fn new(events: &[Event]) -> Self {
        let mut pending: HashMap<(u8, u8), VecDeque<usize>> = HashMap::new();
        let mut view = Self {
            rows: Vec::with_capacity(events.len()),
            durations: vec![None; events.len()],
            keyboard_notes: vec![Vec::new(); events.len()],
            overlaps: vec![false; events.len()],
            uniform_velocity_head: None,
        };
        for (index, event) in events.iter().enumerate() {
            if let Some(&note) = event.data.first() {
                let key = (event.status & 15, note);
                if note_on(event) {
                    pending.entry(key).or_default().push_back(index);
                } else if note_off(event)
                    && let Some(on) = pending.get_mut(&key).and_then(VecDeque::pop_front)
                    && let Some(duration) = event.tick.checked_sub(events[on].tick)
                {
                    view.durations[on] = Some(duration);
                    continue;
                }
            }
            view.rows.push(index);
        }
        let mut start = 0;
        while start < view.rows.len() {
            let tick = events[view.rows[start]].tick;
            let mut end = start + 1;
            while end < view.rows.len() && events[view.rows[end]].tick == tick {
                end += 1;
            }
            view.keyboard_notes[view.rows[end - 1]] = view.rows[start..end]
                .iter()
                .copied()
                .filter(|&index| note_on(&events[index]))
                .collect();
            start = end;
        }
        let ons: Vec<usize> = (0..events.len()).filter(|&i| note_on(&events[i])).collect();
        let mut current = None;
        let mut following = None;
        for &i in ons.iter().rev() {
            let tick = events[i].tick;
            if current != Some(tick) {
                following = current;
                current = Some(tick);
            }
            view.overlaps[i] = following
                .zip(view.durations[i])
                .is_some_and(|(next, duration)| tick.saturating_add(duration) > next);
        }
        view.uniform_velocity_head = ons.first().copied().filter(|&head| {
            ons.iter()
                .all(|&i| events[i].data[1] == events[head].data[1])
        });
        view
    }
}
