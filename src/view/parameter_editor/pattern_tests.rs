use super::*;
#[test]
pub(super) fn pattern_parameter_curves_keep_local_ticks_baselines_and_other_bodies() {
    use cat_smf_editor::score::pattern::{automation as storage, Catalog};
    let (mut a, source, id) = crate::view::patterns::edit_tests::fixture();
    let _instrument = crate::test_support::instrument::install(&mut a, source);
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let p = &catalog.patterns[&id];
    let target = Target {
        group: p.container,
        layer: 1,
        plugin_format: "CLAP".into(),
        plugin_id: "org.surge-synth-team.surge-xt".into(),
        parameter_id: "17".into(),
    };
    let curve = Curve {
        version: 1,
        target: target.clone(),
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 10.0,
        name: "Cutoff".into(),
        path: "Filter".into(),
        points: vec![Point {
            tick: 0,
            value: 4.0,
        }],
        controls: vec![],
    };
    storage::write(
        &mut a.loaded.as_mut().unwrap().music,
        id,
        curve.clone(),
        Some(3.0),
    )
    .unwrap();
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    a.parameter_editor.pattern_curve = Some((id, target));
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let mut next = curve;
    next.points.push(Point {
        tick: 960,
        value: 8.0,
    });
    next.controls.push(Point {
        tick: 480,
        value: 6.0,
    });
    store(&mut a, next.clone());
    assert!(
        a.parameter_editor.error.is_none(),
        "{:?}",
        a.parameter_editor.error
    );
    assert_eq!(current(&a).unwrap(), next);
    let m = &a.loaded.as_ref().unwrap().music;
    let index = Catalog::read(m).unwrap();
    assert_eq!(m.tracks.len(), 2, "a body curve is not a new SMF track");
    assert_eq!(
        storage::curves(m, &index.patterns[&id]).unwrap()[0].points[1].tick,
        960
    );
    assert!(
        matches!(&storage::baselines(m,index.patterns[&id].track).unwrap()[0],storage::Baseline::Parameter {value,..} if *value==3.0)
    );
    a.undo_music().unwrap();
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
