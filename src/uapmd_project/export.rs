//! Portable UAPMD performance archive; editor metadata stays in the SMF document.
use crate::{
    chain_settings::PluginState,
    host::PluginKey,
    model::{Music, Track},
};
use std::{
    ffi::{c_char, CStr, CString},
    io::{Cursor, Write},
    path::Path,
};

unsafe extern "C" {
    fn smfe_write_project(directory: *const c_char, manifest: *const c_char) -> *const c_char;
}

pub struct Part {
    pub layers: Vec<Layer>,
    pub effects: Vec<PluginState>,
    pub volume: f64,
}

pub struct Layer {
    pub track: Track,
    pub plugin: PluginKey,
    pub state: Option<PluginState>,
    pub volume: f64,
    pub muted: bool,
}

mod graph;

pub fn archive(
    division: u16,
    master: Track,
    parts: Vec<Part>,
    master_effects: &[PluginState],
) -> Result<Vec<u8>, String> {
    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
    let root = stage.path();
    let mut entries = vec![];
    let mut add = |name: String, bytes: Vec<u8>| -> Result<(), String> {
        let path = root.join(&name);
        std::fs::create_dir_all(path.parent().ok_or("Missing parent")?)
            .map_err(|e| e.to_string())?;
        std::fs::write(path, &bytes).map_err(|e| e.to_string())?;
        entries.push((name, bytes));
        Ok(())
    };
    let smf = |track| {
        crate::smf_io::encode(&Music {
            format: 0,
            division,
            tracks: vec![track],
        })
    };
    add("clips/master.mid".into(), smf(master)?)?;
    let mut tracks = vec![];
    let mut grouped_sources = std::collections::HashSet::new();
    for (i, part) in parts.into_iter().enumerate() {
        if part.layers.is_empty() || part.layers.len() > 16 {
            return Err("Project effect graph requires between 1 and 16 instrument layers".into());
        }
        let grouped = part.layers.len() > 1;
        let mut clips = vec![];
        for (layer, data) in part.layers.iter().enumerate() {
            let name = if grouped {
                format!("clips/{i}-{layer}.mid")
            } else {
                format!("clips/{i}.mid")
            };
            add(name.clone(), smf(data.track.clone())?)?;
            if grouped {
                grouped_sources.insert(name.clone());
            }
            clips.push(name);
        }
        let graph = graph::track(i, &part, &mut add)?;
        add(
            format!("graphs/{i}.json"),
            serde_json::to_vec(&graph).map_err(|e| e.to_string())?,
        )?;
        tracks.push(serde_json::json!({"volume":part.volume,"clips":clips}));
    }
    if !master_effects.is_empty() {
        let graph = graph::master(master_effects, &mut add)?;
        add(
            "graphs/master.json".into(),
            serde_json::to_vec(&graph).map_err(|e| e.to_string())?,
        )?;
    }
    let manifest =
        CString::new(serde_json::json!({"division":division,"tracks":tracks,"master_effects":!master_effects.is_empty()}).to_string())
            .map_err(|e| e.to_string())?;
    let directory = CString::new(root.to_str().ok_or("Project directory is not UTF-8")?)
        .map_err(|e| e.to_string())?;
    let error = unsafe {
        let ptr = smfe_write_project(directory.as_ptr(), manifest.as_ptr());
        if ptr.is_null() {
            return Err("Project writer returned no response".into());
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    if !error.is_empty() {
        return Err(error);
    }
    for (i, track) in tracks.iter().enumerate() {
        if track["clips"]
            .as_array()
            .is_some_and(|clips| clips.len() > 1)
        {
            let name = format!("clips/{i}.midi2");
            entries.push((
                name.clone(),
                std::fs::read(root.join(name)).map_err(|e| e.to_string())?,
            ));
        }
    }
    entries.push((
        "project.uapmd".into(),
        std::fs::read(root.join("project.uapmd")).map_err(|e| e.to_string())?,
    ));
    entries.retain(|(name, _)| !grouped_sources.contains(name));
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Replace only after all conversion and archive creation succeeded.
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
