//! Use the selector's existing catalog metadata and exclusive four-role classifier.
use crate::app::App;
use cat_smf_editor::{
    patch_reference::{PatchReference, Source},
    patch_roots::Roots,
};
use cmrt_patches::{
    pitch_tracking::{
        pitched_percussion_indices, PitchTrackingTable, TrackingCandidate, PITCH_TRACKING_FILE,
        PITCH_TRACKING_URL,
    },
    DrumPatchRole, PatchRoleIndex, PatchRoleInput,
};

#[derive(serde::Deserialize)]
pub(super) struct Catalog {
    format_version: u32,
    pub(super) patches: Vec<Entry>,
}
#[derive(serde::Deserialize)]
pub(super) struct Entry {
    pub(super) audio: Audio,
}
#[derive(serde::Deserialize)]
pub(super) struct Audio {
    pub(super) reference: Reference,
    normalized_display: String,
    selector_category: Option<String>,
}
#[derive(serde::Deserialize)]
pub(super) struct Reference {
    pub(super) plugin: String,
    display: String,
}

pub(super) fn load(app: &mut App) -> Result<[Vec<PatchReference>; 4], String> {
    let home = dirs::config_local_dir().ok_or("No local application directory")?;
    let paths = [
        home.join("clap-mml-render-tui/patch-catalog/catalog.json"),
        home.join("cat-plugin-player/patch-catalog/surge-xt.json"),
    ];
    let bytes = if let Some(path) = paths.iter().find(|p| p.is_file()) {
        std::fs::read(path).map_err(|e| e.to_string())?
    } else {
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(10)))
                .build(),
        );
        agent.get("https://raw.githubusercontent.com/cat2151/cat-music-patterns/main/surge-xt-patch-catalog.json")
            .call().and_then(|mut r| r.body_mut().read_to_vec()).map_err(|e| e.to_string())?
    };
    let catalog = parse(&bytes)?;
    let mut plugin = cat_smf_editor::chain::default_instrument()?;
    let mut roots = app
        .patch_roots
        .lookup(app.player_selection.executable.clone());
    let installed = roots
        .plugin(&plugin, false)
        .or_else(|_| roots.plugin(&plugin, true))?;
    plugin.bundle_path = installed.bundle_path.clone();
    let entries: Vec<_> = catalog
        .patches
        .iter()
        .filter(|e| e.audio.reference.plugin == format!("id:{}", plugin.id))
        .collect();
    let table = pitch_tracking_table(&home);
    let mut result: [Vec<PatchReference>; 4] = Default::default();
    for (row, display) in rows(&entries, &plugin.id, table.as_ref()) {
        let Some(path) = installed
            .roots
            .iter()
            .map(|r| cat_smf_editor::patch_roots::join(r, &display))
            .find(|p| p.is_file())
        else {
            continue;
        };
        result[row].push(PatchReference {
            plugin: plugin.clone(),
            source: Source::Catalog {
                path: path.clone(),
                file: path,
            },
            display,
        });
    }
    for (row, entries) in result.iter_mut().enumerate() {
        entries.sort_by(|a, b| a.display.cmp(&b.display));
        entries.dedup();
        if entries.is_empty() {
            return Err(format!(
                "No installed {} patches in the catalog",
                cat_smf_editor::score::drums::NAMES[row]
            ));
        }
    }
    Ok(result)
}

pub(super) fn parse(bytes: &[u8]) -> Result<Catalog, String> {
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if catalog.format_version != 5 {
        return Err("Unsupported patch catalog version".into());
    }
    Ok(catalog)
}

/// Drum row index and display of each classified entry. Percussion patches whose pitch
/// follows the note are left out, because the patch browser lists them under lead instead.
pub(super) fn rows(
    entries: &[&Entry],
    plugin_id: &str,
    table: Option<&PitchTrackingTable>,
) -> Vec<(usize, String)> {
    let inputs: Vec<_> = entries
        .iter()
        .map(|e| PatchRoleInput {
            display: &e.audio.reference.display,
            normalized_display: &e.audio.normalized_display,
            selector_category: e.audio.selector_category.as_deref(),
            plugin: Some("Surge XT"),
        })
        .collect();
    let melodic = table.map_or_else(Vec::new, |table| {
        let candidates: Vec<_> = inputs
            .iter()
            .map(|&role| TrackingCandidate {
                group: (),
                plugin_id: Some(plugin_id),
                role,
            })
            .collect();
        pitched_percussion_indices(&candidates, table)
    });
    let index = PatchRoleIndex::build(inputs.iter().copied(), &[]);
    inputs
        .iter()
        .enumerate()
        .filter(|(i, _)| melodic.binary_search(i).is_err())
        .filter_map(|(_, input)| {
            let role = index.drum_role_of(input.display)?;
            let row = DrumPatchRole::ALL.iter().position(|r| *r == role)?;
            Some((row, input.display.to_string()))
        })
        .collect()
}

/// The table cached by cmrt or cat-plugin-player, else the shared one. Without a table
/// every percussion patch stays on its row.
fn pitch_tracking_table(home: &std::path::Path) -> Option<PitchTrackingTable> {
    let cached = ["clap-mml-render-tui", "cat-plugin-player"]
        .iter()
        .map(|app| {
            home.join(app)
                .join("patch-catalog")
                .join(PITCH_TRACKING_FILE)
        })
        .find_map(|path| PitchTrackingTable::parse(&std::fs::read(path).ok()?).ok());
    let result = cached.ok_or(()).or_else(|()| {
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(10)))
                .build(),
        );
        agent
            .get(PITCH_TRACKING_URL)
            .call()
            .and_then(|mut r| r.body_mut().read_to_vec())
            .map_err(|e| e.to_string())
            .and_then(|bytes| PitchTrackingTable::parse(&bytes))
    });
    result
        .inspect_err(|error| eprintln!("Pitch tracking table unavailable: {error}"))
        .ok()
}
