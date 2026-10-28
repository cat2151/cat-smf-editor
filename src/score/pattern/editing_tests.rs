use super::*;
use crate::{
    edit,
    model::{Event, Track},
};
#[test]
fn local_multimeasure_projection_maps_expressions_preserves_neighbors_and_rejects_outside() {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    };
    let t = operations::create_source(&mut m).unwrap();
    let a = operations::create(&mut m, t, None).unwrap();
    let b = operations::create(&mut m, t, None).unwrap();
    let owner = Catalog::read(&m).unwrap().patterns[&a].container;
    operations::relocate(&mut m, b, owner, 20000).unwrap();
    operations::resize(
        &mut m,
        b,
        vec![
            Meter {
                numerator: 3,
                exponent: 2,
                ticks: 1440
            };
            3
        ],
    )
    .unwrap();
    let c = editing::Context {
        track: t,
        measure: 1,
        pattern: b,
        placement: None,
        scene: None,
        standalone: true,
    };
    let v = c.project(&m).unwrap();
    let mut edited = v.clone();
    for (tick, status, data) in [
        (2000, 0xb3, vec![11, 75]),
        (2100, 0x93, vec![61, 104]),
        (4320, 0x83, vec![61, 97]),
    ] {
        edit::insert(
            &mut edited.tracks[t],
            Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data,
            },
        );
    }
    edit::refresh_deltas(&mut edited.tracks[t]).unwrap();
    let mapped = c.map(&m, &v, &edited).unwrap();
    let index = Catalog::read(&mapped).unwrap();
    assert_eq!(index.patterns[&a].tick, 0);
    let body = body::extract(
        &mapped.tracks[t].events,
        index.patterns[&b].range().unwrap(),
    )
    .unwrap();
    assert_eq!(
        body.iter().map(|e| e.tick).collect::<Vec<_>>(),
        vec![22000, 22100, 24320]
    );
    assert_eq!(body[2].data, vec![61, 97]);
    let mut bad = edited;
    edit::insert(
        &mut bad.tracks[t],
        Event {
            tick: 4320,
            delta: 0,
            status: 0xb3,
            meta_type: 0,
            data: vec![11, 1],
        },
    );
    assert!(c.map(&m, &v, &bad).is_err());
    assert!(
        body::extract(&m.tracks[t].events, index.patterns[&a].range().unwrap())
            .unwrap()
            .is_empty()
    );
}
