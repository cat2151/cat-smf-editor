use super::catalog::{parse, rows, Entry};
use cmrt_patches::pitch_tracking::{PitchTrackingTable, PITCH_TRACKING_FILE};
use std::collections::{BTreeMap, BTreeSet};

const SURGE: &str = "org.surge-synth-team.surge-xt";

fn entries(patches: &[(&str, Option<&str>)]) -> Vec<Entry> {
    patches
        .iter()
        .map(|(display, category)| {
            serde_json::from_value(serde_json::json!({ "audio": {
                "reference": { "plugin": format!("id:{SURGE}"), "display": display },
                "normalized_display": display.to_lowercase(),
                "selector_category": category,
            }}))
            .unwrap()
        })
        .collect()
}

fn table(entries: &[(&str, &str, bool)]) -> PitchTrackingTable {
    let entries: Vec<_> = entries
        .iter()
        .map(|(plugin_id, display, tracks_keyboard)| {
            serde_json::json!({
                "plugin_id": plugin_id, "display": display,
                "tracks_keyboard": tracks_keyboard, "observations": [],
            })
        })
        .collect();
    let json = serde_json::json!({ "format_version": 1, "entries": entries });
    PitchTrackingTable::parse(json.to_string().as_bytes()).unwrap()
}

fn by_row(rows: Vec<(usize, String)>) -> [BTreeSet<String>; 4] {
    let mut result: [BTreeSet<String>; 4] = Default::default();
    for (row, display) in rows {
        result[row].insert(display);
    }
    result
}

fn set(displays: &[&str]) -> BTreeSet<String> {
    displays.iter().map(|d| d.to_string()).collect()
}

#[test]
fn tracking_percussion_leaves_the_percussion_row_only() {
    let all = entries(&[
        ("Percussion/Drum One.fxp", Some("Percussion")),
        ("Percussion/Conga.fxp", Some("Percussion")),
        ("Percussion/Kick 909ish.fxp", Some("Percussion")),
        ("Percussion/Snare Tight.fxp", Some("Percussion")),
        ("Percussion/Hat Closed.fxp", Some("Percussion")),
    ]);
    let refs: Vec<_> = all.iter().collect();
    let table = table(&[
        (SURGE, "Percussion/Drum One.fxp", true),
        (SURGE, "Percussion/Conga.fxp", false),
        (SURGE, "Percussion/Kick 909ish.fxp", true),
        (SURGE, "Percussion/Snare Tight.fxp", true),
        (SURGE, "Percussion/Hat Closed.fxp", true),
    ]);
    let rows = by_row(rows(&refs, SURGE, Some(&table)));
    assert_eq!(rows[0], set(&["Percussion/Kick 909ish.fxp"]));
    assert_eq!(rows[1], set(&["Percussion/Snare Tight.fxp"]));
    assert_eq!(rows[2], set(&["Percussion/Hat Closed.fxp"]));
    assert_eq!(rows[3], set(&["Percussion/Conga.fxp"]));
}

#[test]
fn another_plugins_entry_or_no_table_keeps_the_patch() {
    let all = entries(&[("Percussion/Drum One.fxp", Some("Percussion"))]);
    let refs: Vec<_> = all.iter().collect();
    let other = table(&[(
        "com.digital-suburban.dexed",
        "Percussion/Drum One.fxp",
        true,
    )]);
    for table in [Some(&other), None] {
        let rows = by_row(rows(&refs, SURGE, table));
        assert_eq!(rows[3], set(&["Percussion/Drum One.fxp"]));
    }
}

/// Real catalog (the same file `load` reads first) and the sibling repo's table.
#[test]
#[ignore = "reads cmrt's patch catalog cache and ../cat-music-patterns; no audio"]
fn installed_tracking_percussion_leaves_the_percussion_row() {
    let home = dirs::config_local_dir().unwrap();
    let catalog =
        parse(&std::fs::read(home.join("clap-mml-render-tui/patch-catalog/catalog.json")).unwrap())
            .unwrap();
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cat-music-patterns")
        .join(PITCH_TRACKING_FILE);
    let table = PitchTrackingTable::parse(&std::fs::read(file).unwrap()).unwrap();
    let plugin = cat_smf_editor::chain::default_instrument().unwrap();
    let refs: Vec<_> = catalog
        .patches
        .iter()
        .filter(|e| e.audio.reference.plugin == format!("id:{}", plugin.id))
        .collect();
    let before = by_row(rows(&refs, &plugin.id, None));
    let after = by_row(rows(&refs, &plugin.id, Some(&table)));
    let tracking: BTreeSet<String> = table
        .tracking()
        .filter(|(id, _)| *id == plugin.id)
        .map(|(_, display)| display.to_string())
        .collect();
    let counts: BTreeMap<_, _> = cat_smf_editor::score::drums::NAMES
        .iter()
        .zip(before.iter().zip(&after))
        .map(|(name, (b, a))| (*name, (b.len(), a.len())))
        .collect();
    println!(
        "plugin {}, entries {}, table true {}, rows before -> after {counts:?}",
        plugin.id,
        refs.len(),
        tracking.len()
    );
    let removed: BTreeSet<_> = before[3].difference(&after[3]).cloned().collect();
    for display in tracking.difference(&removed) {
        println!("not removed: {display}");
    }
    assert_eq!(
        removed,
        before[3].intersection(&tracking).cloned().collect()
    );
    assert!(after[3].is_disjoint(&tracking));
    assert_eq!(before[..3], after[..3]);
    println!("first percussion patch: {:?}", after[3].first());
}
