//! A container holding patterns p1 and p2 that no Scene or Song places.
use crate::app::App;
use cat_smf_editor::{
    edit,
    model::{Event, Music, Track},
    score::pattern::{self, operations},
    voicevox::Voice,
};

/// The container's track index in [`fixture`] and [`singing`].
pub(crate) const CONTAINER: usize = 1;

/// Track [`CONTAINER`] holds p1 (one quarter note at pitch 60) and p2 (pitch 72). Returns p1.
pub(crate) fn fixture() -> (App, u64) {
    build(false)
}

/// [`fixture`] with the container singing and p1's note carrying the lyric "ど".
pub(crate) fn singing() -> (App, u64) {
    build(true)
}

fn build(sing: bool) -> (App, u64) {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![Event {
                tick: 7680,
                delta: 7680,
                status: 0xff,
                meta_type: 0x2f,
                data: vec![],
            }],
        }],
    };
    let source = operations::create_source(&mut m).unwrap();
    assert_eq!(source, CONTAINER);
    let mut ids = vec![];
    for pitch in [60, 72] {
        let id = operations::create(&mut m, source, None).unwrap();
        let start = pattern::Catalog::read(&m).unwrap().patterns[&id]
            .range()
            .unwrap()
            .0;
        for (tick, status) in [(start, 0x90), (start + 480, 0x80)] {
            edit::insert(
                &mut m.tracks[source],
                Event {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![pitch, 100],
                },
            );
        }
        if sing && pitch == 60 {
            edit::insert(
                &mut m.tracks[source],
                Event {
                    tick: start,
                    delta: 0,
                    status: 0xff,
                    meta_type: edit::lyric::LYRIC,
                    data: "ど".as_bytes().to_vec(),
                },
            );
        }
        ids.push(id);
    }
    if sing {
        Voice::write(&mut m.tracks[source], Some(&Voice::default())).unwrap();
    }
    for t in &mut m.tracks {
        edit::refresh_deltas(t).unwrap();
    }
    (super::app(m), ids[0])
}
