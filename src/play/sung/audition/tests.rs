use super::super::tests::{app_at, donguri, fake, faked, RATE, SINGER};
use super::*;
use crate::play::layer_render::{Effect, Instrument, Render};
use cat_smf_editor::score::note_view::note_on;
use cat_smf_editor::voicevox::score::LEAD_FRAMES;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Silent;
impl Instrument for Silent {
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], _: usize) -> bool {
        out.fill(0.0);
        true
    }
}
impl Effect for Silent {
    fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
        true
    }
}

/// 1 frame = 512 sample（48 kHz）。
const FRAME: usize = 512;

#[test]
fn changing_voice_does_not_reuse_the_previous_voices_one_note_audition() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(crate::view::edit_menu::lyric_tests::wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = app_at(donguri(), None);
    app.voicevox.prerender.target = Some((0, root.path().into()));
    app.voicevox.prerender.sing = Some(sing);
    let ctx = eframe::egui::Context::default();
    let first = cat_smf_editor::voicevox::Voice::default();
    let next = cat_smf_editor::voicevox::Voice {
        style_id: 3001,
        ..first.clone()
    };
    assert!(prerender::note(&mut app, first.clone(), (60, 45)).is_none());
    crate::view::edit_menu::lyric_tests::settle(&mut app, &ctx);
    assert!(prerender::note(&mut app, first, (60, 45)).is_some());
    assert!(prerender::note(&mut app, next.clone(), (60, 45)).is_none());
    crate::view::edit_menu::lyric_tests::settle(&mut app, &ctx);
    assert!(prerender::note(&mut app, next, (60, 45)).is_some());
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

/// The cursor on the `n`-th note-on of the singer, auditioned as a click does.
fn click_note(app: &mut App, n: usize) -> u64 {
    let loaded = app.loaded.as_ref().unwrap();
    let events = &loaded.music.tracks[SINGER].events;
    let index = (0..events.len())
        .filter(|&i| note_on(&events[i]))
        .nth(n)
        .unwrap();
    let row = if app.note_mode {
        loaded.display_rows[SINGER].event_row(index).unwrap()
    } else {
        index
    };
    let tick = events[index].tick;
    crate::cursor::moves::select(app, row);
    tick
}

/// The left channel of the first `frames` frames the audition plays, as the output stream
/// mixes a part that has velocities and a take.
fn heard(take: Arc<Take>, frames: usize) -> Vec<f32> {
    let mut render: Render<Silent, Silent> = Render::new(Vec::new(), None);
    render.cue(Some(take));
    let mut velocities = [0; 128];
    velocities[60] = 80;
    let mut out = vec![0.0; frames * 2];
    let mut changed = 1;
    for block in out.chunks_mut(1024 * 2) {
        assert!(render.process(&velocities, &[0; 128], changed, block, 2));
        changed = 0;
    }
    out.iter().step_by(2).copied().collect()
}

#[test]
fn the_cursor_on_note_i_sounds_from_the_head_of_note_i_s_piece() {
    let mut app = faked(None);
    let score = score::from_track(&donguri(), SINGER).unwrap();
    let voiced: Vec<usize> = (0..score.notes.len())
        .filter(|&j| score.notes[j].key.is_some())
        .collect();
    assert_eq!(voiced.len(), 8);
    for (n, &j) in voiced.iter().enumerate() {
        click_note(&mut app, n);
        let take = app.audition.sung.clone().expect("a sung piece");
        let head = score.starts[j] as usize * FRAME;
        // The piece runs to the next sung note, or to the end of the song.
        let end = voiced
            .get(n + 1)
            .map_or(score.total_frames(), |&k| score.starts[k]) as usize
            * FRAME;
        assert_eq!(take.cues, [Some(head..end)], "note {n}");
        let out = heard(take, 2000);
        for (k, value) in out.iter().enumerate() {
            assert_eq!(*value, (head + k) as f32, "note {n}, frame {k}");
        }
    }
    assert!(!prerender::running(&app));
}

#[test]
fn before_the_song_is_sung_the_instrument_sounds_and_one_job_sings_it() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(crate::view::edit_menu::lyric_tests::wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = app_at(donguri(), None);
    app.voicevox.prerender.target = Some((50126, root.path().into()));
    app.voicevox.prerender.sing = Some(sing);
    click_note(&mut app, 0);
    assert_eq!(app.audition.auditioned, [67]);
    assert!(app.audition.sung.is_none(), "the instrument sounds first");
    assert!(prerender::running(&app));
    // More moves while it sings do not ask again.
    click_note(&mut app, 1);
    assert!(app.audition.sung.is_none());
    let ctx = eframe::egui::Context::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while prerender::running(&app) {
        assert!(std::time::Instant::now() < deadline);
        let _ = ctx.run(Default::default(), |ctx| prerender::poll(&mut app, ctx));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(SUNG.load(Ordering::SeqCst), 1);
    click_note(&mut app, 2);
    assert!(app.audition.sung.is_some(), "the voice sounds once sung");
    assert!(!prerender::running(&app));
    assert_eq!(SUNG.load(Ordering::SeqCst), 1);
}

#[test]
fn a_track_that_does_not_sing_keeps_its_instrument_and_starts_no_job() {
    let mut music = donguri();
    music.tracks[SINGER].events.remove(0);
    let mut app = app_at(music, None);
    let score = score::from_track(&donguri(), SINGER).unwrap();
    prerender::insert(
        &mut app,
        &score,
        RATE,
        fake(score.total_frames() as usize * FRAME),
    );
    click_note(&mut app, 0);
    assert_eq!(app.audition.auditioned, [67]);
    assert!(app.audition.sung.is_none());
    assert!(!prerender::running(&app));
}

#[test]
fn one_note_sounds_from_its_note_head_to_the_end_of_the_tail() {
    let score = one_note(64, 47);
    let total = score.total_frames() as usize * FRAME;
    let made = piece(&score, Arc::new(fake(total)), RATE, 0.0).unwrap();
    let head = LEAD_FRAMES as usize * FRAME;
    let out = heard(Arc::new(made), total);
    for (k, value) in out.iter().enumerate() {
        let expected = if head + k < total {
            (head + k) as f32
        } else {
            0.0
        };
        assert_eq!(*value, expected, "frame {k}");
    }
}
