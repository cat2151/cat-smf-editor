use super::*;
fn voice(character: &str, style: u32) -> Voice {
    Voice {
        name: character.into(),
        speaker_uuid: character.into(),
        style_id: style,
        style_name: style.to_string(),
    }
}
fn drain(queue: &mut Queue) -> Vec<u32> {
    std::iter::from_fn(|| queue.pop().map(|v| v.style_id)).collect()
}
#[test]
fn reservations_cancel_promote_deduplicate_and_remove_obsolete_voices() {
    let (a, b, c) = (voice("a", 1), voice("b", 2), voice("c", 3));
    let mut q = Queue::default();
    for v in [&a, &b, &a, &c] {
        q.reserve(v.clone());
    }
    q.voices.push(b.clone());
    q.deduplicate();
    q.cancel(&a);
    q.front(c.clone());
    assert_eq!(drain(&mut q), [3, 2]);
    for v in [&a, &b, &c] {
        q.reserve(v.clone());
    }
    q.retain_catalog(&[a, c]);
    assert_eq!(drain(&mut q), [1, 3]);
    q.reserve(b);
    q.clear();
    assert!(q.is_empty());
}
#[test]
fn priorities_reorder_for_cursor_filter_visibility_and_stable_catalog_order() {
    let catalog = vec![
        voice("a", 1),
        voice("a", 2),
        voice("b", 3),
        voice("b", 4),
        voice("c", 5),
    ];
    let mut q = Queue::default();
    for v in &catalog {
        q.reserve(v.clone());
    }
    q.sort(Some(&catalog[3]), &[catalog[4].clone()], &catalog);
    assert_eq!(
        q.voices.iter().map(|v| v.style_id).collect::<Vec<_>>(),
        [4, 3, 5, 1, 2]
    );
    q.sort(Some(&catalog[1]), &[catalog[3].clone()], &catalog);
    assert_eq!(
        q.voices.iter().map(|v| v.style_id).collect::<Vec<_>>(),
        [2, 1, 4, 3, 5]
    );
    // A changed filter/scroll reprioritizes visible rows while keeping the same selection.
    q.sort(Some(&catalog[1]), &[catalog[4].clone()], &catalog);
    assert_eq!(drain(&mut q), [2, 1, 5, 3, 4]);
}
