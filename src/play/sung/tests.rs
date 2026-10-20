//! engine を使わず、合成の WAV を「sample の値 = その frame 番号」の fake に差し替えて確かめる。
use super::*;
use crate::app::App;
use crate::play::audition::offline::without_plugins;
use crate::play::layer_render::{Effect, Instrument, Render};
use crate::play::playback::Mode;
use cat_smf_editor::instrument_layers::Layer;
use cat_smf_editor::model::{Event, Music, Track};
use cat_smf_editor::voicevox::score::from_track;
use std::sync::atomic::AtomicBool;

pub(crate) const RATE: u32 = 48000;
/// 125 BPM・480 tick/四分音符で、四分音符 = 45 frame = 0.48 s。note の境界がすべて frame に乗る。
const QUARTER: u64 = 480;
pub(crate) const SINGER: usize = 1;
/// 1 frame = 512 sample（48 kHz）。
const FRAME: usize = 512;
const LEAD: usize = LEAD_FRAMES as usize * FRAME;
/// 2小節目の頭（4/4）。
pub(super) const BAR_2: u64 = 4 * QUARTER;
/// `fade_out` が触る末尾は比べない。
pub(crate) const FADE: usize = RATE as usize / 100;

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// 「どんぐりころころ」（作詞 青木存義・作曲 梁田貞）の冒頭8音。四分休符から始まり、
/// 3音目が2小節目へまたがり、5音目の後に短い休符がある。
pub(crate) fn donguri() -> Music {
    let conductor = vec![
        event(0, 0xff, 0x51, &[0x07, 0x53, 0x00]),
        event(0, 0xff, 0x58, &[4, 2, 24, 8]),
        event(5280, 0xff, 0x2f, &[]),
    ];
    let mut events = vec![event(0, 0xff, 0x03, "ずんだもん".as_bytes())];
    let notes: [(u64, u64, u8, &str); 8] = [
        (480, 480, 67, "ど"),
        (960, 480, 64, "ん"),
        (1440, 960, 64, "ぐ"),
        (2400, 480, 65, "り"),
        (2880, 384, 64, "こ"),
        (3360, 480, 62, "ろ"),
        (3840, 480, 60, "こ"),
        (4320, 960, 60, "ろ"),
    ];
    for (tick, length, key, lyric) in notes {
        events.push(event(tick, 0xff, 0x05, lyric.as_bytes()));
        events.push(event(tick, 0x90, 0, &[key, 100]));
        events.push(event(tick + length, 0x80, 0, &[key, 0]));
    }
    events.sort_by_key(|e| e.tick);
    events.push(event(5280, 0xff, 0x2f, &[]));
    let mut singer = Track { events };
    cat_smf_editor::voicevox::make_singer(&mut singer).unwrap();
    Music {
        format: 1,
        division: QUARTER as u16,
        tracks: vec![Track { events: conductor }, singer],
    }
}

pub(crate) fn app_at(music: Music, cursor: Option<u64>) -> App {
    let mut app = crate::view::tests::app(music);
    app.track = SINGER;
    app.track_view.cursor_tick = cursor;
    app
}

/// 実経路で再生範囲を作り、歌う track だけをプラグイン無しの mixer で描く。
pub(crate) fn render(app: &mut App, mode: Mode) -> Vec<f32> {
    let prepared = crate::play::transport::prepare(app, mode).unwrap();
    let takes = prerender::takes(app, &prepared).expect("sung already");
    let (_, take) = takes.into_iter().find(|(t, _)| *t == SINGER).unwrap();
    let sequence = prepared
        .parts
        .into_iter()
        .find(|(t, _, _)| *t == SINGER)
        .unwrap()
        .2;
    without_plugins(RATE, vec![(sequence, Some(take))], vec![1.0])
        .run(&AtomicBool::new(false))
        .unwrap()
        .unwrap()
}

/// 左は frame 番号、右はその符号反転。
pub(super) fn fake(frames: usize) -> Vec<f32> {
    (0..frames).flat_map(|n| [n as f32, -(n as f32)]).collect()
}

pub(super) fn faked(cursor: Option<u64>) -> App {
    let music = donguri();
    let score = from_track(&music, SINGER).unwrap();
    let mut app = app_at(music, cursor);
    prerender::insert(
        &mut app,
        &score,
        RATE,
        fake(score.total_frames() as usize * FRAME),
    );
    app
}

fn left(samples: &[f32]) -> Vec<f32> {
    samples.iter().step_by(2).copied().collect()
}

#[test]
fn from_the_song_head_the_wav_is_read_straight_through_rests_and_note_heads() {
    let mut app = faked(None);
    let out = render(&mut app, Mode::AllTracks);
    // 5280 ticks = 11 quarters = 5.28 s.
    assert_eq!(out.len(), 11 * 45 * FRAME * 2);
    let head = left(&out);
    let body = head.len() - FADE;
    for (k, value) in head[..body].iter().enumerate() {
        assert_eq!(*value, (LEAD + k) as f32, "frame {k}");
    }
    assert!(out.chunks(2).all(|f| f[0] == -f[1]));
}

#[test]
fn from_bar_2_the_held_note_restarts_and_the_rest_matches_the_song_head() {
    let mut app = faked(Some(BAR_2 + 10));
    let out = left(&render(&mut app, Mode::AllTracksFromMeasure));
    let bar = 4 * 45 * FRAME;
    let quarter = 45 * FRAME;
    // ぐ (tick 1440 = frame 135 + lead) sounds from its head until り at the next quarter.
    for (k, value) in out[..quarter].iter().enumerate() {
        assert_eq!(*value, (LEAD + 135 * FRAME + k) as f32, "frame {k}");
    }
    for (k, value) in out[quarter..out.len() - FADE].iter().enumerate() {
        assert_eq!(*value, (LEAD + bar + quarter + k) as f32, "frame {k}");
    }
}

#[test]
fn a_note_cut_by_the_range_stops_and_a_leading_rest_sounds_from_the_range_start() {
    let mut app = faked(Some(10));
    let out = left(&render(&mut app, Mode::Measure));
    let quarter = 45 * FRAME;
    assert_eq!(out.len(), 4 * quarter);
    // The opening rest plays in time, so ど's consonant leads into its note.
    for (k, value) in out[..3 * quarter].iter().enumerate() {
        assert_eq!(*value, (LEAD + k) as f32, "frame {k}");
    }
    // ぐ spans the bar line; the measure ends half way through it.
    for (k, value) in out[3 * quarter..].iter().enumerate() {
        if k < quarter - FADE {
            assert_eq!(*value, (LEAD + 3 * quarter + k) as f32, "frame {k}");
        }
    }
    // The render ends with the range, so the cut shows in the cue: ぐ stops after a quarter
    // instead of running on into り's consonant.
    let prepared = crate::play::transport::prepare(&app, Mode::Measure).unwrap();
    let take = prerender::takes(&mut app, &prepared).unwrap().remove(0).1;
    let cut = take.cues.last().unwrap().clone().unwrap();
    assert_eq!(
        (cut.start, cut.end),
        (LEAD + 3 * quarter, LEAD + 4 * quarter)
    );

    let mut app = faked(Some(BAR_2 + 10));
    let out = left(&render(&mut app, Mode::Measure));
    // Measure mode does not resume ぐ; り comes after a quarter of silence.
    assert!(out[..quarter].iter().all(|v| *v == 0.0));
    assert_eq!(out[quarter], (LEAD + 5 * quarter) as f32);
}

#[test]
fn the_track_gain_scales_the_voice_and_a_failed_take_is_silent() {
    let mut app = faked(None);
    let prepared = crate::play::transport::prepare(&app, Mode::AllTracks).unwrap();
    let take = prerender::takes(&mut app, &prepared).unwrap().remove(0).1;
    let sequence = prepared
        .parts
        .iter()
        .find(|p| p.0 == SINGER)
        .unwrap()
        .2
        .clone();
    let out = without_plugins(
        RATE,
        vec![
            (sequence.clone(), Some(take)),
            (sequence, Some(Arc::new(Take::silent()))),
        ],
        vec![0.25, 1.0],
    )
    .run(&AtomicBool::new(false))
    .unwrap()
    .unwrap();
    assert_eq!(out[1000 * 2], (LEAD + 1000) as f32 * 0.25);
}

struct Synth {
    heard: std::rc::Rc<std::cell::Cell<u32>>,
}
impl Instrument for Synth {
    fn render(&mut self, v: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], _: usize) -> bool {
        let on = v.iter().any(|v| *v > 0);
        if on {
            self.heard.set(self.heard.get() + 1);
        }
        out.fill(if on { 100.0 } else { 0.0 });
        true
    }
}
struct NoEffect;
impl Effect for NoEffect {
    fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
        true
    }
}

fn layered(layers: Vec<Layer>) -> (Render<Synth, NoEffect>, std::rc::Rc<std::cell::Cell<u32>>) {
    let heard = std::rc::Rc::new(std::cell::Cell::new(0));
    let render = Render::new(
        layers
            .into_iter()
            .map(|layer| {
                (
                    Synth {
                        heard: heard.clone(),
                    },
                    layer,
                )
            })
            .collect(),
        None,
    );
    (render, heard)
}

/// 1 note を歌う take。片の値は 0.5。
fn one_note() -> Arc<Take> {
    Arc::new(Take {
        samples: Arc::new(vec![0.5; 2 * 8]),
        lead: None,
        cues: vec![Some(0..8)],
    })
}

#[test]
fn the_voice_takes_the_first_layer_place_and_silences_the_instruments() {
    let mut velocities = [0; 128];
    velocities[60] = 100;
    let durations = [0; 128];
    let mut out = [0.0; 4];

    let (mut render, heard) = layered(vec![Layer::default(), Layer::default()]);
    render.cue(Some(one_note()));
    assert!(render.process(&velocities, &durations, 1, &mut out, 2));
    assert_eq!(out, [0.5; 4]);
    assert_eq!(heard.get(), 0);

    let (mut render, _) = layered(vec![Layer {
        volume_db: -6.0206,
        ..Default::default()
    }]);
    render.cue(Some(one_note()));
    assert!(render.process(&velocities, &durations, 1, &mut out, 2));
    assert!((out[0] - 0.25).abs() < 1e-4);

    let (mut render, _) = layered(vec![Layer {
        mute: true,
        ..Default::default()
    }]);
    render.cue(Some(one_note()));
    assert!(render.process(&velocities, &durations, 1, &mut out, 2));
    assert_eq!(out, [0.0; 4]);
    // The voice kept its place while muted.
    assert!(render.process(&[0; 128], &durations, 0, &mut out, 2));

    let (mut render, heard) = layered(vec![Layer::default()]);
    render.cue(None);
    assert!(render.process(&velocities, &durations, 1, &mut out, 2));
    assert_eq!(out, [100.0; 4]);
    assert_eq!(heard.get(), 1);
}

#[test]
fn takes_are_empty_without_voice_metadata() {
    let mut music = donguri();
    cat_smf_editor::voicevox::Voice::write(&mut music.tracks[SINGER], None).unwrap();
    let mut app = app_at(music, None);
    let prepared = crate::play::transport::prepare(&app, Mode::AllTracks).unwrap();
    assert!(prerender::takes(&mut app, &prepared).unwrap().is_empty());
    assert!(!prerender::running(&app));
}

#[test]
fn without_an_engine_the_playback_starts_with_the_singer_silent_and_asks_for_run_exe() {
    const UNUSED: u16 = 50127;
    let client =
        cat_smf_editor::voicevox::client::Client::new(&format!("http://127.0.0.1:{UNUSED}"));
    assert!(client.version().is_err(), "port {UNUSED} is in use");
    let root = tempfile::tempdir().unwrap();
    let mut app = app_at(donguri(), None);
    app.voicevox.prerender.target = Some((UNUSED, root.path().into()));
    let wait = |app: &mut App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.playback.pending.is_some() {
            assert!(std::time::Instant::now() < deadline);
            crate::play::transport::advance(app);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    for attempt in 0..2 {
        crate::play::playback::start(&mut app, Mode::AllTracks);
        crate::play::transport::advance(&mut app);
        assert!(crate::play::transport::preparing(&app), "attempt {attempt}");
        assert!(prerender::running(&app), "attempt {attempt}");
        wait(&mut app);
        assert!(app.playback.active.is_some());
        assert!(app.voicevox.wizard);
        assert_eq!(app.voicevox.error, None);
        crate::view::voicevox::close(&mut app);
        assert_eq!(
            app.voicevox.error.as_deref(),
            Some("VOICEVOX engine is not set")
        );
        app.voicevox.error = None;
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
