//! The track map draws a pattern container as it sounds, not as the stored records.
use super::super::Notes;
use crate::view::patterns::library;
use crate::view::tests::unplaced::{fixture, CONTAINER};

fn ticks_and_pitches(marks: &[super::super::notes::Mark]) -> Vec<(u64, u8)> {
    marks
        .iter()
        .map(|&(tick, _, pitch)| (tick, pitch))
        .collect()
}

/// `(own, others)` the map draws for a track, each as `(tick, pitch)`.
type Drawn = (Vec<(u64, u8)>, Vec<(u64, u8)>);

/// What the map draws for the container.
fn container(a: &crate::app::App) -> Drawn {
    let notes = Notes::read(a, CONTAINER);
    (
        ticks_and_pitches(&notes.own),
        ticks_and_pitches(&notes.others),
    )
}

#[test]
fn track_map_draws_nothing_of_a_container_outside_pattern_edit() {
    let (a, _) = fixture();
    assert_eq!(container(&a), (vec![], vec![]));
}

#[test]
fn track_map_draws_only_the_edited_body_during_pattern_edit() {
    let (mut a, p1) = fixture();
    library::edit(&mut a, p1).unwrap();
    assert_eq!(container(&a), (vec![(0, 60)], vec![]));
}
