use super::*;

fn screen(number: u32, bounds: [i32; 4], scale: f32) -> Screen {
    Screen {
        number,
        bounds,
        work: bounds,
        scale,
    }
}
fn placement(monitor: Monitor) -> MonitorPlacement {
    MonitorPlacement {
        monitor,
        left_percent: 0.0,
        top_percent: 0.0,
        width_percent: 100.0,
        height_percent: 100.0,
    }
}
fn three() -> Vec<Screen> {
    vec![
        screen(3, [3000, 0, 1920, 1080], 1.5),
        screen(1, [0, 0, 1920, 1080], 1.0),
        screen(2, [1920, -836, 1080, 1920], 1.0),
    ]
}

#[test]
fn spatial_selection_is_independent_of_enumeration_and_display_number_order() {
    let screens = three();
    for (position, expected) in [
        (Position::Left, 1),
        (Position::Center, 2),
        (Position::Right, 3),
    ] {
        assert_eq!(
            placement(Monitor::Position(position)).resolve(&screens),
            placement(Monitor::Number(expected)).resolve(&screens)
        );
    }
    assert_eq!(
        placement(Monitor::Position(Position::Center))
            .resolve(&screens)
            .unwrap()
            .outer,
        [1920, -836, 1080, 1920]
    );
}

#[test]
fn percentages_use_work_area_and_keep_mixed_dpi_in_physical_pixels() {
    let mut screens = three();
    screens[0].work = [3000, 0, 1920, 1040];
    let mut p = placement(Monitor::Position(Position::Right));
    p.left_percent = 50.0;
    p.width_percent = 50.0;
    p.height_percent = 50.0;
    let resolved = p.resolve(&screens).unwrap();
    assert_eq!(
        resolved,
        Resolved {
            outer: [3960, 0, 960, 520],
            scale: 1.5
        }
    );
    assert_eq!(
        resolved.outer.map(|n| n as f32 / resolved.scale),
        [2640.0, 0.0, 640.0, 346.66666]
    );
}

#[test]
fn one_monitor_accepts_all_positions_and_ambiguities_require_a_number() {
    let mut screens = vec![screen(7, [-1920, -100, 1920, 1080], 1.0)];
    for position in [Position::Left, Position::Center, Position::Right] {
        assert_eq!(
            placement(Monitor::Position(position))
                .resolve(&screens)
                .unwrap()
                .outer,
            screens[0].work
        );
    }
    screens.push(screen(2, [0, 0, 1920, 1080], 1.0));
    assert!(placement(Monitor::Position(Position::Center))
        .resolve(&screens)
        .is_err());
    screens[1].bounds = [-1920, 980, 1920, 1080];
    for position in [Position::Left, Position::Right] {
        assert!(placement(Monitor::Position(position))
            .resolve(&screens)
            .is_err());
    }
    assert!(placement(Monitor::Number(2)).resolve(&screens).is_ok());
    assert!(placement(Monitor::Number(1)).resolve(&screens).is_err());
    assert!(placement(Monitor::Position(Position::Left))
        .resolve(&[])
        .is_err());
}

#[test]
fn invalid_percentages_and_monitor_numbers_are_rejected() {
    let mut p = placement(Monitor::Number(1));
    assert!(p.validate().is_ok());
    for invalid in [0.0, -1.0, 101.0, f32::NAN, f32::INFINITY] {
        p.width_percent = invalid;
        assert!(p.validate().is_err(), "{invalid}");
    }
    p.width_percent = 100.0;
    p.left_percent = 1.0;
    assert!(p.validate().is_err());
    p.left_percent = 0.0;
    p.monitor = Monitor::Number(0);
    assert!(p.validate().is_err());
}

#[test]
fn toml_is_explicit_and_title_is_generated_from_the_panes() {
    let text = r#"
[[window]]
column = [{ view = "song", height = 2 }, { view = "mixer" }]
[window.monitor_placement]
monitor = "center"
left_percent = 0
top_percent = 0
width_percent = 100
height_percent = 100
"#;
    let spec = super::super::parse(text).unwrap();
    assert_eq!(spec.docks[0].title, "cat-smf-editor · song · mixer");
    for valid in ["1", "2", "\"left\"", "\"right\""] {
        assert!(super::super::parse(&text.replace("\"center\"", valid)).is_ok());
    }
    for invalid in ["0", "-1", "\"primary\"", "\"portrait\"", "\"rightmost\""] {
        assert!(super::super::parse(&text.replace("\"center\"", invalid)).is_err());
    }
    assert!(
        super::super::parse(&text.replace("width_percent = 100", "width_percent = 101")).is_err()
    );
    assert!(
        super::super::parse(&text.replace("column =", "rect = [0, 0, 10, 10]\ncolumn =")).is_err()
    );
    assert!(
        super::super::parse(&text.replace("[window.monitor_placement]", "[window.area]")).is_err()
    );
}
