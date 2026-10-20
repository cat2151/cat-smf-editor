//! Latest playback state only. HTTP never runs on the UI/audio threads.
use crate::app::App;
use mascot_render_protocol::{now_unix_ms, PlaybackRequest};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    pub enabled: bool,
    worker: Option<Worker>,
    last: Option<(Instant, bool, bool)>,
}
struct Worker {
    latest: Arc<Mutex<Option<PlaybackRequest>>>,
    wake: mpsc::SyncSender<()>,
}
impl Worker {
    fn start() -> Self {
        let mut failed = false;
        Self::spawn(move |request| {
            let result = mascot_render_client::sync_playback_at(
                mascot_render_client::mascot_render_server_address(),
                &request,
            );
            if let Err(error) = &result {
                if !failed {
                    crate::store::error_log::write("mascot", &format!("{error:#}"));
                }
            }
            failed = result.is_err();
        })
    }
    fn spawn(mut send: impl FnMut(PlaybackRequest) + Send + 'static) -> Self {
        let (wake, rx) = mpsc::sync_channel(1);
        let latest = Arc::new(Mutex::new(None::<PlaybackRequest>));
        let pending = Arc::clone(&latest);
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                let request = pending.lock().unwrap().take();
                if let Some(request) = request {
                    send(request);
                }
            }
        });
        Self { latest, wake }
    }
    fn publish(&self, request: PlaybackRequest) {
        *self.latest.lock().unwrap() = Some(request);
        let _ = self.wake.try_send(());
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.publish(stopped());
    }
}
fn stopped() -> PlaybackRequest {
    PlaybackRequest {
        sent_unix_ms: now_unix_ms(),
        playing: false,
        beat: None,
        beats_per_second: 2.0,
        singing: false,
    }
}
pub fn toggle(app: &mut App) {
    app.mascot.enabled = !app.mascot.enabled;
    app.config.select_mascot_enabled(app.mascot.enabled);
    update(app);
}
pub fn update(app: &mut App) {
    let request = if app.mascot.enabled {
        snapshot(app)
    } else {
        stopped()
    };
    let state = &mut app.mascot;
    if state.worker.is_none() && !request.playing {
        return;
    }
    if !state.enabled && state.last.is_some_and(|(_, playing, _)| !playing) {
        return;
    }
    let now = Instant::now();
    let due = state.last.is_none_or(|(at, playing, singing)| {
        now.duration_since(at) >= Duration::from_millis(100)
            || playing != request.playing
            || singing != request.singing
    });
    if due {
        state.last = Some((now, request.playing, request.singing));
        state
            .worker
            .get_or_insert_with(Worker::start)
            .publish(request);
    }
}
fn snapshot(app: &App) -> PlaybackRequest {
    let Some(active) = &app.playback.active else {
        return stopped();
    };
    let Some(elapsed) = app
        .audition
        .seconds()
        .filter(|s| *s > 0.0 && *s < active.end)
    else {
        return stopped();
    };
    let Some(loaded) = &app.loaded else {
        return stopped();
    };
    state_at(active, &loaded.music, elapsed)
}
fn state_at(
    active: &super::transport::Active,
    music: &cat_smf_editor::model::Music,
    elapsed: f64,
) -> PlaybackRequest {
    let timing = active
        .tempos
        .get(active.clock_track)
        .and_then(|m| m.beat_clock(active.start_seconds + elapsed, music.division));
    let singing = active.sources.iter().any(|(track, sequence)| {
        music
            .tracks
            .get(*track)
            .and_then(cat_smf_editor::voicevox::Voice::read)
            .is_some_and(|voice| {
                voice.speaker_uuid == cat_smf_editor::voicevox::Voice::default().speaker_uuid
            })
            && sequence
                .notes
                .iter()
                .any(|n| n.start <= elapsed && elapsed < n.start + n.duration)
    });
    PlaybackRequest {
        sent_unix_ms: now_unix_ms(),
        playing: true,
        beat: timing.map(|t| t.0),
        beats_per_second: timing.map_or(2.0, |t| t.1),
        singing,
    }
}
#[cfg(test)]
mod tests;
