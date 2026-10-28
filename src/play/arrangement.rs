//! Scene/Song rendering shares the pattern's sound resolver with single preview.
#[cfg(test)]
mod ownership_tests;
use crate::{
    app::App,
    play::{
        audio_sequence::{Note, Sequence},
        playback::Mode,
        transport::{Active, Origin, Prepared},
    },
};
use cat_smf_editor::score::{
    audition_timing::TempoMap,
    note_view::{note_on, NoteView},
    pattern::Catalog,
    scene::arrangement,
};
pub fn prepare(a: &App, scene: Option<u64>) -> Result<Prepared, String> {
    let music = &a.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    let (occurrences, end) = arrangement::occurrences(music, scene)?;
    prepare_occurrences(
        a,
        &occurrences,
        end,
        tempo(a, scene)?,
        Mode::Arrangement(scene),
        0,
    )
}
fn tempo(a: &App, scene: Option<u64>) -> Result<TempoMap, String> {
    let music = &a.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    if scene.is_some() {
        TempoMap::constant(music.division, a.patterns.tempo.0)
    } else {
        Ok(TempoMap::new(music, 0))
    }
}
pub(crate) fn prepare_occurrences(
    a: &App,
    occurrences: &[(u64, u64)],
    end_tick: u64,
    tempo: TempoMap,
    mode: Mode,
    clock_track: usize,
) -> Result<Prepared, String> {
    let loaded = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let c = Catalog::read(&loaded.music)?;
    let expansion =
        cat_smf_editor::score::pattern::render::expand(&loaded.music, &c, occurrences, end_tick)?;
    let end = tempo.seconds_at(expansion.end);
    let mut parts = vec![];
    for (&track, events) in &expansion.events {
        let patch = loaded
            .patches
            .get(track)
            .ok_or("Missing container sound")?
            .clone();
        let view = NoteView::new(events);
        let selected = events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| note_on(e).then_some(i))
            .collect::<Vec<_>>();
        let notes = selected
            .iter()
            .map(|&i| {
                let e = &events[i];
                Note {
                    start: tempo.seconds_at(e.tick),
                    duration: tempo.duration(e.tick, view.durations[i].unwrap_or(0)),
                    pitch: e.data[0],
                    velocity: e.data[1],
                }
            })
            .collect();
        parts.push((
            track,
            patch,
            Sequence::from_source(notes, end, events, &selected, &tempo, 0),
        ));
    }
    if parts.is_empty() {
        parts.push((0, Default::default(), Sequence::new(vec![], end)));
    }
    let automation = super::automation::pattern::prepare(
        a,
        &parts,
        &expansion.parameters,
        &expansion.events,
        &tempo,
    )?;
    Ok(Prepared {
        active: Active {
            automation,
            mode,
            clock_track,
            tempos: vec![tempo.clone(); loaded.music.tracks.len()],
            start_seconds: 0.0,
            end,
            sources: parts.iter().map(|(t, _, s)| (*t, s.clone())).collect(),
        },
        parts,
        origin: Some(Origin {
            events: expansion.events,
            tempo,
        }),
    })
}
pub fn start(a: &mut App, scene: Option<u64>) {
    crate::play::transport::stop(a);
    begin(a, prepare(a, scene));
}
/// Plays from `measure` to the end; a measure past the end plays nothing.
pub fn start_at(a: &mut App, scene: Option<u64>, measure: u64) {
    let tick = a
        .loaded
        .as_ref()
        .ok_or_else(|| "No SMF is loaded".to_string())
        .and_then(|l| arrangement::measure_tick(&l.music, scene, measure));
    match tick {
        Ok(tick) => start_from(a, scene, tick),
        Err(e) => {
            crate::play::transport::stop(a);
            begin(a, Err(e));
        }
    }
}
/// Plays from `tick` to the end; a tick past the end plays nothing.
pub fn start_from(a: &mut App, scene: Option<u64>, tick: u64) {
    crate::play::transport::stop(a);
    let p = tempo(a, scene).map(|t| remaining(a, scene, t.seconds_at(tick), f64::INFINITY));
    match p {
        Ok(Some(p)) => begin(a, Ok(p)),
        Ok(None) => {}
        Err(e) => begin(a, Err(e)),
    }
}
fn begin(a: &mut App, p: Result<Prepared, String>) {
    match p {
        Ok(p) => {
            a.playback.follow = false;
            a.playback.last = p.active.mode;
            a.playback.pending = Some(p);
        }
        Err(e) => a.load_error = Some(e),
    }
}
pub fn remaining(a: &App, scene: Option<u64>, start: f64, duration: f64) -> Option<Prepared> {
    let mut p = prepare(a, scene).ok()?;
    let duration = duration.min(p.active.end - start);
    if start < 0.0 || duration <= 0.0 {
        return None;
    }
    for (_, _, s) in &mut p.parts {
        s.skip(start);
        s.truncate(duration);
    }
    p.active.start_seconds = start;
    p.active.end = duration;
    p.active.sources = p.parts.iter().map(|(t, _, s)| (*t, s.clone())).collect();
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_smf_editor::score::{
        pattern::operations,
        scene::arrangement::{self as grid, PatternOrScene},
    };
    #[test]
    fn arrangement_two_patterns_keep_distinct_owned_sounds_and_global_tempo() {
        let (mut a, source, id) = crate::view::patterns::edit_tests::fixture();
        a.patterns.editing = None;
        crate::view::patterns::refresh(&mut a);
        let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
        let other = *c.patterns.keys().find(|&&p| p != id).unwrap();
        a.edit_tracks(|m| {
            let dest = operations::create_source(m)?;
            let c = Catalog::read(m)?;
            let container = c
                .tracks
                .iter()
                .find_map(|(&id, &t)| (t == dest).then_some(id))
                .unwrap();
            operations::relocate(m, id, container, 0)?;
            let scene = grid::create_scene(m, "Two sounds", 2, 2)?;
            let lanes = cat_smf_editor::score::scene::read(m)?.scenes[0]
                .lanes
                .clone();
            grid::place(
                m,
                Some(scene),
                lanes[0],
                1,
                Some(PatternOrScene::Pattern(id)),
            )?;
            grid::place(
                m,
                Some(scene),
                lanes[1],
                2,
                Some(PatternOrScene::Pattern(other)),
            )?;
            let lane = grid::add_lane(m, None)?;
            grid::place(m, None, lane, 1, Some(PatternOrScene::Scene(scene)))?;
            grid::place(m, None, lane, 3, Some(PatternOrScene::Scene(scene)))
        })
        .unwrap();
        let owner = Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns[&id]
            .track;
        a.loaded.as_mut().unwrap().patches[owner].layers[0]
            .settings
            .octave = 1;
        let p = prepare(&a, None).unwrap();
        assert_eq!(p.parts.len(), 2);
        assert_eq!(p.active.end, 8.0);
        let owned = p.parts.iter().find(|p| p.0 == owner).unwrap();
        let default = p.parts.iter().find(|p| p.0 == source).unwrap();
        assert_eq!(owned.1.layers[0].settings.octave, 1);
        assert_eq!(default.1, Default::default());
        assert_eq!(
            owned.2.notes.iter().map(|n| n.start).collect::<Vec<_>>(),
            vec![0.125, 4.125]
        );
        assert_eq!(
            default.2.notes.iter().map(|n| n.start).collect::<Vec<_>>(),
            vec![2.125, 6.125]
        );
        assert!(p
            .parts
            .iter()
            .all(|p| p.2.notes.iter().all(|n| n.pitch == 67)));
    }
    #[test]
    fn arrangement_sound_timing_undo_and_rebuild_share_preview() {
        let (mut a, _, id) = crate::view::patterns::edit_tests::fixture();
        a.patterns.editing = None;
        crate::view::patterns::refresh(&mut a);
        let mut ids = (0, 0, 0);
        a.edit_tracks(|m| {
            ids.0 = grid::create_scene(m, "A", 2, 2)?;
            let s = cat_smf_editor::score::scene::read(m)?.scenes[0].clone();
            grid::place(
                m,
                Some(ids.0),
                s.lanes[0],
                1,
                Some(PatternOrScene::Pattern(id)),
            )?;
            grid::place(
                m,
                Some(ids.0),
                s.lanes[1],
                1,
                Some(PatternOrScene::Pattern(id)),
            )?;
            ids.1 = grid::add_lane(m, None)?;
            ids.2 = grid::add_lane(m, None)?;
            grid::place(m, None, ids.1, 1, Some(PatternOrScene::Scene(ids.0)))?;
            grid::place(m, None, ids.2, 3, Some(PatternOrScene::Pattern(id)))
        })
        .unwrap();
        let p = prepare(&a, None).unwrap();
        assert_eq!(p.parts.len(), 1);
        assert_eq!(p.parts[0].2.notes.len(), 3);
        assert_eq!(p.active.end, 6.0);
        let preview = crate::view::patterns::preview::prepare(&a, id).unwrap();
        assert_eq!(p.parts[0].1, preview.parts[0].1);
        assert_eq!(
            p.parts[0]
                .2
                .notes
                .iter()
                .map(|n| n.start)
                .collect::<Vec<_>>(),
            vec![0.125, 0.125, 4.125]
        );
        assert_eq!(prepare(&a, Some(ids.0)).unwrap().active.end, 4.0);
        let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
        a.edit_tracks(|m| grid::move_part(m, None, (ids.2, 3), (ids.1, 4)))
            .unwrap();
        assert_eq!(prepare(&a, None).unwrap().parts[0].1, preview.parts[0].1);
        a.undo_music().unwrap();
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            bytes
        );
        start(&mut a, None);
        assert_eq!(
            a.playback.pending.as_ref().unwrap().active.mode,
            Mode::Arrangement(None)
        );
        crate::play::transport::stop(&mut a);
        assert!(a.playback.pending.is_none());
    }
}
