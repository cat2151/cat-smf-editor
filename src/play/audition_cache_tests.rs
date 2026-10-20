//! Real hosted-instance round trips, including dormant-chain expiry.
use super::*;
use cat_smf_editor::{
    chain_settings::PluginState,
    host::{Host, PluginKey},
};
use std::{
    ffi::{c_char, c_void},
    sync::mpsc,
    time::{Duration, Instant},
};

unsafe extern "C" fn wake(_: *mut c_void) {}
unsafe extern "C" fn created(user: *mut c_void, id: i32, error: *const c_char) {
    let sender = unsafe { Box::from_raw(user as *mut mpsc::Sender<Result<i32, String>>) };
    let result = unsafe { cat_smf_editor::host::error_string(error) }.map_or(Ok(id), Err);
    let _ = sender.send(result);
}

fn vaporizer_patch() -> Patch {
    let host = unsafe { Host::new(wake, std::ptr::null_mut()) }.unwrap();
    let key = PluginKey {
        format: "CLAP".into(),
        id: "com.vastdynamics.VAST2".into(),
        name: "Vaporizer2".into(),
        vendor: "VAST Dynamics".into(),
        bundle_path: std::path::PathBuf::from(std::env::var_os("CommonProgramFiles").unwrap())
            .join("CLAP/VASTvaporizer2.clap")
            .to_string_lossy()
            .into_owned(),
    };
    let plugin = host.restore_plugin(&key).expect("installed Vaporizer2");
    let (tx, rx) = mpsc::channel::<Result<i32, String>>();
    unsafe {
        host.create_instance(
            plugin.index,
            48000,
            1024,
            created,
            Box::into_raw(Box::new(tx)) as *mut c_void,
        )
    };
    let start = Instant::now();
    let id = loop {
        host.pump();
        if let Ok(result) = rx.try_recv() {
            break result.unwrap();
        }
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(1));
    };
    assert!(id >= 0);
    let state = host.save_state(id).unwrap();
    host.destroy_instance(id);
    cat_smf_editor::score::instrument::Patch::from(cat_smf_editor::chain_settings::Settings {
        instrument: Some(crate::test_support::referenced(PluginState {
            reference: None,
            plugin: key,
            state,
            preset_name: Some("Init".into()),
            bypass: false,
        })),
        ..Default::default()
    })
}

#[test]
#[ignore = "requires installed Surge XT, Vaporizer2 and audio output; emits audio"]
fn native_round_trips_reuse_chains_and_idle_expiry_keeps_active_audio() {
    let root = std::env::temp_dir().join(format!("cat-smf-cache-test-{}", std::process::id()));
    let _library = cat_smf_editor::native_library::load_at(&root).unwrap();
    let a = Patch::default();
    let b = vaporizer_patch();
    let mut audition = Audition::default();
    audition.browser_used();
    for (index, patch) in [&a, &b, &a, &b, &a].into_iter().enumerate() {
        let start = Instant::now();
        assert!(
            audition.play_track(0, patch, &[(60, 64, Some(0.05))], None),
            "{:?}",
            audition.error()
        );
        println!("switch {index}: {:?}", start.elapsed());
        assert_eq!(
            audition.voice.as_ref().unwrap().cached_count(),
            usize::from(index != 0)
        );
        audition.pump();
        std::thread::sleep(Duration::from_millis(70));
    }
    let last = audition.last_browser_use.unwrap();
    audition.expire_cached(last + Duration::from_secs(299));
    assert_eq!(audition.voice.as_ref().unwrap().cached_count(), 1);
    audition.expire_cached(last + Duration::from_secs(300));
    assert_eq!(audition.voice.as_ref().unwrap().cached_count(), 0);
    assert_eq!(audition.voice.as_ref().unwrap().natives.len(), 1);
    assert!(audition.play_track(0, &a, &[(67, 64, Some(0.05))], None));
    audition.pump();
    assert!(audition.error().is_none());
    audition.reset();
}
