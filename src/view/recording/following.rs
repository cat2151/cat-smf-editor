//! Pattern navigation chooses the keyboard's container without navigating back to its Loop.
use super::*;

pub(crate) fn pattern(a: &mut App, id: u64) {
    if !a.patterns.recording.is_open() {
        return;
    }
    let container = a
        .loaded
        .as_ref()
        .and_then(|l| l.arrangement.as_ref())
        .and_then(|(c, _)| c.patterns.get(&id))
        .map(|p| p.container);
    a.patterns.recording.follow = container;
    apply(a);
}

pub(super) fn apply(a: &mut App) {
    let Some(container) = a.patterns.recording.follow else {
        return;
    };
    let mut state = std::mem::take(&mut a.patterns.recording);
    if let Some(s) = state.session.as_mut()
        && !play::armed(s)
    {
        state.follow = None;
        if !phrase::follow_patch(a, s)
            && let Some(index) = s.sounds.iter().position(|x| x.container == Some(container))
            && index != s.selected
        {
            setup::select_following(a, s, index);
        }
    }
    a.patterns.recording = state;
}
