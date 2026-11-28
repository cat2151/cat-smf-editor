//! `--render-wav`: what all-track or track playback sounds, through the offline mixer,
//! without a window or audio device. The song file and the caches it reads stay unchanged.
use crate::{
    app::App,
    play::{
        audition::offline::{Instruments, CHANNELS},
        playback::Mode,
        sung::audition::stored,
        transport::Prepared,
    },
    store::cli::Render,
};
use cat_smf_editor::message;
use std::{sync::atomic::AtomicBool, time::Duration};

const RATE: u32 = 48_000;

/// The native library must already be loaded.
pub fn run(render: Render) -> Result<(), String> {
    let mut app = load(render.player, render.mid)?;
    let prepared = prepare(&mut app, render.track)?;
    let out = samples(&app, prepared)?;
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    eprintln!(
        "{}: {:.3}s peak {peak:.3}",
        render.wav.display(),
        (out.len() / CHANNELS) as f64 / f64::from(RATE)
    );
    crate::play::wav::write_pcm16(&render.wav, RATE, &out)
}

fn load(player: Option<std::path::PathBuf>, mid: std::path::PathBuf) -> Result<App, String> {
    let mut app = App::empty();
    app.player_selection.executable = player;
    app.load_mid_unchecked(mid);
    match app.load_error.take() {
        Some(error) => Err(error),
        None => Ok(app),
    }
}

/// `track` is 0-based; None prepares all-track playback.
fn prepare(app: &mut App, track: Option<usize>) -> Result<Prepared, String> {
    let music = &app.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    let tracks = music.tracks.len();
    let song = !cat_smf_editor::score::scene::read(music)?.song.is_empty();
    // A song solos a container as its overview row does; otherwise track playback.
    let prepared = match track {
        Some(track) if track >= tracks => {
            return Err(format!("--track {}: {tracks} tracks", track + 1));
        }
        Some(track) if song => crate::play::arrangement::remaining_tracks(
            app,
            None,
            0.0,
            f64::INFINITY,
            Some(&[track]),
        ),
        Some(track) => {
            app.track = track;
            crate::play::transport::prepare(app, Mode::Track)
        }
        None => crate::play::transport::prepare(app, Mode::AllTracks),
    }
    .ok_or(message::NO_NOTES)?;
    if let Some(error) = &prepared.active.automation.error {
        return Err(message::automation(error));
    }
    Ok(prepared)
}

/// Interleaved stereo at `RATE` from the playback start.
fn samples(app: &App, prepared: Prepared) -> Result<Vec<f32>, String> {
    let music = &app.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    let tracks = music.tracks.len();
    let takes = crate::play::sung::prerender::directory(app).map_or_else(Vec::new, |d| {
        stored(
            music,
            &prepared.parts,
            prepared.active.start_seconds,
            RATE,
            &d,
        )
    });
    let gains: Vec<f32> = (0..tracks).map(|t| app.audition.gain(t)).collect();
    let instruments = Instruments::open_automated(
        RATE,
        &prepared.parts,
        app.audition.master(),
        &prepared.active.automation,
    )?;
    // SAFETY: the render thread is joined before the instruments drop.
    let job = unsafe {
        instruments.job_at(
            RATE,
            prepared.parts,
            &gains,
            !app.audition.auto_gain_enabled(),
            takes,
            prepared.active.start_seconds,
        )
    };
    let cancel = AtomicBool::new(false);
    let rendered = std::thread::scope(|scope| {
        let thread = scope.spawn(|| job.run(&cancel));
        // Plugins open on this thread, so it keeps servicing them while the job runs.
        while !thread.is_finished() {
            instruments.pump();
            std::thread::sleep(Duration::from_millis(2));
        }
        thread.join()
    });
    rendered.ok().flatten().ok_or("書き出しが止まりました")?
}

#[cfg(test)]
#[path = "render_wav_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "render_wav_first_play_tests.rs"]
mod first_play_tests;
