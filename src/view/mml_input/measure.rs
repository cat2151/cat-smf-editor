//! The current measure becomes the A–B range, and MML typed into the overlay
//! overwrites the notes under it.
use super::*;

pub(crate) fn begin(app: &mut App, ctx: &egui::Context) {
    let Some(range) = range(app) else {
        return;
    };
    crate::cursor::moves::at_tick(app, range.0);
    crate::play::transport::repeat::start(app, range);
    app.mml_input.measure = Some(range);
    crate::view::busy::request(app, ctx, crate::view::busy::Task::MmlMeasure);
}

/// An empty measure has nothing to loop when the input opens, so the previous loop goes on until
/// the typed notes can loop.
pub(super) fn follow(app: &mut App, state: &State) {
    if let Some(range) = state.measure
        && state.repeat_session
        && app.playback.repeat != Some(range)
    {
        crate::play::transport::repeat::start(app, range);
    }
}

/// The cursor's measure, start inclusive and end exclusive.
pub(crate) fn range(app: &App) -> Option<(u64, u64)> {
    let map = app.loaded.as_ref()?.time.map(app.track)?;
    let mut position = crate::play::playback::location(app).1?;
    position.beat = 1;
    position.tick_numerator = 0;
    position.tick_denominator = 1;
    let start = map.tick_at(&position)?;
    position.measure += 1;
    Some((start, map.tick_at(&position)?))
}
