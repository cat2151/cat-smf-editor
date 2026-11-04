use super::*;
use crate::{
    model::Music,
    score::{
        note_view::NoteView,
        pattern::{self, operations, Catalog},
    },
};

fn fixture() -> (Music, usize, u64) {
    let mut music = edit::tracks::new_song();
    let track = operations::create_source(&mut music).unwrap();
    write(&mut music.tracks[track], Some(&Settings::default())).unwrap();
    let id = operations::create(&mut music, track, None).unwrap();
    operations::resize(
        &mut music,
        id,
        vec![
            pattern::Meter {
                numerator: 4,
                exponent: 2,
                ticks: 1920,
            };
            2
        ],
    )
    .unwrap();
    (music, track, id)
}

fn hits(events: &[Event], start: u64) -> Vec<(u64, u8, u64)> {
    let view = NoteView::new(events);
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .map(|(i, e)| (e.tick - start, e.data[0], view.durations[i].unwrap()))
        .collect()
}

#[test]
fn existing_short_drum_pattern_renders_across_bars_without_mutating_the_source() {
    let (mut music, track, id) = fixture();
    let catalog = Catalog::read(&music).unwrap();
    let p = &catalog.patterns[&id];
    for (tick, pitch) in [(0, 36), (240, 38), (2160, 36)] {
        for (offset, status, velocity) in [(0, 0x99, 97), (120, 0x89, 43)] {
            edit::insert(
                &mut music.tracks[track],
                Event {
                    tick: p.tick + tick + offset,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![pitch, velocity],
                },
            );
        }
    }
    edit::refresh_deltas(&mut music.tracks[track]).unwrap();
    let before = crate::smf_io::encode(&music).unwrap();
    let output = pattern::render::expand(&music, &catalog, &[(id, 100)], 3940).unwrap();
    assert_eq!(
        hits(&output.events[&track], 100),
        vec![(0, 36, 2159), (240, 38, 3600), (2160, 36, 1680)]
    );
    assert!(output.events[&track]
        .iter()
        .filter(|e| crate::score::note_view::note_off(e))
        .all(|e| e.data[1] == 43));
    assert_eq!(crate::smf_io::encode(&music).unwrap(), before);
    let context = pattern::editing::Context {
        track,
        pattern: id,
        measure: 1,
        placement: None,
        scene: None,
        standalone: true,
    };
    let projected = context.project(&music).unwrap();
    assert_eq!(
        hits(&projected.tracks[track].events, 0),
        hits(&output.events[&track], 100)
    );
    edit::regenerate(&mut music).unwrap();
    let bytes = crate::smf_io::encode(&music).unwrap();
    let mut restored = crate::ffi::parse(&bytes).unwrap();
    edit::regenerate(&mut restored).unwrap();
    assert_eq!(crate::smf_io::encode(&restored).unwrap(), bytes);
}

#[test]
fn editing_and_deleting_a_hit_rebuilds_only_its_pattern_and_keeps_other_rows_independent() {
    let (mut music, track, id) = fixture();
    let neighbor = operations::create(&mut music, track, None).unwrap();
    let catalog = Catalog::read(&music).unwrap();
    let neighbor_tick = catalog.patterns[&neighbor].tick;
    toggle(
        &mut music.tracks[track],
        480,
        2,
        neighbor_tick.div_ceil(120),
    )
    .unwrap();
    let context = pattern::editing::Context {
        track,
        pattern: id,
        measure: 1,
        placement: None,
        scene: None,
        standalone: true,
    };
    for (row, bar, step) in [(0, 2, 2), (0, 1, 0), (1, 1, 1), (0, 1, 4)] {
        let view = context.project(&music).unwrap();
        let mut edited = view.clone();
        toggle_measure(&mut edited, track, row, bar, step).unwrap();
        music = context.map(&music, &view, &edited).unwrap();
        edit::regenerate(&mut music).unwrap();
    }
    let p = &Catalog::read(&music).unwrap().patterns[&id];
    assert_eq!(
        hits(
            &pattern::body::extract(&music.tracks[track].events, p.range().unwrap()).unwrap(),
            p.tick
        ),
        vec![
            (0, 36, 479),
            (120, 38, 3720),
            (480, 36, 1679),
            (2160, 36, 1680)
        ]
    );
    let view = context.project(&music).unwrap();
    let mut edited = view.clone();
    toggle_measure(&mut edited, track, 0, 1, 4).unwrap();
    music = context.map(&music, &view, &edited).unwrap();
    edit::regenerate(&mut music).unwrap();
    assert_eq!(
        hits(&context.project(&music).unwrap().tracks[track].events, 0),
        vec![(0, 36, 2159), (120, 38, 3720), (2160, 36, 1680)]
    );
    let p = &Catalog::read(&music).unwrap().patterns[&neighbor];
    let neighbor_body =
        pattern::body::extract(&music.tracks[track].events, p.range().unwrap()).unwrap();
    assert_eq!(hits(&neighbor_body, p.tick).len(), 1);
    assert_eq!(neighbor_body[0].data, vec![42, 100]);
}
