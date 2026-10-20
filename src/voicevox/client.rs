//! VOICEVOX engine の REST API。
use super::{Voice, QUERY_SPEAKER};
use std::time::Duration;

pub const DEFAULT_URL: &str = "http://127.0.0.1:50021";

pub struct Client {
    url: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(url: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(2)))
            .build()
            .new_agent();
        Self {
            url: url.trim_end_matches('/').to_owned(),
            agent,
        }
    }

    /// engine が応答すればその version。
    pub fn version(&self) -> Result<String, String> {
        let response = self
            .agent
            .get(format!("{}/version", self.url))
            .config()
            .timeout_global(Some(Duration::from_secs(2)))
            .build()
            .call()
            .map_err(|e| format!("VOICEVOX /version: {e}"))?;
        let version = body("/version", response)?;
        Ok(version.trim().trim_matches('"').to_owned())
    }

    /// 譜面 JSON を ずんだもん ハミングで歌わせ、`rate` Hz・stereo・16bit の WAV を返す。
    pub fn sing(&self, score_json: &str, rate: u32) -> Result<Vec<u8>, String> {
        self.sing_as(score_json, rate, &Voice::default())
    }

    pub fn voices(&self) -> Result<Vec<Voice>, String> {
        let response = self
            .agent
            .get(format!("{}/singers", self.url))
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .build()
            .call()
            .map_err(|e| format!("VOICEVOX /singers: {e}"))?;
        voices(&body("/singers", response)?)
    }

    pub fn sing_as(&self, score_json: &str, rate: u32, voice: &Voice) -> Result<Vec<u8>, String> {
        if !self.voices()?.iter().any(|v| v.same_style(voice)) {
            return Err(format!("VOICEVOX voice unavailable: {}", voice.label()));
        }
        let query = self.post(
            &format!("/sing_frame_audio_query?speaker={QUERY_SPEAKER}"),
            score_json.as_bytes(),
        )?;
        let mut query: serde_json::Value = serde_json::from_slice(&query)
            .map_err(|e| format!("VOICEVOX /sing_frame_audio_query: {e}"))?;
        let fields = query
            .as_object_mut()
            .ok_or("VOICEVOX /sing_frame_audio_query: not an object")?;
        fields.insert("outputSamplingRate".into(), rate.into());
        fields.insert("outputStereo".into(), true.into());
        self.post(
            &format!("/frame_synthesis?speaker={}", voice.style_id),
            query.to_string().as_bytes(),
        )
    }

    fn post(&self, path: &str, json: &[u8]) -> Result<Vec<u8>, String> {
        let response = self
            .agent
            .post(format!("{}{path}", self.url))
            .header("Content-Type", "application/json")
            .send(json)
            .map_err(|e| format!("VOICEVOX {path}: {e}"))?;
        let status = response.status();
        let mut response = response;
        // ureq の既定上限 10 MiB は 48kHz stereo 16bit で約55秒分にしかならない。
        let bytes = response
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_vec()
            .map_err(|e| format!("VOICEVOX {path}: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "VOICEVOX {path}: {status} {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        Ok(bytes)
    }
}

fn body(path: &str, mut response: ureq::http::Response<ureq::Body>) -> Result<String, String> {
    let status = response.status();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("VOICEVOX {path}: {e}"))?;
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("VOICEVOX {path}: {status} {text}"))
    }
}

/// Only frame decoders are humming voices; teachers are query generators.
pub fn voices(json: &str) -> Result<Vec<Voice>, String> {
    #[derive(serde::Deserialize)]
    struct Singer {
        name: String,
        speaker_uuid: String,
        styles: Vec<Style>,
    }
    #[derive(serde::Deserialize)]
    struct Style {
        name: String,
        id: u32,
        #[serde(rename = "type")]
        kind: String,
    }
    let singers: Vec<Singer> =
        serde_json::from_str(json).map_err(|e| format!("VOICEVOX /singers: {e}"))?;
    let mut voices: Vec<_> = singers
        .into_iter()
        .flat_map(|s| {
            s.styles
                .into_iter()
                .filter(|v| v.kind == "frame_decode")
                .map(move |v| Voice {
                    speaker_uuid: s.speaker_uuid.clone(),
                    name: s.name.clone(),
                    style_id: v.id,
                    style_name: v.name,
                })
        })
        .collect();
    voices.sort_by_key(|v| (v.name != super::SINGER, !v.same_style(&Voice::default())));
    Ok(voices)
}

#[cfg(test)]
mod tests {
    #[test]
    fn zundamon_normal_leads_without_reordering_other_characters() {
        let voices = super::voices(r#"[
            {"name":"四国めたん","speaker_uuid":"m","styles":[{"name":"ノーマル","id":3000,"type":"frame_decode"}]},
            {"name":"ずんだもん","speaker_uuid":"388f246b-8c41-4ac1-8e2d-5d79f3ff56d9","styles":[{"name":"あまあま","id":3002,"type":"frame_decode"},{"name":"ノーマル","id":3003,"type":"frame_decode"}]},
            {"name":"春日部つむぎ","speaker_uuid":"t","styles":[{"name":"ノーマル","id":3004,"type":"frame_decode"}]}]"#).unwrap();
        assert_eq!(
            voices.iter().map(|v| v.style_id).collect::<Vec<_>>(),
            [3003, 3002, 3000, 3004]
        );
    }

    #[test]
    fn catalog_keeps_all_frame_decoders_and_excludes_talk_and_teachers() {
        let voices = super::voices(
            r#"[
            {"name":"A","speaker_uuid":"a","styles":[
                {"name":"Normal","id":1,"type":"frame_decode"},
                {"name":"Sweet","id":2,"type":"frame_decode"},
                {"name":"Talk","id":3,"type":"talk"}]},
            {"name":"B","speaker_uuid":"b","styles":[
                {"name":"Normal","id":4,"type":"frame_decode"},
                {"name":"Teacher","id":5,"type":"singing_teacher"}]}]"#,
        )
        .unwrap();
        assert_eq!(
            voices.iter().map(|v| v.style_id).collect::<Vec<_>>(),
            [1, 2, 4]
        );
        assert_eq!(voices[2].speaker_uuid, "b");
    }
}
