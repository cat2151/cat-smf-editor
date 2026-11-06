use super::*;

pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if a.loaded
        .as_ref()
        .is_some_and(|l| l.patches.get(a.track).is_some_and(|p| p.kit.is_none()))
        && matches!(s, Screen::Main | Screen::Scene | Screen::Song)
        && !a.patterns.confirming_creation()
    {
        out.push(Command::new(
            s,
            "record-pattern",
            "Virtual MIDI keyboard",
            &[(Key::R, Modifiers::SHIFT)],
            |a, ctx| {
                if let Err(error) = open(a, ctx) {
                    a.load_error = Some(error);
                }
            },
        ));
    }
}
