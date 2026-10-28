//! A singing container's pattern is sung in the pitches and at the times it sounds,
//! whether auditioned, played in a Scene, played in the Song, or played while edited alone.
use super::prerender;
use crate::app::App;
use crate::play::playback::Mode;
use cat_smf_editor::{
    edit,
    model::{Event, Music, Track},
    score::{
        pattern::operations,
        scene::arrangement::{self as grid, PatternOrScene},
    },
    voicevox::{
        score::{FRAMES_PER_SECOND, LEAD_FRAMES},
        Voice,
    },
};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// The Song runs at 150 BPM; Scenes and auditions at the preview tempo (120 BPM by default).
/// The container sings a pattern of "ど" (60) then, a quarter rest later, "れ" (64). A Scene puts
/// the pattern at its measure 2, and the Song puts that Scene at its measure 2.
/// Returns the app, the pattern and the Scene.
fn fixture() -> (App, u64, u64) {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                event(0, 0xff, 0x51, &[0x06, 0x1a, 0x80]),
                event(15360, 0xff, 0x2f, &[]),
            ],
        }],
    };
    let source = operations::create_source(&mut m).unwrap();
    let id = operations::create(&mut m, source, None).unwrap();
    let start = cat_smf_editor::score::pattern::Catalog::read(&m)
        .unwrap()
        .patterns[&id]
        .range()
        .unwrap()
        .0;
    for (offset, pitch, lyric) in [(0, 60, "ど"), (960, 64, "れ")] {
        let tick = start + offset;
        for e in [
            event(tick, 0xff, edit::lyric::LYRIC, lyric.as_bytes()),
            event(tick, 0x90, 0, &[pitch, 100]),
            event(tick + 480, 0x80, 0, &[pitch, 0]),
        ] {
            edit::insert(&mut m.tracks[source], e);
        }
    }
    Voice::write(&mut m.tracks[source], Some(&Voice::default())).unwrap();
    let scene = grid::create_scene(&mut m, "Verse", 2, 1).unwrap();
    let lane = cat_smf_editor::score::scene::read(&m).unwrap().scenes[0].lanes[0];
    grid::place(
        &mut m,
        Some(scene),
        lane,
        2,
        Some(PatternOrScene::Pattern(id)),
    )
    .unwrap();
    let song = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, song, 2, Some(PatternOrScene::Scene(scene))).unwrap();
    for t in &mut m.tracks {
        edit::refresh_deltas(t).unwrap();
    }
    (crate::view::tests::app(m), id, scene)
}

type Notes = Vec<(usize, u8, u64)>;

/// `(track, pitch, start frame)` of the notes playback sounds, and of the notes it would sing.
fn compare(a: &App, mode: Mode) -> (Notes, Notes) {
    let prepared = crate::play::transport::prepare(a, mode).expect("prepared");
    let frame = |seconds: f64| LEAD_FRAMES + (seconds * FRAMES_PER_SECOND).round() as u64;
    let mut played: Notes = prepared
        .parts
        .iter()
        .flat_map(|(track, _, sequence)| {
            sequence
                .notes
                .iter()
                .map(move |n| (*track, n.pitch, frame(n.start)))
        })
        .collect();
    let mut sung: Notes = prerender::scores(a, &prepared)
        .into_iter()
        .flat_map(|(track, score)| {
            score
                .notes
                .iter()
                .zip(&score.starts)
                .filter_map(|(n, &start)| Some((track, n.key?, start)))
                .collect::<Vec<_>>()
        })
        .collect();
    played.sort();
    sung.sort();
    (played, sung)
}

fn assert_sings_what_sounds(a: &App, mode: Mode) {
    let (played, sung) = compare(a, mode);
    assert!(!played.is_empty(), "{mode:?} sounds nothing");
    assert_eq!(sung, played, "{mode:?}: sung vs played");
}

#[test]
fn pattern_audition_sings_what_it_sounds() {
    let (a, id, _) = fixture();
    assert_sings_what_sounds(&a, Mode::Pattern(id));
}

#[test]
fn scene_playback_sings_what_it_sounds() {
    let (a, _, scene) = fixture();
    assert_sings_what_sounds(&a, Mode::Arrangement(Some(scene)));
}

#[test]
fn song_playback_sings_what_it_sounds() {
    let (a, _, _) = fixture();
    assert_sings_what_sounds(&a, Mode::Arrangement(None));
}

#[test]
fn standalone_edit_sings_what_it_sounds() {
    let (mut a, id, _) = fixture();
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    assert_sings_what_sounds(&a, Mode::Track);
}

#[test]
fn song_sings_the_lyrics_it_sounds() {
    let (a, _, _) = fixture();
    let prepared = crate::play::transport::prepare(&a, Mode::Arrangement(None)).unwrap();
    let scores = prerender::scores(&a, &prepared);
    let lyrics: Vec<_> = scores[0]
        .1
        .notes
        .iter()
        .filter(|n| n.key.is_some())
        .map(|n| n.lyric.as_str())
        .collect();
    assert_eq!(lyrics, ["ど", "れ"]);
}

/// Started part way, playback still sings the note at the time it sounds in the whole Song.
#[test]
fn song_started_part_way_sings_what_it_sounds() {
    let (a, _, _) = fixture();
    let whole = crate::play::transport::prepare(&a, Mode::Arrangement(None)).unwrap();
    let seconds = whole.parts[0].2.notes[1].start - 0.1;
    let skipped = crate::play::selection_playback::skip(&a, whole.active, seconds).unwrap();
    let frame = |seconds: f64| LEAD_FRAMES + (seconds * FRAMES_PER_SECOND).round() as u64;
    let played: Notes = skipped.parts[0]
        .2
        .notes
        .iter()
        .map(|n| {
            (
                skipped.parts[0].0,
                n.pitch,
                frame(skipped.active.start_seconds + n.start),
            )
        })
        .collect();
    let (_, sung) = compare(&a, Mode::Arrangement(None));
    assert_eq!(played, sung[1..]);
    assert_eq!(
        prerender::scores(&a, &skipped)
            .into_iter()
            .map(|(_, s)| s.starts)
            .collect::<Vec<_>>(),
        prerender::scores(
            &a,
            &crate::play::transport::prepare(&a, Mode::Arrangement(None)).unwrap()
        )
        .into_iter()
        .map(|(_, s)| s.starts)
        .collect::<Vec<_>>()
    );
}

/// With the sung WAV ready, every note playback sounds starts a sung piece, from the head and
/// part way through.
#[test]
fn every_sounded_note_cues_a_sung_piece() {
    let (mut a, id, scene) = fixture();
    let rate = a.audition.rate();
    for mode in [
        Mode::Pattern(id),
        Mode::Arrangement(Some(scene)),
        Mode::Arrangement(None),
    ] {
        let whole = crate::play::transport::prepare(&a, mode).unwrap();
        for (_, score) in prerender::scores(&a, &whole) {
            let frames =
                (score.total_frames() as f64 * f64::from(rate) / FRAMES_PER_SECOND).ceil() as usize;
            prerender::insert(&mut a, &score, rate, vec![0.0; frames * 2]);
        }
        let seconds = whole.parts[0].2.notes[1].start - 0.1;
        let skipped = crate::play::selection_playback::skip(&a, whole.active, seconds).unwrap();
        let whole = crate::play::transport::prepare(&a, mode).unwrap();
        for (prepared, expected) in [(&whole, 2), (&skipped, 1)] {
            let takes = prerender::takes(&mut a, prepared).expect("sung already");
            let voiced: Vec<_> = takes.iter().map(|(t, take)| (*t, take.voiced())).collect();
            assert_eq!(voiced, [(1, expected)], "{mode:?}");
        }
    }
}
