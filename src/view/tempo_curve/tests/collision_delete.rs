use super::edit::{hold, release};
use super::*;

fn curve(app: &crate::app::App) -> Curve {
    tempo_curve::read(&app.loaded.as_ref().unwrap().music)
        .unwrap()
        .unwrap()
}

fn marks(output: &egui::FullOutput) -> Vec<egui::Pos2> {
    circles(output, POINT)
        .into_iter()
        .map(|(pos, _, _)| pos)
        .collect()
}

#[test]
fn pushing_past_either_neighbor_for_half_a_second_deletes_once_before_release() {
    for right in [false, true] {
        let mut app = app(with_curve());
        let (ctx, output) = opened_in(&mut app, 0);
        let points = marks(&output);
        let before = curve(&app);
        let generation = app.generation;
        let to = if right {
            points[2] + egui::vec2(30.0, 0.0)
        } else {
            points[0] - egui::vec2(30.0, 0.0)
        };
        hold(&mut app, &ctx, 11.0, points[1], to);
        at(&mut app, &ctx, 11.529, vec![]);
        assert_eq!(curve(&app), before);
        at(&mut app, &ctx, 11.531, vec![]);
        let after = curve(&app);
        assert_eq!(after.points, vec![before.points[0], before.points[2]]);
        assert!(after.validate().is_ok());
        assert_eq!(app.generation, generation + 1);
        at(&mut app, &ctx, 11.9, vec![]);
        release(&mut app, &ctx, 12.0, to);
        assert_eq!(curve(&app), after);
        assert_eq!(app.generation, generation + 1);
    }
}

#[test]
fn returning_inside_or_switching_collision_direction_restarts_the_timer() {
    for switch_side in [false, true] {
        let mut app = app(with_curve());
        let (ctx, output) = opened_in(&mut app, 0);
        let points = marks(&output);
        let before = curve(&app);
        let right = points[2] + egui::vec2(30.0, 0.0);
        hold(&mut app, &ctx, 11.0, points[1], right);
        let to = if switch_side {
            points[0] - egui::vec2(30.0, 0.0)
        } else {
            points[1]
        };
        at(&mut app, &ctx, 11.3, vec![egui::Event::PointerMoved(to)]);
        let to = if switch_side { to } else { right };
        at(&mut app, &ctx, 11.4, vec![egui::Event::PointerMoved(to)]);
        at(&mut app, &ctx, 11.79, vec![]);
        assert_eq!(curve(&app), before);
        at(&mut app, &ctx, 11.91, vec![]);
        assert_eq!(curve(&app).points.len(), 2);
        release(&mut app, &ctx, 12.0, to);
        assert_eq!(curve(&app).points.len(), 2);
    }
}

#[test]
fn controls_and_outer_edges_without_neighbors_do_not_delete() {
    for (index, dx) in [(0, -10000.0), (2, 10000.0), (3, 10000.0)] {
        let mut app = app(with_curve());
        let (ctx, output) = opened_in(&mut app, 0);
        let before = curve(&app);
        let from = if index == 3 {
            squares(&output, CONTROL)[0].0.center()
        } else {
            marks(&output)[index]
        };
        let to = from + egui::vec2(dx, 0.0);
        hold(&mut app, &ctx, 11.0, from, to);
        at(&mut app, &ctx, 12.0, vec![]);
        assert_eq!(curve(&app), before);
        release(&mut app, &ctx, 12.1, to);
        assert_eq!(curve(&app).points.len(), 3);
    }
}
