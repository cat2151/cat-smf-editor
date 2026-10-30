use super::{fixtures::Fixture, *};
use egui::PointerButton::Primary;

fn scroll(f: &Fixture) -> f32 {
    f.ctx
        .data(|d| d.get_temp::<Snapshot>(egui::Id::new((f.screen, "snapshot"))))
        .unwrap()
        .scroll
}

#[test]
fn hiding_and_reopening_a_timeline_does_not_resume_a_held_drag() {
    for screen in Screen::ALL {
        let mut f = Fixture::new(screen);
        let before = f.bytes();
        let map = f.rect("minimap");
        let point = |fraction| egui::pos2(map.left() + map.width() * fraction, map.center().y);
        f.pointer(point(0.6), Primary, true);
        f.frame(vec![egui::Event::PointerMoved(point(0.7))]);
        f.frame(vec![]);
        assert!(scroll(&f) > 0.0, "fixture pans {screen:?}");
        f.visible(false);
        f.frame(vec![]);
        f.visible(true);
        f.frame(vec![]);
        f.frame(vec![]);
        let reopened = scroll(&f);
        let cursor = crate::play::playback::location(&f.app).0;
        f.frame(vec![egui::Event::PointerMoved(point(0.9))]);
        f.frame(vec![]);
        f.pointer(point(0.9), Primary, false);
        f.frame(vec![]);
        assert!((scroll(&f) - reopened).abs() < 0.5, "stale drag {screen:?}");
        assert_eq!(
            crate::play::playback::location(&f.app).0,
            cursor,
            "stale click {screen:?}"
        );
        assert!(
            f.app.playback.pending.is_none() && f.app.playback.active.is_none(),
            "{screen:?}"
        );
        assert_eq!(f.bytes(), before, "{screen:?}");
    }
}
