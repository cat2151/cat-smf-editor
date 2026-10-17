use super::*;
use cat_smf_editor::{
    host::PluginKey,
    model::{Event, Music, Track},
    resident_ipc::{Listener, Receiver, ToEditor},
    score::instrument::{self as chain_settings, Patch as Settings},
};

fn event(tick: u64, status: u8, meta_type: u8, data: Vec<u8>) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data,
    }
}

fn notes(channel: u8, key: u8) -> Track {
    Track {
        events: vec![
            event(0, 0x90 | channel, 0, vec![key, 100]),
            event(480, 0x80 | channel, 0, vec![key, 0]),
            event(960, 0xff, 0x2f, vec![]),
        ],
    }
}

fn music() -> Music {
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    event(0, 0xff, 0x51, vec![7, 161, 32]),
                    event(480, 0xff, 0x51, vec![3, 208, 144]),
                    event(960, 0xff, 0x2f, vec![]),
                ],
            },
            notes(0, 60),
            notes(1, 64),
        ],
    }
}

fn slot(id: &str, state: Vec<u8>) -> PluginState {
    crate::test_support::referenced(PluginState {
        reference: None,
        plugin: PluginKey {
            format: "CLAP".into(),
            id: id.into(),
            name: id.into(),
            vendor: String::new(),
            bundle_path: format!("X:/plugins/{id}.clap"),
        },
        state,
        preset_name: Some(format!("{id} preset")),
        bypass: false,
    })
}

type Signature = Vec<(u64, u8, u8, Vec<u8>)>;

fn signature<'a>(events: impl Iterator<Item = &'a Event>) -> Signature {
    events
        .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
        .collect()
}

/// Events other than the chain and master-effect metadata.
fn content(track: &Signature) -> Signature {
    track
        .iter()
        .filter(|(_, status, meta_type, data)| {
            let event = event(0, *status, *meta_type, data.clone());
            !(*status == 0xff && *meta_type == 0x7f && data.starts_with(chain_settings::PREFIX))
                && !cat_smf_editor::master_effects::owned(&event)
                && !event
                    .data
                    .starts_with(cat_smf_editor::master_references::PREFIX)
        })
        .cloned()
        .collect()
}

fn tracks(app: &App) -> Vec<Signature> {
    let music = &app.loaded.as_ref().unwrap().music;
    music
        .tracks
        .iter()
        .map(|t| signature(t.events.iter()))
        .collect()
}

fn chain(app: &App, track: usize) -> Settings {
    {
        let patch = &app.loaded.as_ref().unwrap().patches[track];
        assert_eq!(
            cat_smf_editor::instrument_layers::read(
                &app.loaded.as_ref().unwrap().music.tracks[track],
                &mut crate::player::patch_roots::Assumed
            )
            .unwrap(),
            patch.saved().unwrap()
        );
        patch.clone()
    }
}

#[test]
fn a_patch_changes_only_the_chain_metadata_of_its_track() {
    let mut app = crate::view::tests::app(music());
    let before = tracks(&app);
    let synth = slot("synth", vec![1, 2, 3]);
    apply(&mut app, Open::Patch { track: 2 }, synth.clone()).unwrap();
    let after = tracks(&app);
    assert_eq!(after[0], before[0], "tempo track untouched");
    assert_eq!(after[1], before[1], "other track untouched");
    assert_ne!(after[2], before[2]);
    assert_eq!(content(&after[2]), content(&before[2]), "notes untouched");
    let settings = chain(&app, 2);
    assert_eq!(settings.layers[0].instrument, Some(synth));
    assert_eq!(app.loaded.as_ref().unwrap().patches[2], settings);
    assert_eq!(app.loaded.as_ref().unwrap().patches[1], Settings::default());
}

#[test]
fn effects_replace_or_add_a_slot_and_keep_one_instance_per_plugin() {
    let mut app = crate::view::tests::app(music());
    let before = tracks(&app);
    let effect = |slot_at| Open::Effect {
        track: 1,
        slot: slot_at,
    };
    assert!(apply(&mut app, effect(None), slot("echo", vec![])).is_err());
    assert_eq!(tracks(&app), before, "a refused effect changes nothing");

    apply(&mut app, Open::Patch { track: 1 }, slot("synth", vec![1])).unwrap();
    apply(&mut app, effect(None), slot("echo", vec![1])).unwrap();
    apply(&mut app, effect(None), slot("hall", vec![2])).unwrap();
    apply(&mut app, effect(Some(1)), slot("echo", vec![3])).unwrap();
    let ids = |app: &App| -> Vec<String> {
        let patches = &app.loaded.as_ref().unwrap().patches[1];
        patches
            .effects
            .iter()
            .map(|e| e.plugin.id.clone())
            .collect()
    };
    assert_eq!(ids(&app), ["hall", "echo"]);

    let kept = tracks(&app);
    assert!(apply(&mut app, effect(Some(0)), slot("echo", vec![4])).is_err());
    assert!(apply(&mut app, effect(Some(5)), slot("plate", vec![4])).is_err());
    assert_eq!(tracks(&app), kept);
    assert_eq!(kept[0], before[0]);
    assert_eq!(kept[2], before[2]);
    assert_eq!(content(&kept[1]), content(&before[1]));
}

#[test]
fn a_master_effect_changes_only_the_master_metadata() {
    let mut app = crate::view::tests::app(music());
    let before = tracks(&app);
    let hall = slot("hall", vec![5; 4]);
    apply(&mut app, Open::MasterEffect { slot: None }, hall.clone()).unwrap();
    assert_eq!(app.audition.master().effects, [hall]);
    let after = tracks(&app);
    assert_eq!(after[1..], before[1..]);
    assert_eq!(content(&after[0]), content(&before[0]), "tempo untouched");
    assert_eq!(
        cat_smf_editor::master_references::read(
            &app.loaded.as_ref().unwrap().music,
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        crate::test_support::master_saved(app.audition.master())
    );
}

#[test]
fn a_selection_from_the_pipe_is_applied_and_answered() {
    let name = format!("editor-resident-test-{}", std::process::id());
    let listener = Listener::create(&name).unwrap();
    let player = {
        let name = name.clone();
        std::thread::spawn(move || resident_ipc::connect(&name).unwrap())
    };
    let (mut editor_out, mut editor_in) = listener.accept().unwrap();
    let (mut player_out, mut player_in) = player.join().unwrap();

    let state: Vec<u8> = (0..1_300_001u32).map(|i| (i % 251) as u8).collect();
    let shared = player_out.share(&state).unwrap();
    let synth = slot("synth", Vec::new());
    player_out
        .send(&ToEditor::Selected(Selected {
            reference: synth.reference.clone().map(Box::new),
            target: Open::Patch { track: 1 },
            plugin: synth.plugin.clone(),
            preset_name: synth.preset_name.clone(),
            state: shared.reference.clone(),
        }))
        .unwrap();

    let mut app = crate::view::tests::app(music());
    let target = |open| Target {
        open,
        bulk: false,
        original: app.loaded.as_ref().unwrap().original.clone(),
    };
    let patch_1 = target(Open::Patch { track: 1 });
    let patch_2 = target(Open::Patch { track: 2 });
    let ToEditor::Selected(selected) = editor_in.receive().unwrap().unwrap() else {
        panic!("expected selection")
    };
    answer(&mut app, selected, Some(&patch_1), &mut editor_out).unwrap();
    assert_eq!(
        player_in.receive().unwrap(),
        Some(ToPlayer::Applied(Ok(())))
    );
    drop(shared);
    let instrument = app.loaded.as_ref().unwrap().patches[1].layers[0]
        .instrument
        .clone();
    assert_eq!(instrument.unwrap().state, state);

    // A state the player no longer shares is reported, not applied.
    let mut gone = player_out.share(&[1]).unwrap().reference;
    gone.name.push_str("-missing");
    player_out
        .send(&ToEditor::Selected(Selected {
            reference: synth.reference.clone().map(Box::new),
            target: Open::Patch { track: 2 },
            plugin: synth.plugin,
            preset_name: None,
            state: gone,
        }))
        .unwrap();
    let ToEditor::Selected(selected) = editor_in.receive().unwrap().unwrap() else {
        panic!("expected selection")
    };
    answer(&mut app, selected, Some(&patch_2), &mut editor_out).unwrap();
    assert!(matches!(
        player_in.receive().unwrap(),
        Some(ToPlayer::Applied(Err(_)))
    ));
    assert_eq!(app.loaded.as_ref().unwrap().patches[2], Settings::default());
}

/// A connected pair: the editor's ends, then the player's.
fn pipes() -> ((Sender, Receiver), (Sender, Receiver)) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("editor-resident-pair-{}-{next}", std::process::id());
    let listener = Listener::create(&name).unwrap();
    let player = std::thread::spawn(move || resident_ipc::connect(&name).unwrap());
    let editor = listener.accept().unwrap();
    (editor, player.join().unwrap())
}

/// Sends `slot` for `open` from the player side and answers it in the editor.
fn select(
    app: &mut App,
    open: Open,
    target: Option<&Target>,
    slot: PluginState,
) -> Result<(), String> {
    let ((mut editor_out, mut editor_in), (mut player_out, mut player_in)) = pipes();
    let shared = player_out.share(&slot.state).unwrap();
    player_out
        .send(&ToEditor::Selected(Selected {
            reference: slot.reference.clone().map(Box::new),
            target: open,
            plugin: slot.plugin,
            preset_name: slot.preset_name,
            state: shared.reference.clone(),
        }))
        .unwrap();
    let ToEditor::Selected(selected) = editor_in.receive().unwrap().unwrap() else {
        panic!("expected selection")
    };
    answer(app, selected, target, &mut editor_out).unwrap();
    let Some(ToPlayer::Applied(result)) = player_in.receive().unwrap() else {
        panic!("no answer");
    };
    result
}

#[test]
fn a_selection_for_another_browser_or_song_is_refused() {
    let mut app = crate::view::tests::app(music());
    let before = tracks(&app);
    let original = app.loaded.as_ref().unwrap().original.clone();
    let patch = Open::Patch { track: 1 };
    let shown = |open, original| Target {
        open,
        bulk: false,
        original,
    };
    let effect = shown(
        Open::Effect {
            track: 1,
            slot: None,
        },
        original.clone(),
    );
    let other_song = shown(patch, "another.mid".into());
    for target in [None, Some(&effect), Some(&other_song)] {
        let result = select(&mut app, patch, target, slot("synth", vec![1]));
        assert!(result.unwrap_err().contains("target changed"));
    }
    assert_eq!(tracks(&app), before);
    let current = shown(patch, original);
    select(&mut app, patch, Some(&current), slot("synth", vec![1])).unwrap();
    assert_ne!(tracks(&app), before);
}

#[test]
fn a_bulk_patch_goes_to_every_track_with_notes() {
    let mut app = crate::view::tests::app(music());
    let before = tracks(&app);
    let bulk = Target {
        open: Open::Patch { track: 1 },
        bulk: true,
        original: app.loaded.as_ref().unwrap().original.clone(),
    };
    let synth = slot("synth", vec![9]);
    select(&mut app, bulk.open, Some(&bulk), synth.clone()).unwrap();
    let after = tracks(&app);
    assert_eq!(after[0], before[0], "conductor untouched");
    for track in [1, 2] {
        assert_eq!(
            chain(&app, track).layers[0].instrument.as_ref(),
            Some(&synth)
        );
        assert_eq!(content(&after[track]), content(&before[track]));
    }
}

#[test]
fn a_selection_while_playing_continues_from_the_same_position_with_it() {
    for open in [Open::Patch { track: 1 }, Open::MasterEffect { slot: None }] {
        let mut app = crate::view::tests::app(music());
        crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
        crate::play::transport::advance(&mut app);
        app.audition.set_test_seconds(0.25);
        let playing = app.playback.active.as_ref().unwrap().sources.len();
        let target = Target {
            open,
            bulk: false,
            original: app.loaded.as_ref().unwrap().original.clone(),
        };
        let chosen = slot("chosen", vec![7]);
        select(&mut app, open, Some(&target), chosen.clone()).unwrap();
        let prepared = app.playback.pending.as_ref().expect("playback restarts");
        assert_eq!(prepared.active.start_seconds, 0.25, "{open:?}");
        assert_eq!(prepared.parts.len(), playing);
        if let Open::Patch { track } = open {
            let part = prepared.parts.iter().find(|p| p.0 == track).unwrap();
            assert_eq!(part.1.layers[0].instrument.as_ref(), Some(&chosen));
        } else {
            assert_eq!(app.audition.master().effects, [chosen]);
        }
        crate::play::transport::advance(&mut app);
        assert!(app.playback.active.is_some());
    }
}

#[test]
#[ignore = "needs RESIDENT_TIMING_STATE (a Surge XT patch state), Surge XT and an audio output; emits audio"]
fn timing_from_a_selection_to_the_new_sound() {
    use std::time::{Duration, Instant};
    let state = std::path::PathBuf::from(std::env::var_os("RESIDENT_TIMING_STATE").unwrap());
    let cache = state.parent().unwrap().join("native-cache");
    let _library = cat_smf_editor::native_library::load_at(&cache).unwrap();
    let mut song = music();
    for track in &mut song.tracks[1..] {
        track.events[1].tick = 7680;
        track.events[2].tick = 7680;
    }
    song.tracks[0].events.last_mut().unwrap().tick = 7680;
    let mut app = crate::view::tests::app(song);
    app.audition = crate::play::audition::Audition::default();
    let pump_until_sounding = |app: &mut App| {
        let start = Instant::now();
        crate::play::transport::advance(app);
        while app.audition.seconds().is_none_or(|s| s <= 0.0) {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "{:?}",
                app.audition.error()
            );
            app.audition.pump();
            std::thread::sleep(Duration::from_millis(1));
        }
        start.elapsed()
    };
    crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
    let first = pump_until_sounding(&mut app);
    std::thread::sleep(Duration::from_millis(300));
    let open = Open::Patch { track: 1 };
    let target = Target {
        open,
        bulk: false,
        original: app.loaded.as_ref().unwrap().original.clone(),
    };
    let chosen = crate::test_support::referenced(PluginState {
        reference: None,
        plugin: cat_smf_editor::chain::default_instrument().unwrap(),
        state: std::fs::read(&state).unwrap(),
        preset_name: Some("timing".into()),
        bypass: false,
    });
    let start = Instant::now();
    select(&mut app, open, Some(&target), chosen).unwrap();
    let answered = start.elapsed();
    let sounding = pump_until_sounding(&mut app);
    println!(
        "start 2 tracks {first:?}; selection answered {answered:?}; then new sound after {sounding:?}"
    );
    app.audition.stop();
}

#[path = "resident_layer_tests.rs"]
mod layers;
