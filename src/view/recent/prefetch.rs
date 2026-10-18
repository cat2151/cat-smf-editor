//! Only while the overlay is open: each file's first measure becomes a WAV named by the
//! SMF's hash. Never installs a document, records history, or sends notes to the editor.
use crate::{
    app::App,
    play::{
        audio_sequence::Sequence,
        audition::offline::{Instruments, CHANNELS},
    },
};
use cat_smf_editor::{
    master_effects,
    score::{instrument::Patch, note_view::note_on, state_cache::Cache},
};
use eframe::egui;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, atomic::Ordering, mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

/// Plugins open on the UI thread, so they wait for a pause in cursor movement.
const IDLE: Duration = Duration::from_millis(750);
#[derive(Clone, PartialEq, Eq)]
struct Stamp(Option<(u64, SystemTime)>);
fn stamp(path: &Path) -> Stamp {
    Stamp(
        std::fs::metadata(path)
            .ok()
            .and_then(|m| Some((m.len(), m.modified().ok()?))),
    )
}

enum Status {
    Missing,
    Ready,
    Failed(String),
}
struct Known {
    stamp: Stamp,
    player: Option<PathBuf>,
    wav: PathBuf,
    status: Status,
}

struct Range {
    parts: Vec<(usize, Patch, Sequence)>,
    gains: Vec<f32>,
    master: master_effects::Settings,
    states: Cache,
}
enum Phase {
    Loading(mpsc::Receiver<Result<Range, String>>),
    Loaded(Result<Range, String>),
    Rendering {
        thread: Option<JoinHandle<Option<Result<(), String>>>>,
        /// None for test auditions, which render silence.
        instruments: Option<Instruments>,
    },
}
struct Work {
    path: PathBuf,
    known: (Stamp, Option<PathBuf>),
    wav: PathBuf,
    cancel: Arc<AtomicBool>,
    phase: Phase,
}
impl Drop for Work {
    /// The render thread stops before its instruments are dropped.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Phase::Rendering { thread, .. } = &mut self.phase
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    last_input: Option<Instant>,
    known: HashMap<PathBuf, Known>,
    work: Option<Work>,
    /// Replaces the cache directory in tests.
    pub(super) directory: Option<PathBuf>,
}

fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Rereads the SMF only when its size, time or the player changed.
fn refresh<'a>(app: &'a mut App, path: &Path) -> Result<&'a Known, String> {
    let player = app.player_selection.executable.clone();
    let state = &mut app.recent.prefetch;
    let current = stamp(path);
    if !state
        .known
        .get(path)
        .is_some_and(|k| k.stamp == current && k.player == player)
    {
        let directory = state
            .directory
            .clone()
            .or_else(|| {
                cat_smf_editor::storage_paths::cache_directory().map(|c| c.join("recent-preview"))
            })
            .ok_or("Cache directory unavailable")?;
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let wav = directory.join(format!("{}.wav", hash(&bytes)));
        let status = if wav.exists() {
            Status::Ready
        } else {
            Status::Missing
        };
        state.known.insert(
            path.to_owned(),
            Known {
                stamp: current,
                player,
                wav,
                status,
            },
        );
    }
    Ok(&state.known[path])
}

/// The WAV to play: None while it is not rendered yet.
pub(super) fn wav(app: &mut App, path: &Path) -> Result<Option<PathBuf>, String> {
    let known = refresh(app, path)?;
    match &known.status {
        Status::Ready => Ok(Some(known.wav.clone())),
        Status::Missing => Ok(None),
        Status::Failed(error) => Err(error.clone()),
    }
}

pub(super) fn status(app: &App, path: &Path) -> &'static str {
    let state = &app.recent.prefetch;
    if state.work.as_ref().is_some_and(|w| w.path == path) {
        return "準備中…";
    }
    match state.known.get(path).map(|k| &k.status) {
        Some(Status::Ready) => "準備済み",
        Some(Status::Failed(_)) => "準備失敗",
        _ => "",
    }
}

/// From the first note of any track, one measure of the conductor's meter.
fn first_measure(app: &App) -> Option<(u64, u64)> {
    let loaded = app.loaded.as_ref()?;
    let first = loaded
        .music
        .tracks
        .iter()
        .filter_map(|t| t.events.iter().find(|e| note_on(e)).map(|e| e.tick))
        .min()?;
    let map = loaded.time.map(0)?;
    let mut position = map.position(first);
    position.measure += 1;
    // A shorter next measure has no such beat; its end is then the measure after it.
    let end = map.tick_at(&position).or_else(|| {
        position.measure += 1;
        position.beat = 1;
        position.tick_numerator = 0;
        position.tick_denominator = 1;
        map.tick_at(&position)
    })?;
    Some((first, end))
}

fn load(
    path: PathBuf,
    player: Option<PathBuf>,
    roots: crate::player::patch_roots::Cache,
    states: Cache,
) -> Result<Range, String> {
    let mut preview = App::empty();
    preview.player_selection.executable = player;
    preview.patch_roots = roots;
    preview.state_cache = states;
    preview.load_mid_unchecked(path);
    if let Some(error) = preview.load_error.take() {
        return Err(error);
    }
    let range = first_measure(&preview).ok_or("No notes")?;
    let prepared = crate::play::transport::repeat::prepare(&preview, range).ok_or("No notes")?;
    let tracks = preview.loaded.as_ref().map_or(0, |l| l.music.tracks.len());
    Ok(Range {
        parts: prepared.parts,
        gains: (0..tracks).map(|t| preview.audition.gain(t)).collect(),
        master: preview.audition.master().clone(),
        states: preview.state_cache,
    })
}

fn output_rate() -> u32 {
    use cpal::traits::{DeviceTrait, HostTrait};
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.default_output_config().ok())
        .map_or(48000, |c| c.sample_rate().0)
}

/// Opens the plugins here and renders on a worker thread.
fn render(
    app: &mut App,
    range: Range,
    wav: PathBuf,
    cancel: Arc<AtomicBool>,
) -> Result<Phase, String> {
    app.state_cache.absorb(range.states);
    let disabled = app.audition.is_disabled();
    let rate = if disabled { 48000 } else { output_rate() };
    let instruments = if disabled {
        None
    } else {
        Some(Instruments::open(rate, &range.parts, &range.master)?)
    };
    let frames = range
        .parts
        .iter()
        .map(|(_, _, s)| s.end)
        .fold(0.0, f64::max);
    let silent = (frames * f64::from(rate)).round() as usize * CHANNELS;
    // SAFETY: Work joins this thread before dropping the instruments.
    let job = instruments.as_ref().map(|i| unsafe {
        i.job(
            rate,
            range.parts,
            &range.gains,
            !app.audition.auto_gain_enabled(),
        )
    });
    let thread = std::thread::Builder::new()
        .name("recent-render".into())
        .spawn(move || {
            let samples = match job {
                Some(job) => job.run(&cancel)?,
                None => Ok(vec![0.0; silent]),
            };
            Some(samples.and_then(|s| crate::play::wav::write(&wav, rate, &s)))
        })
        .map_err(|e| e.to_string())?;
    Ok(Phase::Rendering {
        thread: Some(thread),
        instruments,
    })
}

pub(crate) fn update(app: &mut App, ctx: &egui::Context) {
    if !app.recent.open || app.player_selection.setup.is_active() {
        app.recent.prefetch.work = None;
        app.recent.prefetch.last_input = None;
        return;
    }
    let now = Instant::now();
    let input = ctx.input(|i| !i.events.is_empty() || i.pointer.any_down());
    let last = app.recent.prefetch.last_input.get_or_insert(now);
    if input {
        *last = now;
    }
    let wait = IDLE.saturating_sub(now.saturating_duration_since(*last));
    if let Some(mut work) = app.recent.prefetch.work.take() {
        let finished = match &mut work.phase {
            Phase::Loading(receiver) => {
                match receiver.try_recv() {
                    Ok(result) => work.phase = Phase::Loaded(result),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        work.phase = Phase::Loaded(Err("Prefetch worker stopped".into()))
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                None
            }
            Phase::Rendering {
                thread,
                instruments,
            } => {
                if let Some(instruments) = instruments {
                    instruments.pump();
                }
                if thread.as_ref().is_some_and(|t| t.is_finished()) {
                    let result = thread.take().unwrap().join();
                    Some(match result {
                        Ok(Some(result)) => result,
                        _ => Err("Render stopped".into()),
                    })
                } else {
                    None
                }
            }
            Phase::Loaded(_) => None,
        };
        if wait.is_zero()
            && matches!(work.phase, Phase::Loaded(_))
            && let Phase::Loaded(result) =
                std::mem::replace(&mut work.phase, Phase::Loaded(Err(String::new())))
        {
            let current = (stamp(&work.path), app.player_selection.executable.clone());
            if current != work.known {
                // Changed while loading: the next frame rereads it.
                ctx.request_repaint();
                return;
            }
            match result.and_then(|r| render(app, r, work.wav.clone(), work.cancel.clone())) {
                Ok(phase) => work.phase = phase,
                Err(error) => {
                    finish(app, &work, Err(error));
                    ctx.request_repaint();
                    return;
                }
            }
        }
        if let Some(result) = finished {
            finish(app, &work, result);
            ctx.request_repaint();
            return;
        }
        app.recent.prefetch.work = Some(work);
        ctx.request_repaint_after(
            Duration::from_millis(16).max(wait.min(Duration::from_millis(100))),
        );
        return;
    }
    let listened = app.recent.listen.clone();
    let paths: Vec<_> = listened
        .into_iter()
        .chain(app.recent.history.entries.iter().map(|e| e.path.clone()))
        .collect();
    let candidate = paths.into_iter().find_map(|path| {
        let known = refresh(app, &path).ok()?;
        matches!(known.status, Status::Missing).then(|| {
            (
                (known.stamp.clone(), known.player.clone()),
                known.wav.clone(),
                path,
            )
        })
    });
    let Some((known, wav, path)) = candidate else {
        return;
    };
    let roots = app.patch_roots.detached();
    let states = app.state_cache.clone();
    let (sender, receiver) = mpsc::channel();
    let wake = ctx.clone();
    let worker_path = path.clone();
    let player = known.1.clone();
    let spawn = std::thread::Builder::new()
        .name("recent-prefetch".into())
        .spawn(move || {
            let _ = sender.send(load(worker_path, player, roots, states));
            wake.request_repaint();
        });
    if spawn.is_ok() {
        app.recent.prefetch.work = Some(Work {
            path,
            known,
            wav,
            cancel: Arc::new(AtomicBool::new(false)),
            phase: Phase::Loading(receiver),
        });
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

/// A file that changed meanwhile keeps the status of its new hash.
fn finish(app: &mut App, work: &Work, result: Result<(), String>) {
    if let Some(known) = app.recent.prefetch.known.get_mut(&work.path)
        && known.wav == work.wav
    {
        known.status = match result {
            Ok(()) => Status::Ready,
            Err(error) => Status::Failed(error),
        };
    }
    super::listen(app);
}

#[cfg(test)]
mod tests;
