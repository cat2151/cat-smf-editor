use super::super::timeline_repeat::TickAxis;
use super::*;

#[test]
fn binding_uses_content_axis_for_known_boundaries_and_limits() {
    struct Uneven;
    impl TickAxis for Uneven {
        fn x(&self, tick: u64) -> f32 {
            if tick < 480 {
                tick as f32 * 2.0 + 8.0
            } else {
                968.0 + (tick - 480) as f32 * 0.5
            }
        }
        fn tick(&self, x: f32) -> u64 {
            if x < 968.0 {
                ((x - 8.0).max(0.0) / 2.0).round() as u64
            } else {
                480 + ((x - 968.0) * 2.0).round() as u64
            }
        }
    }
    let binding = Binding {
        screen: Screen::Staff,
        axis: &Uneven,
        viewport: egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(200.0, 100.0)),
        scroll: 968.0,
        end: 2000,
        zoom_range: None,
    };
    assert_eq!(binding.visible(), (480.0, 880.0));
    assert_eq!(binding.center(480.0), 868.0);
    assert_eq!(binding.clamp(-200.0), 0.0);
    assert_eq!(binding.clamp(99999.0), 1528.0);
}

#[test]
fn horizontal_zoom_reuses_the_same_anchor_and_limits() {
    assert_eq!(zoom(500.0, 400.0, 1.0, 1.5, 100.0), (1.5, 700.0));
    assert_eq!(zoom(0.0, 400.0, 1.0, 0.5, -100.0), (0.5, 0.0));
}

mod fixtures;
mod gestures;
mod policies;

mod repeat;

mod lifecycle;
