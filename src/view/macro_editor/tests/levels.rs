use super::*;

fn six_sines(value: f64) -> Curve {
    Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "org.baconpaul.six-sines".into(),
            parameter_id: "40000".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: -1.0,
        maximum: 1.0,
        name: "Macro 1".into(),
        path: "Macros".into(),
        points: vec![Point { tick: 0, value }],
        controls: vec![],
    }
}

#[test]
fn rows_edit_positions_and_send_the_level_range() {
    let row = Row {
        curve: six_sines(0.5),
        value: Arc::new(AtomicU64::new(0.5f64.to_bits())),
        bipolar: true,
        readback: None,
    };
    assert_eq!(row.get(), 0.75);
    assert_eq!(row.signal(), 0.5);
    row.set(0.25);
    assert_eq!(row.saved().points[0].value, -0.5);
    assert_eq!(f64::from_bits(row.value.load(Ordering::Relaxed)), -0.5);
    row.set(2.0);
    assert_eq!(row.saved().points[0].value, 1.0);
    row.set_value(-1.0);
    assert_eq!(row.get(), 0.0);
    assert_eq!(initial(&row.curve, None, Some(0.0)), Some(-1.0));
    assert_eq!(initial(&row.curve, Some(0.2), Some(0.0)), Some(0.2));
}

#[test]
fn saved_macro_values_take_only_the_instruments_own_range() {
    let save = |curve: Curve| {
        macro_values::write(
            &mut tracks::new_song().tracks[0],
            &Values {
                version: 1,
                parameters: vec![curve],
                variations: vec![],
            },
        )
    };
    assert!(save(six_sines(-0.25)).is_ok());
    let mut unit = six_sines(0.25);
    unit.minimum = 0.0;
    assert!(save(unit).is_err());
    let mut surge = six_sines(-0.25);
    surge.target.plugin_id = "org.surge-synth-team.surge-xt".into();
    assert!(save(surge).is_err());
}
