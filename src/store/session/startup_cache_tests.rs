//! Opt-in real restoration and output-device timing, separate from subjective listening.
use crate::play::{audition::Audition, playback, transport};
use cat_smf_editor::{
    instrument_layers::{self, Layer, Settings},
    model::{Event, Music, Track},
    patch_reference::PatchReference,
    score::state_cache::Cache,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires STATE_REQUEST_REFERENCE, STATE_REQUEST_WORK_DIR, RESIDENT_PLAYER, Surge XT and audio output; emits audio"]
fn startup_session_cache_restores_layers_and_reaches_audio_without_player() {
    let work = PathBuf::from(std::env::var_os("STATE_REQUEST_WORK_DIR").unwrap());
    let root = work.join(format!("startup-{}", std::process::id()));
    let songs = root.join("songs");
    std::fs::create_dir_all(&songs).unwrap();
    let _library = cat_smf_editor::native_library::load_at(&root).unwrap();
    let reference: PatchReference = serde_json::from_slice(
        &std::fs::read(std::env::var_os("STATE_REQUEST_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    let event = |tick, status, data: &[u8]| Event {
        delta: 0,
        tick,
        status,
        meta_type: if status == 0xff { 0x2f } else { 0 },
        data: data.to_vec(),
    };
    let mut track = Track {
        events: vec![
            event(0, 0x90, &[60, 80]),
            event(7680, 0x80, &[60, 0]),
            event(7680, 0xff, &[]),
        ],
    };
    let settings = Settings {
        layers: vec![
            Layer {
                patch: Some(reference.clone()),
                volume_db: -6.0,
                ..Default::default()
            },
            Layer {
                id: 2,
                patch: Some(reference),
                octave: 1,
                volume_db: -9.0,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    instrument_layers::write(
        &mut track,
        &settings,
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    let music = Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    };
    std::fs::write(
        songs.join("organ.mid"),
        cat_smf_editor::smf_io::encode(&music).unwrap(),
    )
    .unwrap();
    let status = root.join("status.json");
    std::fs::write(
        &status,
        serde_json::json!({
            "navigation": { "last_mid": songs.join("organ.mid") }, "session": { "file":"organ.mid", "track":1,
            "mbt":null, "tick":null, "row":null, "note_mode":true }, "program_target":"Current",
        })
        .to_string(),
    )
    .unwrap();
    for warm in [false, true] {
        let mut app = crate::view::tests::app(Music {
            format: 0,
            division: 480,
            tracks: vec![],
        });
        app.loaded = None;
        app.config = crate::store::config::Store::open(status.clone());
        app.state_cache = Cache::new(Some(root.clone()));
        app.player_selection.executable = Some(if warm {
            PathBuf::from("X:/missing/player.exe")
        } else {
            std::env::var_os("RESIDENT_PLAYER").unwrap().into()
        });
        app.audition = Audition::default();
        let start = Instant::now();
        app.restore_session();
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        assert_eq!(
            app.loaded.as_ref().unwrap().patches[0].saved().unwrap(),
            settings
        );
        let restored = start.elapsed();
        playback::start(&mut app, playback::Mode::AllTracks);
        transport::advance(&mut app);
        while app.audition.seconds().is_none_or(|s| s <= 0.0) {
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "{:?}",
                app.audition.error()
            );
            app.audition.pump();
            std::thread::sleep(Duration::from_millis(1));
        }
        println!("warm={warm}: session restored={restored:?}, first output callback={:?}; layers=2; explicit player {}", start.elapsed(), if warm { "invalid (cache-only)" } else { "real" });
        app.state_cache.save().unwrap();
        app.audition.reset();
    }
}
