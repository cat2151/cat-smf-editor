use crate::{
    host::PluginKey,
    imported_state, instrument_layers,
    model::{Event, Music, Track},
    patch_reference::{PatchReference, Source},
    patch_roots::{self, Roots},
};
use serde::Deserialize;
use std::{
    ffi::{c_char, CStr, CString},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub mod export;
#[cfg(test)]
mod tests;

unsafe extern "C" {
    fn smfe_read_project(path: *const c_char) -> *const c_char;
}

#[derive(Deserialize)]
struct RawEvent {
    tick: u64,
    status: u8,
    meta_type: u8,
    data: Vec<u8>,
}
#[derive(Deserialize)]
struct RawTrack {
    graph: String,
    events: Vec<RawEvent>,
}
#[derive(Deserialize)]
struct RawProject {
    division: u16,
    master: Vec<RawEvent>,
    master_graph: String,
    tracks: Vec<RawTrack>,
}
#[derive(Deserialize)]
struct Graph {
    #[serde(default)]
    plugins: Vec<Plugin>,
    #[serde(default)]
    connections: Vec<Connection>,
    #[serde(default)]
    nodes: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct Plugin {
    node_id: String,
    plugin_id: String,
    format: String,
    display_name: String,
    state_file: String,
}
#[derive(Deserialize)]
struct Connection {
    bus_type: String,
    source: Endpoint,
    target: Endpoint,
}
#[derive(Deserialize)]
struct Endpoint {
    #[serde(rename = "type")]
    kind: String,
    node_id: String,
    bus_index: u32,
}

pub struct Import {
    pub music: Music,
    pub patches: Vec<PatchReference>,
}

impl Import {
    pub fn into_music(mut self, roots: &mut dyn Roots) -> Result<Music, String> {
        for (track, patch) in self.music.tracks.iter_mut().skip(1).zip(self.patches) {
            let reference = patch_roots::resolve(&patch, roots)?;
            instrument_layers::write(
                track,
                &instrument_layers::Settings {
                    layers: vec![instrument_layers::Layer {
                        patch: Some(reference),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                roots,
            )?;
        }
        Ok(self.music)
    }
}

pub fn read(path: &Path) -> Result<Import, String> {
    read_with(path, imported_state::store)
}

fn read_with(
    path: &Path,
    mut store: impl FnMut(&[u8]) -> Result<String, String>,
) -> Result<Import, String> {
    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
    let file = extract(path, stage.path())?;
    let name = CString::new(file.to_str().ok_or("Project path is not UTF-8")?)
        .map_err(|e| e.to_string())?;
    // The DLL owns the thread-local response until the next call; copy it immediately.
    let response = unsafe {
        let ptr = smfe_read_project(name.as_ptr());
        if ptr.is_null() {
            return Err("Project reader returned no data".into());
        }
        CStr::from_ptr(ptr).to_bytes().to_vec()
    };
    let value: serde_json::Value = serde_json::from_slice(&response).map_err(|e| e.to_string())?;
    if let Some(error) = value.get("error").and_then(|e| e.as_str()) {
        return Err(error.into());
    }
    let raw: RawProject = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let root = file.parent().ok_or("Project directory unavailable")?;
    if !raw.master_graph.is_empty() {
        let graph = graph(root, &raw.master_graph)?;
        if !graph.plugins.is_empty()
            || !graph.connections.is_empty()
            || graph.nodes.iter().any(|node| {
                node.get("type").and_then(|v| v.as_str()) != Some("webaudio:AnalyserNode")
            })
        {
            return Err("Master graph processing is not supported".into());
        }
    }
    let mut master = track(raw.master)?;
    master.events.dedup_by(|a, b| {
        a.tick == b.tick && a.status == b.status && a.meta_type == b.meta_type && a.data == b.data
    });
    let mut tracks = vec![master];
    let mut pending = vec![];
    for raw_track in raw.tracks {
        let mut graph = graph(root, &raw_track.graph)?;
        validate_graph(&graph)?;
        let plugin = graph.plugins.remove(0);
        let mut track = track(raw_track.events)?;
        track.events.insert(
            0,
            Event {
                delta: 0,
                tick: 0,
                status: 0xff,
                meta_type: 3,
                data: plugin.display_name.as_bytes().to_vec(),
            },
        );
        let state =
            std::fs::read(contained(root, &plugin.state_file)?).map_err(|e| e.to_string())?;
        if state.is_empty() || state.len() > 16 * 1024 * 1024 {
            return Err("Invalid project plugin state size".into());
        }
        pending.push((plugin, state));
        tracks.push(track);
    }
    let music = Music {
        format: 1,
        division: raw.division,
        tracks,
    };
    crate::smf_io::encode(&music)?;
    // Validate the complete input before creating any permanent state assets.
    let patches = pending
        .into_iter()
        .map(|(plugin, state)| {
            Ok(PatchReference {
                plugin: PluginKey {
                    format: plugin.format,
                    id: plugin.plugin_id,
                    name: plugin.display_name.clone(),
                    vendor: String::new(),
                    bundle_path: String::new(),
                },
                source: Source::Imported {
                    sha256: store(&state)?,
                },
                display: plugin.display_name,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Import { music, patches })
}

fn track(events: Vec<RawEvent>) -> Result<Track, String> {
    let mut events: Vec<_> = events
        .into_iter()
        .map(|e| Event {
            delta: 0,
            tick: e.tick,
            status: e.status,
            meta_type: e.meta_type,
            data: e.data,
        })
        .collect();
    events.sort_by_key(|e| (e.tick, e.status == 0xff, e.meta_type));
    let mut previous = 0;
    for event in &mut events {
        event.delta = u32::try_from(event.tick - previous).map_err(|e| e.to_string())?;
        previous = event.tick;
    }
    events.push(Event {
        delta: 0,
        tick: previous,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    });
    Ok(Track { events })
}

fn graph(root: &Path, name: &str) -> Result<Graph, String> {
    serde_json::from_slice(&std::fs::read(contained(root, name)?).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

fn validate_graph(graph: &Graph) -> Result<(), String> {
    if graph.plugins.len() != 1 || !graph.nodes.is_empty() {
        return Err(
            "Only one instrument per track is supported; graph restoration is not supported".into(),
        );
    }
    let plugin = &graph.plugins[0];
    if !matches!(plugin.format.as_str(), "CLAP" | "VST3") {
        return Err("Unsupported instrument format".into());
    }
    let mut input = false;
    let mut output = false;
    for edge in &graph.connections {
        let incoming = edge.source.kind == "graph_input"
            && edge.source.node_id == "graph:input"
            && edge.target.kind == "plugin"
            && edge.target.node_id == plugin.node_id
            && edge.source.bus_index == 0
            && edge.target.bus_index == 0;
        let outgoing = edge.source.kind == "plugin"
            && edge.source.node_id == plugin.node_id
            && edge.target.kind == "graph_output"
            && edge.target.node_id == "graph:output"
            && edge.source.bus_index == edge.target.bus_index;
        match edge.bus_type.as_str() {
            "event" if incoming => input = true,
            "audio" if incoming => {}
            "audio" if outgoing => output |= edge.source.bus_index == 0,
            _ => return Err("Unsupported instrument graph connection".into()),
        }
    }
    if !input || !output {
        return Err("Instrument graph has no MIDI input or audio output".into());
    }
    Ok(())
}

fn contained(root: &Path, name: &str) -> Result<PathBuf, String> {
    let relative = PathBuf::from(name.replace('\\', "/"));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Invalid project reference path".into());
    }
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(root.canonicalize().map_err(|e| e.to_string())?) {
        return Err("Project reference escapes the archive".into());
    }
    Ok(path)
}

fn extract(path: &Path, root: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let relative = Path::new(&name);
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || entry.is_symlink()
        {
            return Err("Invalid archive entry path".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Project archive too large")?;
        if total > 64 * 1024 * 1024 {
            return Err("Project archive too large".into());
        }
        let output = root.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
            continue;
        }
        std::fs::create_dir_all(output.parent().ok_or("Invalid archive path")?)
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != entry.size() {
            return Err("Archive entry length mismatch".into());
        }
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut output, &bytes).map_err(|e| e.to_string())?;
    }
    let project = root.join("project.uapmd");
    if !project.is_file() {
        return Err("Archive has no project.uapmd".into());
    }
    Ok(project)
}
