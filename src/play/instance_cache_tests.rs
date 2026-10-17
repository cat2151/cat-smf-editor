use super::*;
use cat_smf_editor::{chain_settings::PluginState, host::PluginKey};

fn patch(id: &str, state: u8) -> Patch {
    cat_smf_editor::score::instrument::Patch::from(cat_smf_editor::chain_settings::Settings {
        instrument: Some(crate::test_support::referenced(PluginState {
            reference: None,
            plugin: PluginKey {
                format: "CLAP".into(),
                id: id.into(),
                name: id.into(),
                vendor: String::new(),
                bundle_path: format!("X:/plugins/{id}.clap"),
            },
            state: vec![state],
            preset_name: None,
            bypass: false,
        })),
        ..Default::default()
    })
}

#[test]
fn alternating_plugins_reuses_each_instance_without_lending_it_twice() {
    let a = patch("a", 1);
    let b = patch("b", 2);
    let mut cache = Cache::default();
    cache.put(0, a.clone(), 10);
    cache.put(0, b.clone(), 20);
    for _ in 0..4 {
        let (a_instance, reload) = cache.take(0, &a).unwrap();
        assert_eq!((a_instance, reload), (10, false));
        assert!(cache.take(0, &a).is_none());
        cache.put(0, a.clone(), a_instance);
        let (b_instance, reload) = cache.take(0, &b).unwrap();
        assert_eq!((b_instance, reload), (20, false));
        cache.put(0, b.clone(), b_instance);
    }
}

#[test]
fn track_isolation_and_new_state_reload_survive_a_round_trip() {
    let a = patch("a", 1);
    let changed = patch("a", 3);
    let mut cache = Cache::default();
    cache.put(0, a.clone(), 10);
    cache.put(1, a.clone(), 11);
    assert!(cache.take(2, &a).is_none());
    assert_eq!(cache.take(1, &changed), Some((11, true)));
    assert_eq!(cache.take(0, &a), Some((10, false)));
}

#[test]
fn exact_state_takes_priority_and_effect_layout_must_match() {
    let a = patch("a", 1);
    let changed = patch("a", 2);
    let mut effect = a.clone();
    effect
        .effects
        .push(patch("effect", 1).layers.remove(0).instrument.unwrap());
    let mut cache = Cache::default();
    cache.put(0, changed, 10);
    cache.put(0, a.clone(), 11);
    assert!(cache.take(0, &effect).is_none());
    assert_eq!(cache.take(0, &a), Some((11, false)));
    assert_eq!(cache.take(0, &a), Some((10, true)));
}

#[test]
fn clearing_cache_releases_owned_instances() {
    let dropped = std::rc::Rc::new(std::cell::Cell::new(false));
    struct Instance(std::rc::Rc<std::cell::Cell<bool>>);
    impl Drop for Instance {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let mut cache = Cache::default();
    cache.put(0, patch("a", 1), Instance(dropped.clone()));
    drop(cache);
    assert!(dropped.get());
}
