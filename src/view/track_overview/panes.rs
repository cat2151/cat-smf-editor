//! Visible track or container rows and their shared performance minimap.
use super::{kind, Kind};
use crate::app::App;
use crate::view::piano_roll::{self, Layout};

/// Unzoomed height of one track's pane.
pub(super) const PANE_HEIGHT: f32 = 60.0;

pub(super) struct Panes {
    /// All visible notes; Song uses the conductor's meter.
    pub overview: Layout,
    /// One roll per displayed row, without the other rows' notes.
    pub layouts: Vec<Layout>,
    pub kinds: Vec<Kind>,
    pub tracks: Vec<usize>,
    pub song: Option<std::sync::Arc<super::song::Song>>,
    pub curves: Vec<Vec<cat_smf_editor::edit::parameter_curve::Curve>>,
}

impl Panes {
    pub(super) fn read(app: &App, tick: u64) -> Option<Self> {
        if super::song::enabled(app) {
            return Self::song(app, tick).ok();
        }
        let loaded = app.loaded.as_ref()?;
        let sources = crate::play::playback::performance(app);
        let tracks: Vec<usize> = (0..sources.len()).collect();
        let overview = piano_roll::layout_from(loaded, &sources, &tracks, app.track, tick);
        let layouts = tracks
            .iter()
            .map(|&track| {
                let mut layout = piano_roll::layout_from(loaded, &sources, &[track], track, tick);
                layout.references.clear();
                piano_roll::panes::fit_height(&mut layout, PANE_HEIGHT);
                layout
            })
            .collect();
        let kinds = tracks.iter().map(|&t| kind(&sources[t].events)).collect();
        Some(Self {
            overview,
            layouts,
            kinds,
            tracks,
            song: None,
            curves: super::curves::read(loaded),
        })
    }

    pub fn song(app: &App, tick: u64) -> Result<Self, String> {
        use crate::play::playback::Sounding;
        use cat_smf_editor::score::note_view::NoteView;
        use std::borrow::Cow;
        let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
        let song = super::song::Song::read(app)?;
        let source = |events: Vec<cat_smf_editor::model::Event>| Sounding {
            durations: Cow::Owned(NoteView::new(&events).durations),
            events: Cow::Owned(events),
            end: song.end,
        };
        let mut sources: Vec<_> = (0..loaded.music.tracks.len())
            .map(|_| source(vec![]))
            .collect();
        for row in &song.rows {
            sources[row.track]
                .events
                .to_mut()
                .extend(row.events.clone());
        }
        for s in &mut sources {
            s.events.to_mut().sort_by_key(|e| e.tick);
            s.durations = Cow::Owned(NoteView::new(&s.events).durations);
        }
        let tracks: Vec<_> = song.rows.iter().map(|r| r.track).collect();
        let all: Vec<_> = (0..sources.len()).collect();
        let mut overview = piano_roll::layout_from(loaded, &sources, &all, 0, tick);
        song_axis(&mut overview, &song, loaded.music.division);
        let mut layouts = vec![];
        let mut kinds = vec![];
        for row in &song.rows {
            let mut one: Vec<_> = (0..sources.len()).map(|_| source(vec![])).collect();
            one[row.track] = source(row.events.clone());
            let mut layout = piano_roll::layout_from(loaded, &one, &[row.track], 0, tick);
            layout.references.clear();
            song_axis(&mut layout, &song, loaded.music.division);
            // Leave room for the pattern name above its notes.
            piano_roll::panes::fit_height(&mut layout, PANE_HEIGHT - 16.0);
            for (_, rect) in &mut layout.notes {
                *rect = rect.translate(eframe::egui::vec2(0.0, 16.0));
            }
            layout.size.y = PANE_HEIGHT;
            layouts.push(layout);
            kinds.push(if row.track == 0 || !row.occurrences.is_empty() {
                Kind::Events
            } else {
                Kind::Empty
            });
        }
        let curves = song.rows.iter().map(|_| vec![]).collect();
        Ok(Self {
            overview,
            layouts,
            kinds,
            tracks,
            song: Some(song),
            curves,
        })
    }
}

fn song_axis(layout: &mut Layout, song: &super::song::Song, division: u16) {
    let scale = layout.scale;
    let end = song.end.max(u64::from(division) * 4);
    layout.size.x = end as f32 * layout.scale + f32::from(division) * layout.scale;
    layout.beats = song
        .time
        .map(0)
        .unwrap()
        .beat_runs(end)
        .iter()
        .flat_map(|run| {
            (0..run.count).map(move |i| {
                let tick = run.tick as f64 + (i * run.step) as f64 / run.denominator as f64;
                (tick as f32 * scale, run.position(i).beat)
            })
        })
        .collect();
}
