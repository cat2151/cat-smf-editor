//! Wheel actions follow the hovered bar, or scroll time elsewhere.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Mode {
    TimeScroll,
    BpmScroll,
    TimeZoom,
    BpmZoom,
}

impl Viewport {
    pub(super) fn wheel(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        base: Axis,
        enabled: bool,
        mut mode: Mode,
    ) {
        if !ui.rect_contains_pointer(rect) {
            return;
        }
        let delta = ui.input_mut(|i| {
            let delta = if i.smooth_scroll_delta.y != 0.0 {
                i.smooth_scroll_delta.y
            } else {
                i.smooth_scroll_delta.x
            };
            i.smooth_scroll_delta = egui::Vec2::ZERO;
            i.raw_scroll_delta = egui::Vec2::ZERO;
            delta
        });
        if enabled && delta != 0.0 {
            if self
                .horizontal_bar
                .is_some_and(|bar| ui.rect_contains_pointer(bar))
            {
                mode = Mode::TimeScroll;
            } else if self
                .vertical_scrollbar
                .is_some_and(|bar| ui.rect_contains_pointer(bar))
            {
                mode = Mode::BpmScroll;
            }
            self.apply_wheel(delta, base, mode);
        }
    }

    pub(super) fn apply_wheel(&mut self, delta: f32, base: Axis, mode: Mode) {
        match mode {
            Mode::TimeScroll => {
                self.pending_scroll = Some((self.scroll - delta).max(0.0));
            }
            Mode::BpmScroll => {
                if self.limit_to_fit {
                    return;
                }
                let axis = self.axis.unwrap_or(base);
                let height = self.tempo_rect.map_or(200.0, |r| r.height()).max(1.0);
                let span = axis.high - axis.low;
                let bounds = self.bounds();
                let low = (axis.low + f64::from(delta / height) * span)
                    .clamp(*bounds.start(), (*bounds.end() - span).max(*bounds.start()));
                self.axis = Some(Axis {
                    low,
                    high: low + span,
                });
            }
            Mode::TimeZoom => {
                self.zoom_time(self.zoom * timeline_zoom::wheel(delta), self.width / 2.0);
            }
            Mode::BpmZoom => self.zoom_bpm(base, timeline_zoom::wheel_f64(f64::from(delta)), 0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variation_axis_restores_fit_but_never_enlarges_beyond_it_or_scrolls() {
        let base = Axis {
            low: 0.0,
            high: 1.0,
        };
        let mut view = Viewport {
            limit_to_fit: true,
            bounds: Some(Axis {
                low: -1.5,
                high: 2.5,
            }),
            axis: Some(Axis {
                low: -0.15,
                high: 1.15,
            }),
            ..Default::default()
        };
        let before = view.axis;
        view.zoom_edge(2, true, base, 0.0);
        view.apply_wheel(40.0, base, Mode::BpmZoom);
        view.apply_wheel(40.0, base, Mode::BpmScroll);
        let axis = view.axis.unwrap();
        let before = before.unwrap();
        assert!((axis.low - before.low).abs() < 1e-9);
        assert!((axis.high - before.high).abs() < 1e-9);
        view.zoom_edge(2, false, base, 0.0);
        let axis = view.axis.unwrap();
        assert!(axis.low < -0.15 && axis.high > 1.15);
        assert!((axis.high + axis.low - 1.0).abs() < 1e-9);
        for _ in 0..20 {
            view.apply_wheel(-40.0, base, Mode::BpmZoom);
        }
        assert_eq!(view.axis, view.bounds);
        view.zoom_edge(2, true, base, 0.0);
        let restored = view.axis.unwrap();
        assert!(restored.high - restored.low < 4.0);
        assert!((restored.high + restored.low - 1.0).abs() < 1e-9);
        for _ in 0..20 {
            view.apply_wheel(40.0, base, Mode::BpmZoom);
        }
        let restored = view.axis.unwrap();
        assert!((restored.low + 0.15).abs() < 1e-9);
        assert!((restored.high - 1.15).abs() < 1e-9);
        view.zoom_edge(0, true, base, 0.0);
        assert!(view.zoom > 1.0);
    }

    #[test]
    fn wheel_operations_leave_the_other_axis_unchanged() {
        let base = Axis {
            low: 100.0,
            high: 200.0,
        };
        let mut view = Viewport {
            scroll: 500.0,
            width: 400.0,
            ..Default::default()
        };
        view.apply_wheel(-40.0, base, Mode::TimeScroll);
        assert_eq!(view.pending_scroll.take(), Some(540.0));
        assert_eq!(view.axis, None);
        assert_eq!(view.zoom, 1.0);

        view.zoom_edge(0, true, base, 200.0);
        let zoom = view.zoom;
        view.apply_wheel(40.0, base, Mode::TimeZoom);
        assert!(view.zoom > zoom);
        assert_eq!(view.axis, None);
        view.pending_scroll = None;

        view.zoom_edge(2, true, base, 0.5);
        let zoom = view.zoom;
        let axis = view.axis.unwrap();
        view.apply_wheel(40.0, base, Mode::BpmZoom);
        let next = view.axis.unwrap();
        assert!(next.high - next.low < axis.high - axis.low);
        assert!((next.high + next.low - axis.high - axis.low).abs() < 1e-9);
        assert_eq!(view.zoom, zoom);

        let axis = view.axis.unwrap();
        view.apply_wheel(40.0, base, Mode::BpmScroll);
        let next = view.axis.unwrap();
        assert!(next.low > axis.low);
        assert!((next.high - next.low - (axis.high - axis.low)).abs() < 1e-9);
        assert_eq!(view.pending_scroll, None);
        assert_eq!(view.zoom, zoom);

        view.apply_wheel(40.0, base, Mode::TimeScroll);
        assert_eq!(view.pending_scroll, Some(460.0));
        assert_eq!(view.axis, Some(next));
        assert_eq!(view.zoom, zoom);
    }
}
