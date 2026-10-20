//! The editor's own sequencer-specific payloads, named by what they restore.
use super::Part;
use crate::{
    app_meta, chain_settings,
    edit::tempo_curve::{self, Curve},
    instrument_layers, master_effects, master_references,
    score::layer_track::DEFAULT_LABEL,
};

pub(super) fn parts(data: &[u8]) -> Option<Vec<Part>> {
    let (kind, value) = if let Some(value) = data.strip_prefix(app_meta::TRACK_VOLUME) {
        ("Track volume", volume(value))
    } else if let Some(value) = data.strip_prefix(app_meta::AUTO_SAVE) {
        let state = if value == b"\x01\x01" {
            "On"
        } else {
            "invalid"
        };
        ("Auto save", state.into())
    } else if let Some(json) = data.strip_prefix(crate::voicevox::voice::PREFIX) {
        (
            "VOICEVOX voice",
            serde_json::from_slice::<crate::voicevox::Voice>(json)
                .map(|v| v.label())
                .unwrap_or_else(|_| "invalid".into()),
        )
    } else if let Some(json) = data.strip_prefix(tempo_curve::PREFIX) {
        ("Tempo curve", tempo(json))
    } else if let Some(json) = data.strip_prefix(instrument_layers::PREFIX) {
        ("Instrument layers", references(json, true))
    } else if let Some(json) = data.strip_prefix(master_references::PREFIX) {
        ("Master effects", references(json, false))
    } else if data.starts_with(chain_settings::PREFIX) {
        // Plugin states make these large; their size is what matters, not their bytes.
        (
            "Track chain with plugin states",
            format!("{} bytes", data.len()),
        )
    } else if data.starts_with(master_effects::PREFIX) {
        (
            "Master effects with plugin states",
            format!("{} bytes", data.len()),
        )
    } else {
        return None;
    };
    Some(super::field(kind, ": ", value))
}

// Schema version 1, then the gain as little-endian f32.
fn volume(value: &[u8]) -> String {
    match value {
        [1, gain @ ..] if gain.len() == 4 => {
            let gain = f32::from_le_bytes(gain.try_into().unwrap());
            format!("{:+.1} dB", 20.0 * gain.log10())
        }
        _ => "invalid".into(),
    }
}

fn tempo(json: &[u8]) -> String {
    match serde_json::from_slice::<Curve>(json)
        .map_err(|e| e.to_string())
        .and_then(|c| c.validate().map(|()| c))
    {
        Ok(curve) => {
            curve
                .points
                .iter()
                .map(|p| format!("{:.2}", p.bpm))
                .collect::<Vec<_>>()
                .join(" → ")
                + " BPM"
        }
        Err(error) => format!("invalid ({error})"),
    }
}

fn references(json: &[u8], layers: bool) -> String {
    let settings = match serde_json::from_slice::<instrument_layers::Settings>(json)
        .map_err(|e| e.to_string())
        .and_then(|s| s.validate_portable().map(|()| s))
    {
        Ok(settings) => settings,
        Err(error) => return format!("invalid ({error})"),
    };
    let mut items = Vec::new();
    if layers {
        items.push(
            settings
                .layers
                .iter()
                .map(layer)
                .collect::<Vec<_>>()
                .join(" + "),
        );
    }
    if !settings.effects.is_empty() {
        let effects = settings
            .effects
            .iter()
            .map(|e| {
                if e.bypass {
                    format!("{} (Bypass)", e.patch.display)
                } else {
                    e.patch.display.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" → ");
        items.push(if layers {
            format!("Effects {effects}")
        } else {
            effects
        });
    }
    items.join(" · ")
}

fn layer(layer: &instrument_layers::Layer) -> String {
    let name = layer.patch.as_ref().map_or(DEFAULT_LABEL, |p| &p.display);
    let details: Vec<String> = [
        (layer.octave != 0).then(|| format!("{:+} oct", layer.octave)),
        (layer.volume_db != 0.0).then(|| format!("{:+.1} dB", layer.volume_db)),
        layer.mute.then(|| "Mute".into()),
        layer.solo.then(|| "Solo".into()),
    ]
    .into_iter()
    .flatten()
    .collect();
    if details.is_empty() {
        name.into()
    } else {
        format!("{name} ({})", details.join(", "))
    }
}
