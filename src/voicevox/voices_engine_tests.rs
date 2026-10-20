use super::{
    cache, engine,
    score::{demo, one_note},
    Voice,
};

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path; offline synthesis only"]
fn voicevox_every_humming_style_synthesizes_its_own_cached_audio() {
    let configured = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.voicevox-engine-path"
    ))
    .unwrap();
    let engine = engine::connect(
        50131,
        Some(std::path::Path::new(configured.trim())),
        engine::START_TIMEOUT,
    )
    .unwrap();
    let voices = engine.client.voices().unwrap();
    assert!(!voices.is_empty());
    assert!(voices.iter().any(|v| v.same_style(&Voice::default())));
    let root = tempfile::tempdir().unwrap();
    let mut keys = std::collections::HashSet::new();
    for voice in &voices {
        let score = demo(voice.clone());
        assert!(keys.insert(cache::key(&score, 48000)));
        let samples = cache::load_or_sing(root.path(), &score, 48000, |json, rate| {
            engine.client.sing_as(json, rate, voice)
        })
        .unwrap_or_else(|e| panic!("{}: {e}", voice.label()));
        assert!(
            samples.iter().any(|v| v.abs() > 0.001),
            "{} is silent",
            voice.label()
        );
        let again =
            cache::load_or_sing(root.path(), &score, 48000, |_, _| panic!("cache missed")).unwrap();
        assert_eq!(samples, again);
        println!("{}: {} samples", voice.label(), samples.len());
    }
    let missing = Voice {
        style_id: u32::MAX,
        ..Voice::default()
    };
    assert!(engine
        .client
        .sing_as(&one_note(60, 30).json(), 48000, &missing)
        .unwrap_err()
        .contains("unavailable"));
    println!("Verified {} humming styles", voices.len());
}
