use super::{fixtures::Fixture, *};
use crate::play::transport;
use egui::PointerButton::Primary;

#[test]
fn registered_stopped_click_policies_are_explicit_and_do_not_edit() {
    for screen in Screen::ALL {
        let mut f = Fixture::new(screen);
        assert_eq!(
            screen.policy(),
            f.expected_click(),
            "registered stopped-click contract {screen:?}"
        );
        let bytes = f.bytes();
        let cursor = crate::play::playback::location(&f.app).0;
        let map = f.rect("minimap");
        let pos = egui::pos2(map.left() + map.width() * 0.4, map.center().y);
        f.app.playback.follow = true;
        f.pointer(pos, Primary, true);
        f.pointer(pos, Primary, false);
        f.frame(vec![]);
        assert!(!f.app.playback.follow, "{screen:?}");
        match f.expected_click() {
            Click::SeekPlaying => {
                assert!(
                    f.app.playback.pending.is_none() && f.app.playback.active.is_none(),
                    "{screen:?}"
                );
                assert_eq!(
                    crate::play::playback::location(&f.app).0,
                    cursor,
                    "{screen:?}"
                );
            }
            Click::Cursor => {
                assert!(f.app.playback.pending.is_none() && f.app.playback.active.is_none());
                assert!(crate::play::playback::location(&f.app).0 > cursor);
            }
            Click::Song | Click::Scene => {
                let expected = f.expected_mode();
                let active = f
                    .app
                    .playback
                    .pending
                    .as_ref()
                    .map(|p| &p.active)
                    .or(f.app.playback.active.as_ref())
                    .expect("arrangement starts");
                assert_eq!(active.mode, expected);
            }
        }
        assert_eq!(f.bytes(), bytes);
    }
}

#[test]
fn every_registered_ui_drag_during_playback_preserves_position_scope_and_repeat() {
    for screen in Screen::ALL {
        let mut f = Fixture::new(screen);
        let mode = f.expected_mode();
        crate::play::playback::start(&mut f.app, mode);
        transport::advance(&mut f.app);
        assert!(
            f.app.playback.active.is_some(),
            "fixture {screen:?}: {:?}",
            f.app.load_error
        );
        f.app.audition.set_test_seconds(1.0);
        f.app.playback.repeat = Some((0, 153600));
        f.app.playback.follow = true;
        let seconds = f
            .app
            .playback
            .active
            .as_ref()
            .expect("fixture playing")
            .start_seconds;
        let before = f.bytes();
        let map = f.rect("minimap");
        let from = egui::pos2(map.left() + map.width() * 0.65, map.center().y);
        let to = from + egui::vec2(map.width() * 0.1, 0.0);
        f.pointer(from, Primary, true);
        f.frame(vec![egui::Event::PointerMoved(to)]);
        f.pointer(to, Primary, false);
        f.frame(vec![]);
        f.frame(vec![]);
        assert!(!f.app.playback.follow, "{screen:?}");
        assert_eq!(f.app.playback.repeat, Some((0, 153600)));
        let active = f.app.playback.active.as_ref().expect("still playing");
        assert_eq!(active.mode, mode, "{screen:?}");
        assert_eq!(active.start_seconds, seconds, "drag cannot seek {screen:?}");
        assert_eq!(f.bytes(), before);
    }
}

#[test]
fn probe_follow() {
    for screen in Screen::ALL {
        let mut f = Fixture::new(screen);
        let mode = f.expected_mode();
        crate::play::playback::start(&mut f.app, mode);
        transport::advance(&mut f.app);
        f.app.audition.set_test_seconds(1.0);
        f.app.playback.follow = false;
        let key = |down| egui::Event::Key {
            key: egui::Key::F,
            physical_key: None,
            pressed: down,
            repeat: false,
            modifiers: Default::default(),
        };
        f.frame(vec![key(true), key(false)]);
        let after_key = f.app.playback.follow;
        f.app.playback.follow = true;
        let mut log = vec![];
        for s in [5.0, 20.0, 40.0] {
            if f.app.playback.active.is_none() {
                log.push("stopped".into());
                break;
            }
            f.app.audition.set_test_seconds(s);
            transport::advance(&mut f.app);
            for _ in 0..4 {
                f.frame(vec![]);
            }
            let tick = f
                .app
                .playback
                .active
                .as_ref()
                .and_then(|a| crate::play::transport::tick_for(&f.app, a.clock_track));
            log.push(format!(
                "s={s} tick={tick:?} scroll={:.0} follow={}",
                f.ctx
                    .data(|d| d.get_temp::<Snapshot>(egui::Id::new((screen, "snapshot"))))
                    .unwrap()
                    .scroll,
                f.app.playback.follow
            ));
        }
        eprintln!(
            "PROBE {screen:?} F-key toggled_to={after_key} | {}",
            log.join(" | ")
        );
    }
}
