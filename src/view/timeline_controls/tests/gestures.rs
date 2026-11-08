use super::{fixtures::Fixture, *};
use egui::PointerButton::{Primary, Secondary};

fn snapshot(f: &Fixture) -> Snapshot {
    f.ctx
        .data(|d| d.get_temp(egui::Id::new((f.screen, "snapshot"))))
        .unwrap()
}

#[test]
fn all_registered_ui_adapters_pan_live_preserve_domain_and_release_without_click() {
    for (screen, pane) in Fixture::cases() {
        let mut f = Fixture::new(screen, pane);
        let before = f.bytes();
        let generation = f.app.generation;
        let initial = snapshot(&f);
        assert!(initial.width > 0.0, "{screen:?} pane={pane}");
        let map = f.rect("minimap");
        assert!(
            f.rect("visible").width() < map.width() * 0.8,
            "long fixture: {screen:?} pane={pane}"
        );
        f.app.playback.repeat = Some((0, 480));
        for outside in [false, true] {
            let visible = f.rect("visible");
            let from = if outside {
                egui::pos2(map.left() + map.width() * 0.8, map.center().y)
            } else {
                visible.center()
            };
            let to = from + egui::vec2(map.width() * 0.12, 0.0);
            f.app.playback.follow = true;
            let cursor = crate::play::playback::location(&f.app).0;
            f.pointer(from, Primary, true);
            f.frame(vec![egui::Event::PointerMoved(to)]);
            f.frame(vec![]);
            let held = snapshot(&f);
            assert!(held.scroll > 0.0, "{screen:?} pane={pane}");
            assert!(!f.app.playback.follow, "{screen:?} pane={pane}");
            f.pointer(to, Primary, false);
            f.frame(vec![]);
            assert!(
                (snapshot(&f).scroll - held.scroll).abs() < 0.5,
                "release {screen:?} pane={pane}"
            );
            assert_eq!(
                snapshot(&f).end_x,
                initial.end_x,
                "zoom {screen:?} pane={pane}"
            );
            assert_eq!(
                f.app.playback.repeat,
                Some((0, 480)),
                "{screen:?} pane={pane}"
            );
            assert_eq!(
                crate::play::playback::location(&f.app).0,
                cursor,
                "drag must not seek {screen:?} pane={pane}"
            );
            assert!(
                f.app.playback.pending.is_none() && f.app.playback.active.is_none(),
                "{screen:?} pane={pane}"
            );
            assert_eq!(f.app.generation, generation);
            assert_eq!(f.bytes(), before);
        }
        // Both ends clamp, including while the pointer is outside the minimap.
        for right in [false, true] {
            let from = f.rect("visible").center();
            let to = egui::pos2(
                if right {
                    map.right() + 200.0
                } else {
                    map.left() - 200.0
                },
                map.center().y,
            );
            f.pointer(from, Primary, true);
            f.frame(vec![egui::Event::PointerMoved(to)]);
            f.pointer(to, Primary, false);
            f.frame(vec![]);
            let s = snapshot(&f);
            assert!(
                s.scroll >= 0.0 && s.scroll <= (s.end_x - s.width).max(0.0) + 0.5,
                "{screen:?} pane={pane} {s:?}"
            );
        }
    }
}

#[test]
fn all_registered_bar_adapters_share_click_range_and_wheel_zoom() {
    for (screen, pane) in Fixture::cases().filter(|(s, _)| s.has_bars()) {
        let mut f = Fixture::new(screen, pane);
        let bytes = f.bytes();
        let before = snapshot(&f).end_x;
        // The zoom bar is above the overview, with a known 24px thickness.
        let map = f.rect("minimap");
        let pos = egui::pos2(map.center().x, map.top() - 12.0);
        f.pointer(pos, Primary, true);
        f.pointer(pos, Primary, false);
        f.frame(vec![]);
        assert!(
            snapshot(&f).end_x > before,
            "left click {screen:?} pane={pane}"
        );
        f.pointer(pos, Secondary, true);
        f.pointer(pos, Secondary, false);
        f.frame(vec![]);
        assert!(
            (snapshot(&f).end_x - before).abs() < 1.0,
            "right click {screen:?} pane={pane}"
        );
        let prior = snapshot(&f).end_x;
        f.frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 40.0),
                modifiers: Default::default(),
            },
        ]);
        for _ in 0..12 {
            f.frame(vec![]);
        }
        assert!(snapshot(&f).end_x > prior, "wheel {screen:?} pane={pane}");
        for button in [Primary, Secondary] {
            let from = pos - egui::vec2(60.0, 0.0);
            let to = pos + egui::vec2(60.0, 0.0);
            let from = if button == Primary { from } else { to };
            let to = if button == Primary {
                to
            } else {
                pos - egui::vec2(60.0, 0.0)
            };
            let before = snapshot(&f).end_x;
            f.pointer(from, button, true);
            f.frame(vec![egui::Event::PointerMoved(to)]);
            f.pointer(to, button, false);
            f.frame(vec![]);
            assert!(
                snapshot(&f).end_x > before,
                "range {button:?} {screen:?} pane={pane}"
            );
        }
        assert_eq!(f.bytes(), bytes);
    }
}

#[test]
fn all_registered_bar_adapters_draw_the_range_over_content_without_zooming_until_release() {
    // The band spans the content, which is far taller than the 24px bar.
    fn band(f: &Fixture, from: f32, to: f32) -> bool {
        f.shapes.iter().any(|c| match &c.shape {
            egui::Shape::Rect(r) => {
                (r.rect.left() - from).abs() < 1.0
                    && (r.rect.right() - to).abs() < 1.0
                    && r.rect.height() > 100.0
            }
            _ => false,
        })
    }
    fn forecast(f: &Fixture) -> Option<egui::Rect> {
        f.ctx
            .data(|d| d.get_temp::<Option<egui::Rect>>(egui::Id::new((f.screen, "forecast"))))
            .flatten()
    }
    for (screen, pane) in Fixture::cases().filter(|(s, _)| s.has_bars()) {
        let mut f = Fixture::new(screen, pane);
        let bytes = f.bytes();
        let before = snapshot(&f);
        let map = f.rect("minimap");
        let pos = egui::pos2(map.center().x, map.top() - 12.0);
        let (from, to) = (pos - egui::vec2(60.0, 0.0), pos + egui::vec2(60.0, 0.0));
        assert!(!band(&f, from.x, to.x), "{screen:?} pane={pane}");
        f.pointer(from, Primary, true);
        f.frame(vec![egui::Event::PointerMoved(to)]);
        for _ in 0..20 {
            f.frame(vec![]);
            assert_eq!(
                snapshot(&f).end_x,
                before.end_x,
                "held {screen:?} pane={pane}"
            );
            assert!(band(&f, from.x, to.x), "band {screen:?} pane={pane}");
        }
        let predicted = forecast(&f).expect("forecast while held");
        f.pointer(to, Primary, false);
        f.frame(vec![]);
        f.frame(vec![]);
        assert!(
            snapshot(&f).end_x > before.end_x * 2.0,
            "release {screen:?} pane={pane}"
        );
        assert!(
            !band(&f, from.x, to.x),
            "band after release {screen:?} pane={pane}"
        );
        assert!(
            forecast(&f).is_none(),
            "forecast after release {screen:?} pane={pane}"
        );
        let after = f.rect("visible");
        if screen == Screen::Song {
            // Song's fixed label and gaps do not scale, so its release fit lands
            // off the 90%-centered target that the forecast shows.
            assert!(
                after.intersects(predicted),
                "forecast {screen:?} pane={pane}"
            );
        } else {
            assert!(
                (after.left() - predicted.left()).abs() < 1.0
                    && (after.right() - predicted.right()).abs() < 1.0,
                "forecast {screen:?} pane={pane} {predicted:?} {after:?}"
            );
        }
        assert_eq!(f.bytes(), bytes);
    }
}
