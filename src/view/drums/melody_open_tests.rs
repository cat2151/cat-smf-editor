//! Which entries open an ordinary container's pattern on the step grid.
use super::melody_tests::{frame, ordinary_pattern, plain, shift};
use super::*;
#[test]
fn a_container_opened_on_the_grid_reopens_it_for_every_pattern_and_after_a_restart() {
    let mut a = ordinary_pattern();
    let first = crate::view::patterns::mode::editing(&a, a.track)
        .unwrap()
        .pattern;
    crate::view::patterns::library::create(&mut a, None).unwrap();
    let catalog =
        cat_smf_editor::score::pattern::Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let second = *catalog.patterns.keys().find(|&&id| id != first).unwrap();
    let edit = |a: &mut App, id: u64| crate::view::patterns::library::edit(a, id).unwrap();
    let shown = |a: &App, id: u64| {
        is_open(a)
            && melody::active(a)
            && crate::view::patterns::mode::editing(a, a.track).is_some_and(|c| c.pattern == id)
    };
    // Never opened on the grid: a candidate stays on the event list.
    edit(&mut a, second);
    assert!(!is_open(&a));
    edit(&mut a, first);
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift(&mut a, &ctx, Key::D);
    assert!(shown(&a, first));
    // A candidate click (`library::edit`) of the same container opens the grid.
    edit(&mut a, second);
    assert!(shown(&a, second), "{:?}", a.load_error);
    // The session carries it over a restart.
    let session = a.session().unwrap();
    let mut restarted = crate::view::tests::app(a.loaded.as_ref().unwrap().music.clone());
    restarted.restore_position(&session);
    assert!(shown(&restarted, second), "{:?}", restarted.load_error);
    // Quitting on the candidate cursor rather than inside a pattern opens it as well.
    let mut browsing = session.clone();
    browsing.edited_pattern = None;
    let mut restarted = crate::view::tests::app(a.loaded.as_ref().unwrap().music.clone());
    restarted.restore_position(&browsing);
    assert!(
        is_open(&restarted) && melody::active(&restarted),
        "{:?}",
        restarted.load_error
    );
    // Escape forgets it.
    frame(&mut a, &ctx, vec![]);
    plain(&mut a, &ctx, Key::Escape);
    assert!(!is_open(&a));
    edit(&mut a, first);
    assert!(!is_open(&a));
    assert!(a.session().unwrap().step_containers.is_empty());
}
