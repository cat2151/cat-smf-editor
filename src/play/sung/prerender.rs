//! 再生の前に、歌う track を VOICEVOX で WAV にする。engine の起動と合成は別 thread で行う。
use super::{take, Take};
use crate::{app::App, play::transport::Prepared};
use cat_smf_editor::voicevox::{
    cache,
    engine::{self, Engine, Failure},
    is_singer,
    score::{self, one_note, Score},
    Voice,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;

mod scores;
pub(crate) use scores::scores;

#[derive(Default)]
pub struct State {
    job: Option<JoinHandle<Outcome>>,
    connecting: Arc<AtomicBool>,
    /// cache key ごとの歌った WAV。直前の再生で使ったものだけを残す。
    sung: HashMap<String, Arc<Vec<f32>>>,
    /// 歌えなかった key。この再生ではその track を黙らせ、次の再生でやり直す。
    failed: HashSet<String>,
    /// job の実行中に歌詞が確定した。job が終わったら、もう一度歌わせる。
    again: bool,
    /// 試聴の1音 (pitch, 譜面の frame 数) ごとの歌った WAV。
    notes: HashMap<(Voice, u8, u64), Arc<Vec<f32>>>,
    /// 次の job で歌わせる1音。
    queued: Vec<(Voice, u8, u64)>,
    /// 実行中の job が歌っている1音の cache key。
    singing: HashMap<String, (Voice, u8, u64)>,
    /// 試聴が歌わせようとした cache key。歌えない間、試聴のたびに engine を起こさない。
    asked: HashSet<String>,
    /// test では engine の port と cache の場所を、利用者のものから外す。
    #[cfg(test)]
    pub(crate) target: Option<(u16, PathBuf)>,
    /// test では engine の代わりに歌う。
    #[cfg(test)]
    pub(crate) sing: Option<Sing>,
}

#[cfg(test)]
pub(crate) type Sing = fn(&str, u32) -> Result<Vec<u8>, String>;

pub struct Request {
    pub port: u16,
    pub configured: Option<PathBuf>,
    pub directory: PathBuf,
    pub rate: u32,
    pub scores: Vec<(String, Score)>,
    #[cfg(test)]
    pub sing: Option<Sing>,
}

pub struct Outcome {
    pub engine: Option<Engine>,
    pub sung: Vec<(String, Result<Vec<f32>, String>)>,
    pub no_executable: bool,
}

/// cache に無い譜面だけ engine に歌わせる。engine は要るときだけ繋ぎ、繋いだものを返す。
pub(crate) fn run(
    request: Request,
    mut engine: Option<Engine>,
    connecting: Arc<AtomicBool>,
) -> Outcome {
    let mut no_executable = false;
    let mut sung = Vec::new();
    for (key, score) in request.scores {
        let result = cache::load_or_sing(&request.directory, &score, request.rate, |json, rate| {
            #[cfg(test)]
            if let Some(sing) = request.sing {
                return sing(json, rate);
            }
            if engine.as_ref().is_none_or(|e| e.client.version().is_err()) {
                engine = None;
                connecting.store(true, Ordering::Relaxed);
                let connected = engine::connect(
                    request.port,
                    request.configured.as_deref(),
                    engine::START_TIMEOUT,
                )
                .map_err(|failure| match failure {
                    Failure::NoExecutable => {
                        no_executable = true;
                        "VOICEVOX engine is not set".to_owned()
                    }
                    Failure::Failed(error) => error,
                })?;
                engine = Some(connected);
                connecting.store(false, Ordering::Relaxed);
            }
            engine
                .as_ref()
                .expect("connected")
                .client
                .sing_as(json, rate, &score.voice)
        });
        sung.push((key, result));
    }
    Outcome {
        engine,
        sung,
        no_executable,
    }
}

/// 歌う track の take。まだ歌い終わっていなければ `None`（裏で歌わせている）。
pub fn takes(app: &mut App, prepared: &Prepared) -> Option<Vec<(usize, Arc<Take>)>> {
    if crate::view::voicevox::singer::running(app) {
        return None;
    }
    let singers = scores(app, prepared);
    if singers.is_empty() {
        return Some(Vec::new());
    }
    if app
        .voicevox
        .prerender
        .job
        .as_ref()
        .is_some_and(|job| !job.is_finished())
    {
        return None;
    }
    finish(app);
    let rate = app.audition.rate();
    let keyed: Vec<(usize, String, Score)> = singers
        .into_iter()
        .map(|(track, score)| (track, cache::key(&score, rate), score))
        .collect();
    let state = &mut app.voicevox.prerender;
    let missing: Vec<(String, Score)> = keyed
        .iter()
        .filter(|(_, key, _)| !state.sung.contains_key(key) && !state.failed.contains(key))
        .map(|(_, key, score)| (key.clone(), score.clone()))
        .collect();
    if !missing.is_empty() {
        return match spawn(app, missing, rate) {
            Ok(()) => None,
            Err(error) => {
                app.voicevox.error = Some(error);
                Some(silent(&keyed))
            }
        };
    }
    let state = &mut app.voicevox.prerender;
    let used: HashSet<&String> = keyed.iter().map(|(_, key, _)| key).collect();
    state.sung.retain(|key, _| used.contains(key));
    state.failed.clear();
    let start = prepared.active.start_seconds;
    Some(
        keyed
            .iter()
            .map(|(track, key, score)| {
                let sequence = prepared.parts.iter().find(|(t, _, _)| t == track);
                let made = match (state.sung.get(key), sequence) {
                    (Some(samples), Some((_, _, sequence))) => {
                        take(score, samples.clone(), rate, sequence, start)
                    }
                    _ => Take::silent(),
                };
                (*track, Arc::new(made))
            })
            .collect(),
    )
}

/// cache に無い譜面を裏の job で歌わせる。
fn spawn(app: &mut App, missing: Vec<(String, Score)>, rate: u32) -> Result<(), String> {
    #[cfg(not(test))]
    let port = engine::PORT;
    #[cfg(test)]
    let port = app.voicevox.prerender.target.as_ref().map_or(0, |t| t.0);
    let directory = directory(app).ok_or("VOICEVOX: no cache directory")?;
    let request = Request {
        port,
        configured: app.config.voicevox_engine.clone(),
        directory,
        rate,
        scores: missing,
        #[cfg(test)]
        sing: app.voicevox.prerender.sing,
    };
    let engine = app.voicevox.engine.take();
    let connecting = Arc::new(AtomicBool::new(false));
    app.voicevox.prerender.connecting = connecting.clone();
    match std::thread::Builder::new()
        .name("voicevox".into())
        .spawn(move || run(request, engine, connecting))
    {
        Ok(job) => {
            app.voicevox.prerender.job = Some(job);
            Ok(())
        }
        Err(error) => Err(format!("VOICEVOX: {error}")),
    }
}

/// 歌った WAV の保存先。
#[cfg(not(test))]
pub fn directory(_: &App) -> Option<PathBuf> {
    cache::directory()
}

/// test では利用者の保存先から外す。
#[cfg(test)]
pub fn directory(app: &App) -> Option<PathBuf> {
    app.voicevox.prerender.target.as_ref().map(|t| t.1.clone())
}

/// 試聴の1音 (pitch, 譜面の frame 数) の歌った WAV。まだなら裏の job に回して `None`。
pub fn note(app: &mut App, voice: Voice, pitch: (u8, u64)) -> Option<Arc<Vec<f32>>> {
    let note = (voice.clone(), pitch.0, pitch.1);
    let state = &mut app.voicevox.prerender;
    if let Some(samples) = state.notes.get(&note) {
        return Some(samples.clone());
    }
    let mut score = one_note(pitch.0, pitch.1);
    score.voice = voice;
    let key = cache::key(&score, app.audition.rate());
    let state = &mut app.voicevox.prerender;
    let waiting = state.queued.contains(&note) || state.singing.values().any(|n| n == &note);
    if state.asked.insert(key) && !waiting {
        state.queued.push(note);
        if !running(app) {
            start(app, false);
        }
    }
    None
}

/// `track` の譜面と、その歌った WAV。まだなら一度だけ [`ahead`] で歌わせ始めて `None`。
pub fn song(app: &mut App, track: usize) -> Option<(Score, Arc<Vec<f32>>)> {
    let loaded = app.loaded.as_ref()?;
    if !singer(app, track) {
        return None;
    }
    let score = score::from_track(&loaded.music, track)?;
    let key = cache::key(&score, app.audition.rate());
    let state = &mut app.voicevox.prerender;
    if let Some(samples) = state.sung.get(&key) {
        return Some((score, samples.clone()));
    }
    if state.asked.insert(key) {
        ahead(app, track);
    }
    None
}

/// 歌詞を確定したとき、`track` が歌う track なら、再生を待たずに裏で歌わせておく。
pub fn ahead(app: &mut App, track: usize) {
    if !singer(app, track) {
        return;
    }
    if running(app) {
        app.voicevox.prerender.again = true;
        return;
    }
    start(app, true);
}

/// 毎 frame。再生を待たずに始めた job が終わったら結果を取り込み、その間に確定した歌詞の分を始める。
/// 再生の準備中は [`takes`] が取り込む。
pub fn poll(app: &mut App, ctx: &eframe::egui::Context) {
    if let Some(job) = &app.voicevox.prerender.job {
        if !job.is_finished() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        if app.playback.pending.is_some() {
            return;
        }
        finish(app);
        // 再生を待たずに歌わせた失敗は、次の再生でやり直す。
        app.voicevox.prerender.failed.clear();
    }
    let again = std::mem::take(&mut app.voicevox.prerender.again);
    if again || !app.voicevox.prerender.queued.is_empty() {
        start(app, again);
    }
}

/// 待っている試聴の1音と、`song` なら曲中の歌う track のうちまだ歌っていない譜面を歌わせる。
fn start(app: &mut App, song: bool) {
    if crate::view::voicevox::singer::running(app) {
        app.voicevox.prerender.again |= song;
        return;
    }
    let rate = app.audition.rate();
    let state = &mut app.voicevox.prerender;
    let mut missing: Vec<(String, Score)> = Vec::new();
    for (voice, pitch, frames) in std::mem::take(&mut state.queued) {
        let mut score = one_note(pitch, frames);
        score.voice = voice.clone();
        let key = cache::key(&score, rate);
        state.singing.insert(key.clone(), (voice, pitch, frames));
        missing.push((key, score));
    }
    if song && let Some(loaded) = &app.loaded {
        let state = &app.voicevox.prerender;
        missing.extend(
            (0..loaded.music.tracks.len())
                .filter(|&track| singer(app, track))
                .filter_map(|track| score::from_track(&loaded.music, track))
                .map(|score| (cache::key(&score, rate), score))
                .filter(|(key, _)| !state.sung.contains_key(key)),
        );
    }
    if missing.is_empty() {
        return;
    }
    if let Err(error) = spawn(app, missing, rate) {
        app.voicevox.prerender.singing.clear();
        app.voicevox.error = Some(error);
    }
}

/// Whether `track` sings when played: it has a voice and is not a pattern container that
/// playback keeps silent.
fn singer(app: &App, track: usize) -> bool {
    app.loaded.as_ref().is_some_and(|loaded| {
        track < loaded.music.tracks.len()
            && is_singer(&loaded.music.tracks[track])
            && !crate::play::playback::silent(app, track)
    })
}

fn silent(keyed: &[(usize, String, Score)]) -> Vec<(usize, Arc<Take>)> {
    keyed
        .iter()
        .map(|(track, _, _)| (*track, Arc::new(Take::silent())))
        .collect()
}

/// 終わった job の結果を取り込む。engine は app に戻し、editor の終了時に落とす。
fn finish(app: &mut App) {
    let Some(job) = app.voicevox.prerender.job.take() else {
        return;
    };
    let outcome = match job.join() {
        Ok(outcome) => outcome,
        Err(_) => {
            app.voicevox.error = Some("VOICEVOX: prerender thread panicked".into());
            return;
        }
    };
    if outcome.engine.is_some() {
        app.voicevox.engine = outcome.engine;
    }
    let state = &mut app.voicevox.prerender;
    if outcome.sung.iter().any(|(_, result)| result.is_ok()) {
        // engine が歌えるので、歌えなかった試聴もまた頼めるようにする。
        state.asked.clear();
    }
    for (key, result) in outcome.sung {
        let note = app.voicevox.prerender.singing.remove(&key);
        match (result, note) {
            (Ok(samples), Some(note)) => {
                app.voicevox.prerender.notes.insert(note, Arc::new(samples));
            }
            (Ok(samples), None) => {
                app.voicevox.prerender.sung.insert(key, Arc::new(samples));
            }
            (Err(error), Some(_)) => {
                if !outcome.no_executable {
                    app.voicevox.error = Some(error);
                }
            }
            (Err(error), None) => {
                app.voicevox.prerender.failed.insert(key);
                if !outcome.no_executable {
                    app.voicevox.error = Some(error);
                }
            }
        }
    }
    if outcome.no_executable {
        crate::view::voicevox::request_executable(app);
    }
}

/// 歌わせている間。起動を待つ側はこの間を時間切れに数えない。
pub fn running(app: &App) -> bool {
    app.voicevox.prerender.job.is_some()
}

/// editor の終了時。起動途中の engine を残さないよう、job が返す engine まで受け取って落とす。
pub fn shut_down(app: &mut App) {
    crate::view::voicevox::singer::shut_down(app);
    if let Some(job) = app.voicevox.prerender.job.take()
        && let Ok(outcome) = job.join()
    {
        drop(outcome.engine);
    }
    app.voicevox.engine = None;
}

#[cfg(test)]
pub fn insert(app: &mut App, score: &Score, rate: u32, samples: Vec<f32>) {
    app.voicevox
        .prerender
        .sung
        .insert(cache::key(score, rate), Arc::new(samples));
}

/// Progress while the background job is still running.
pub fn progress(app: &App) -> Option<&'static str> {
    let state = &app.voicevox.prerender;
    let job = state.job.as_ref()?;
    if job.is_finished() {
        return None;
    }
    Some(if state.connecting.load(Ordering::Relaxed) {
        "Starting VOICEVOX engine…"
    } else {
        "Preparing VOICEVOX audio…"
    })
}

#[cfg(test)]
mod progress_tests;
