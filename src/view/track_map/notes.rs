//! The notes the map draws, as playback sounds them: its own track's and, dimmed, every other track's.
use crate::app::App;
use cat_smf_editor::score::note_view::note_on;

/// Note On tick, length and pitch.
pub(crate) type Mark = (u64, Option<u64>, u8);

pub(crate) struct Notes {
    pub own: Vec<Mark>,
    pub others: Vec<Mark>,
    /// Last tick of the track's performance.
    pub end: u64,
}

impl Notes {
    pub(crate) fn read(app: &App, track: usize) -> Self {
        let sources = crate::play::playback::performance(app);
        if track >= sources.len() {
            return Self {
                own: vec![],
                others: vec![],
                end: 0,
            };
        }
        let marks = |t: usize| -> Vec<Mark> {
            let source = &sources[t];
            source
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| note_on(e))
                .map(|(i, e)| (e.tick, source.durations[i], e.data[0]))
                .collect()
        };
        Self {
            own: marks(track),
            others: (0..sources.len())
                .filter(|&t| t != track)
                .flat_map(marks)
                .collect(),
            end: sources[track]
                .events
                .last()
                .map_or(0, |e| e.tick)
                .max(sources[track].end),
        }
    }

    /// Lowest and highest pitch of the track's own notes.
    pub(crate) fn own_range(&self) -> Option<(u8, u8)> {
        range(&self.own)
    }

    /// A track without notes, such as the conductor, spans every track's notes.
    pub(crate) fn range(&self) -> Option<(u8, u8)> {
        self.own_range().or_else(|| range(&self.others))
    }
}

fn range(marks: &[Mark]) -> Option<(u8, u8)> {
    let pitches = marks.iter().map(|m| m.2);
    Some((pitches.clone().min()?, pitches.max()?))
}
