use super::*;

#[test]
fn presses_sweep_velocity_up_and_down_as_cat_plugin_player() {
    let v: Vec<u8> = (0..18)
        .map(|n| Velocity::Range80To127.of_press(n))
        .collect();
    assert_eq!(
        v,
        [80, 86, 93, 100, 106, 113, 120, 127, 127, 121, 114, 107, 101, 94, 87, 80, 80, 86]
    );
    assert_eq!(Velocity::Range40To100.of_press(7), 100);
    assert_eq!(Velocity::V100.of_press(5), 100);
}

#[test]
fn cc1_sweeps_0_127_0_over_four_seconds() {
    let ms = Duration::from_millis;
    let m = Modulation::Sweep;
    assert_eq!(m.value(ms(0)), Some(0));
    assert_eq!(m.value(ms(1000)), Some(63));
    assert_eq!(m.value(ms(2000)), Some(127));
    assert_eq!(m.value(ms(3000)), Some(64));
    assert_eq!(m.value(ms(4000)), Some(0));
    assert_eq!(Modulation::Off.value(ms(1000)), None);
    assert_eq!(Modulation::Fixed(127).value(ms(1000)), Some(127));
}

#[test]
fn changes_match_the_value_at_their_time_and_stay_in_the_span() {
    let ms = Duration::from_millis;
    let m = Modulation::Sweep;
    let changes = m.changes(ms(0), ms(1500), ms(2500));
    assert_eq!(
        changes.first(),
        Some(&(ms(1500), m.value(ms(1500)).unwrap()))
    );
    assert!(changes
        .iter()
        .all(|&(at, v)| at < ms(2500) && m.value(at) == Some(v)));
    // Each value is new: up to 127, then down.
    assert!(changes.windows(2).all(|w| w[0].1 != w[1].1));
    assert!(changes.iter().any(|c| c.1 == 127));
    assert_eq!(changes.len(), 1 + 127 / 2);
    assert_eq!(
        Modulation::Fixed(0).changes(ms(0), ms(0), ms(9000)),
        vec![(ms(0), 0)]
    );
    assert!(Modulation::Off.changes(ms(0), ms(0), ms(9000)).is_empty());
}

#[test]
fn the_sweep_holds_its_first_value_until_it_starts() {
    let ms = Duration::from_millis;
    let m = Modulation::Sweep;
    let changes = m.changes(ms(5000), ms(4000), ms(6000));
    assert_eq!(changes.first(), Some(&(ms(4000), 0)));
    assert!(changes[1..]
        .iter()
        .all(|&(at, v)| at > ms(5000) && m.value(at - ms(5000)) == Some(v)));
    assert_eq!(changes.last().unwrap().1, 63);
}
