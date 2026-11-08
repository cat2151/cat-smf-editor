//! Other windows are read and commanded only while the backend has them.
use super::super::{finish, input_for, send_to, Native};
use crate::view::patterns::workspace_tests::{context, harness};
use eframe::egui::{ViewportCommand, ViewportId};

#[test]
fn a_window_not_shown_is_neither_read_nor_commanded_into_existence() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let host = ViewportId::from_hash_of("windows-test-host");
    let absent = ViewportId::from_hash_of("windows-test-absent");
    for frame in 0..3 {
        let _ = c.run(Default::default(), |c| {
            assert_eq!(input_for(c, absent, |i| i.focused), None);
            send_to(c, absent, ViewportCommand::Focus);
            Native::Keyboard.show(&mut a, c, host, Default::default(), |_, _, _| {});
            finish(&a, c);
        });
        assert_eq!(
            harness::unshown(&c),
            Vec::<ViewportId>::new(),
            "frame {frame}"
        );
    }
}

#[test]
fn a_command_before_a_window_first_shows_reaches_it_at_the_end_of_the_frame() {
    let mut a = crate::view::palette::tests::app();
    let c = context();
    let window = ViewportId::from_hash_of("windows-test-new");
    let out = c.run(Default::default(), |c| {
        send_to(c, window, ViewportCommand::Focus);
        Native::Keyboard.show(&mut a, c, window, Default::default(), |_, _, _| {});
        finish(&a, c);
    });
    let commands = &out.viewport_output[&window].commands;
    assert!(commands.contains(&ViewportCommand::Focus), "{commands:?}");
}
