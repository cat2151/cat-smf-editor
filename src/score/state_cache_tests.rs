use super::*;
use crate::host::PluginKey;

fn fixture() -> (PathBuf, PatchReference) {
    let root = std::env::temp_dir().join(format!(
        "cat-state-cache-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let bundle = root.join("test.clap");
    let patch = root.join("organ.preset");
    std::fs::write(&bundle, b"plugin-v1").unwrap();
    std::fs::write(&patch, b"patch-v1").unwrap();
    let reference = PatchReference {
        plugin: PluginKey {
            format: "CLAP".into(),
            id: "test".into(),
            name: "test".into(),
            vendor: "test".into(),
            bundle_path: bundle.to_str().unwrap().into(),
        },
        source: Source::Catalog {
            path: patch.clone(),
            file: patch,
        },
        display: "Organ".into(),
    };
    (root, reference)
}

#[test]
fn disk_hits_survive_new_session_and_partial_misses_request_only_missing_patch() {
    let (root, reference) = fixture();
    let mut first = Cache::new(Some(root.clone()));
    first.resolve(&reference, |_| Ok(vec![10, 20])).unwrap();
    first.save().unwrap();
    assert!(root.join("cache/patch-states-v1").is_dir());
    assert!(!root.join("patch-states-v1").exists());
    drop(first);
    let mut second = Cache::new(Some(root.clone()));
    let mut calls = Vec::new();
    let hit = second
        .resolve(&reference, |r| {
            calls.push(r.clone());
            Err("must not run".into())
        })
        .unwrap();
    assert_eq!(hit.state, [10, 20]);
    assert!(calls.is_empty());
    let mut other = reference.clone();
    other.source = Source::Effect {
        path: root.join("organ.preset"),
        value: "Second program".into(),
    };
    second
        .resolve(&other, |r| {
            calls.push(r.clone());
            Ok(vec![30])
        })
        .unwrap();
    assert_eq!(calls, [other.clone()]);
    second.save().unwrap();
    drop(second);
    let mut third = Cache::new(Some(root.clone()));
    for r in [&reference, &other] {
        third
            .resolve(r, |_| panic!("persisted history lost"))
            .unwrap();
    }
}

#[test]
fn changed_sources_plugins_and_corrupt_cache_regenerate_without_losing_good_entries() {
    let (root, reference) = fixture();
    let mut cache = Cache::new(Some(root.clone()));
    cache.resolve(&reference, |_| Ok(vec![1])).unwrap();
    cache.save().unwrap();
    let path = root
        .join("cache")
        .join("patch-states-v1")
        .join(format!("{}.json", key(&reference).unwrap()));
    std::fs::write(&path, b"partial").unwrap();
    let mut next = Cache::new(Some(root.clone()));
    assert_eq!(
        next.resolve(&reference, |_| Ok(vec![2])).unwrap().state,
        [2]
    );
    next.save().unwrap();
    let good = std::fs::read(&path).unwrap();
    std::fs::write(root.join("organ.preset"), b"updated patch: different size").unwrap();
    assert!(next
        .resolve(&reference, |_| Err("generation failed".into()))
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), good);
    assert_eq!(
        next.resolve(&reference, |_| Ok(vec![3])).unwrap().state,
        [3]
    );
    next.save().unwrap();
    std::fs::write(
        &reference.plugin.bundle_path,
        b"updated plugin different size",
    )
    .unwrap();
    assert_eq!(
        next.resolve(&reference, |_| Ok(vec![4])).unwrap().state,
        [4]
    );
}

#[test]
fn source_changed_during_generation_does_not_get_a_fresh_fingerprint_for_old_state() {
    let (root, reference) = fixture();
    let mut cache = Cache::new(Some(root.clone()));
    let result = cache.resolve(&reference, |_| {
        std::fs::write(root.join("organ.preset"), b"changed while generating").unwrap();
        Ok(vec![1])
    });
    assert!(result.is_err());
    assert!(cache.used.is_empty());
}

#[test]
fn removing_cache_regenerates_states_and_preserves_persistent_data() {
    let (root, reference) = fixture();
    let status = root.join("status.json");
    std::fs::write(&status, b"persistent settings").unwrap();
    let mut first = Cache::new(Some(root.clone()));
    first.resolve(&reference, |_| Ok(vec![1])).unwrap();
    first.save().unwrap();
    drop(first);
    std::fs::remove_dir_all(crate::storage_paths::cache(&root)).unwrap();
    let mut regenerated = Cache::new(Some(root.clone()));
    assert_eq!(
        regenerated
            .resolve(&reference, |_| Ok(vec![2]))
            .unwrap()
            .state,
        [2]
    );
    regenerated.save().unwrap();
    assert_eq!(std::fs::read(status).unwrap(), b"persistent settings");
    let mut reopened = Cache::new(Some(root));
    assert_eq!(
        reopened
            .resolve(&reference, |_| panic!("cache miss"))
            .unwrap()
            .state,
        [2]
    );
}
