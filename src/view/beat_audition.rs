//! Move the cursor to a beat and audition that beat across all tracks.
use crate::app::App;

pub(crate) fn start(app: &mut App, track: usize, tick: u64) {
    if crate::play::transport::timeline::running(app) {
        crate::play::transport::timeline::seek(app, track, tick);
        return;
    }
    let Some(map) = app.loaded.as_ref().and_then(|l| l.time.map(track)) else {
        return;
    };
    let mut position = map.position(tick);
    position.tick_numerator = 0;
    position.tick_denominator = 1;
    let Some(a) = map.tick_at(&position) else {
        return;
    };
    crate::cursor::split::go(app, track, a);
    crate::play::transport::timeline::audition(app, track, a);
}
