//! A Song overview never mistakes the currently edited body for a placed occurrence.
use super::super::panes::Panes;
use super::*;
use crate::view::patterns::library;
use crate::view::piano_roll::Layout;
use crate::view::tests::unplaced::{fixture, CONTAINER};

/// `(tick, pitch)` of the notes a roll draws, scaled to any pane height.
pub(super) fn drawn(layout: &Layout) -> Vec<(u64, u8)> {
    let row = layout.size.y / f32::from(layout.high - layout.low + 1);
    let mut notes: Vec<_> = layout
        .notes
        .iter()
        .map(|(_, r)| {
            (
                (r.left() / layout.scale).round() as u64,
                layout.high - (r.top() / row).round() as u8,
            )
        })
        .collect();
    notes.sort();
    notes
}

/// `(overview, container pane, container kind)` the overview draws.
type Drawn = (Vec<(u64, u8)>, Vec<(u64, u8)>, Kind);

fn performed(a: &App) -> Drawn {
    let panes = Panes::read(a, 0).unwrap();
    (
        drawn(&panes.overview),
        drawn(&panes.layouts[CONTAINER]),
        panes.kinds[CONTAINER],
    )
}

#[test]
fn overview_draws_nothing_of_a_container_outside_pattern_edit() {
    let (a, _) = fixture();
    assert_eq!(performed(&a), (vec![], vec![], Kind::Empty));
}

#[test]
fn overview_never_confuses_an_unplaced_edit_with_a_song_occurrence() {
    let (mut a, p1) = fixture();
    library::edit(&mut a, p1).unwrap();
    assert_eq!(performed(&a), (vec![], vec![], Kind::Empty));
}
