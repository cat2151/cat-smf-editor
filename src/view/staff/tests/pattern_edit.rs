//! The staff draws a pattern container as it sounds, not as the stored records.
use super::super::{notation, score};
use crate::app::App;
use crate::play::playback::performance as sounding;
use crate::view::patterns::library;
use crate::view::tests::unplaced::{fixture, CONTAINER};

/// `(tick, pitch)` of the notes the staff draws for `track`.
fn drawn(a: &App, track: usize) -> Vec<(u64, u8)> {
    let loaded = a.loaded.as_ref().unwrap();
    score::notes(loaded, track, &sounding(a)[track])
        .iter()
        .map(|n| {
            let pitch = (0..=127u8)
                .find(|&p| notation::spell(p, 0) == n.spelled)
                .unwrap();
            (n.tick, pitch)
        })
        .collect()
}

#[test]
fn staff_draws_nothing_of_a_container_outside_pattern_edit() {
    let (a, _) = fixture();
    assert_eq!(drawn(&a, CONTAINER), vec![]);
}

#[test]
fn staff_draws_only_the_edited_body_during_pattern_edit() {
    let (mut a, p1) = fixture();
    library::edit(&mut a, p1).unwrap();
    assert_eq!(drawn(&a, CONTAINER), vec![(0, 60)]);
}

#[test]
fn staff_minimap_draws_the_edited_body_without_gray_references_during_pattern_edit() {
    let (mut a, p1) = fixture();
    library::edit(&mut a, p1).unwrap();
    let loaded = a.loaded.as_ref().unwrap();
    let overview =
        crate::view::piano_roll::layout_from(loaded, &sounding(&a), &[CONTAINER], CONTAINER, 0);
    let notes: Vec<_> = overview
        .notes
        .iter()
        .map(|(_, r)| {
            let row = (r.top() / crate::view::piano_roll::ROW_HEIGHT).round() as u8;
            (
                (r.left() / overview.scale).round() as u64,
                overview.high - row,
            )
        })
        .collect();
    assert_eq!((notes, overview.references.len()), (vec![(0, 60)], 0));
}
