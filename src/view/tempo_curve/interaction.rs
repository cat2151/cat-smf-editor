//! Curve editing previews and one-beat all-track audition.
use super::*;

type Update = (
    Option<Curve>,
    Option<(Target, usize)>,
    Option<Option<Curve>>,
);

pub(super) fn update(
    state: &mut State,
    roll: &Roll,
    response: &egui::Response,
    curve: Option<&Curve>,
    handled: bool,
) -> Update {
    let primary = egui::PointerButton::Primary;
    if !handled && response.drag_started_by(primary) {
        state.draft = None;
        if let Some(origin) = response.ctx.input(|i| i.pointer.press_origin()) {
            if let Some((target, index)) = curve.and_then(|c| roll.hit(c, origin)) {
                state.drag = Some(Drag {
                    target,
                    index,
                    offset: roll.marks(curve.unwrap(), target)[index] - origin,
                    axis: roll.axis,
                    blocked: None,
                });
            } else {
                let (tick, bpm) = roll.at(origin);
                state.draft = points::add(curve, tick, bpm);
                // A new point at an occupied tick edits that point instead.
                let base = state.draft.as_ref().or(curve);
                if let Some(c) = base {
                    let index = c
                        .points
                        .partition_point(|p| p.tick < tick.max(0.0).round() as u64)
                        .min(c.points.len() - 1);
                    state.drag = Some(Drag {
                        target: Target::Point,
                        index,
                        offset: egui::Vec2::ZERO,
                        axis: roll.axis,
                        blocked: None,
                    });
                }
            }
        }
    }
    if !handled && let Some(edited) = delete_after_collision(state, roll, response, curve) {
        return (None, None, Some(edited));
    }
    let pointer = response.interact_pointer_pos().or(response.hover_pos());
    let preview = state
        .drag
        .zip(state.draft.as_ref().or(curve))
        .zip(pointer)
        .map(|((drag, c), pos)| {
            let (tick, bpm) = roll.at(pos + drag.offset);
            points::moved(c, drag.target, drag.index, tick, bpm)
        });
    let hover = curve
        .zip(response.hover_pos())
        .and_then(|(c, pos)| roll.hit(c, pos));
    let mut edited = None;
    if !handled && response.drag_stopped_by(primary) {
        state.drag = None;
        state.draft = None;
        edited = preview.clone().filter(|p| Some(p) != curve).map(Some);
    } else if !handled && hover.is_none() && response.clicked() {
        edited = pointer
            .and_then(|pos| {
                let (tick, bpm) = roll.at(pos);
                points::add(curve, tick, bpm)
            })
            .map(Some);
    } else if !handled && response.secondary_clicked() {
        edited = curve
            .zip(hover)
            .filter(|(_, (kind, _))| *kind == Target::Point)
            .map(|(c, (_, i))| points::remove(c, i));
    }
    let hover = state.drag.map(|d| (d.target, d.index)).or(hover);
    (preview.filter(|_| state.drag.is_some()), hover, edited)
}

/// A held pointer past a neighbor for 500 ms removes only the dragged tempo point.
fn delete_after_collision(
    state: &mut State,
    roll: &Roll,
    response: &egui::Response,
    curve: Option<&Curve>,
) -> Option<Option<Curve>> {
    let drag = state.drag.as_mut()?;
    let c = state.draft.as_ref().or(curve)?;
    let held = response
        .ctx
        .input(|i| i.focused && i.pointer.primary_down());
    let direction = response.interact_pointer_pos().and_then(|pos| {
        let tick = roll.at(pos + drag.offset).0;
        if drag.target != Target::Point || !held {
            None
        } else if drag.index > 0 && tick < (c.points[drag.index - 1].tick + 1) as f64 {
            Some(-1)
        } else if c
            .points
            .get(drag.index + 1)
            .is_some_and(|p| tick > (p.tick - 1) as f64)
        {
            Some(1)
        } else {
            None
        }
    });
    let Some(direction) = direction else {
        drag.blocked = None;
        return None;
    };
    let now = response.ctx.input(|i| i.time);
    let since = match drag.blocked {
        Some((side, since)) if side == direction => since,
        _ => {
            drag.blocked = Some((direction, now));
            now
        }
    };
    let remaining = 0.5 - (now - since);
    if remaining > 0.0 {
        response
            .ctx
            .request_repaint_after(std::time::Duration::from_secs_f64(remaining));
        return None;
    }
    let edited = points::remove(c, drag.index);
    state.drag = None;
    state.draft = None;
    Some(edited)
}

pub(super) fn audition(app: &mut App, tick: u64) {
    crate::view::beat_audition::start(app, 0, tick);
}
