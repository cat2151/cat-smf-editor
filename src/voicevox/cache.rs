//! 歌わせた WAV を、譜面・style・rate の hash で保存して使い回す。
use super::{pcm, score::Score, QUERY_SPEAKER};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

/// 既定の保存先。消しても engine で作り直せるので cache 側に置く。
pub fn directory() -> Option<PathBuf> {
    crate::storage_paths::cache_directory().map(|cache| cache.join("voicevox"))
}

pub fn key(score: &Score, rate: u32) -> String {
    let mut hash = Sha256::new();
    hash.update(score.json().as_bytes());
    hash.update(
        format!(
            "\0{QUERY_SPEAKER}\0{}\0{rate}\0{}",
            score.voice.style_id, score.voice.speaker_uuid
        )
        .as_bytes(),
    );
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// cache にあれば読み、無ければ `sing`（譜面 JSON と rate を受けて WAV を返す）で作って保存する。
/// 戻り値は stereo インターリーブの f32 sample。
pub fn load_or_sing(
    directory: &Path,
    score: &Score,
    rate: u32,
    sing: impl FnOnce(&str, u32) -> Result<Vec<u8>, String>,
) -> Result<Vec<f32>, String> {
    let path = directory.join(format!("{}.wav", key(score, rate)));
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            let bytes = sing(&score.json(), rate)?;
            check(&bytes, rate)?;
            store(directory, &path, &bytes)?;
            bytes
        }
    };
    check(&bytes, rate)
}

fn check(bytes: &[u8], rate: u32) -> Result<Vec<f32>, String> {
    let (actual, samples) = pcm::decode(bytes)?;
    if actual != rate {
        return Err(format!("VOICEVOX WAV: {actual} Hz, expected {rate} Hz"));
    }
    Ok(samples)
}

/// 置き換えは atomic にし、読み手が書きかけの file を見ないようにする。
fn store(directory: &Path, path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| format!("{}: {e}", directory.display()))?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voicevox::score::ScoreNote;

    fn score(key: u8) -> Score {
        Score {
            voice: Default::default(),
            notes: vec![ScoreNote {
                key: Some(key),
                frame_length: 1,
                lyric: "ど".into(),
            }],
            starts: vec![0],
            sources: vec![Some(0)],
        }
    }

    fn wav(rate: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 2] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * 4).to_le_bytes());
        for v in [4u16, 16] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(b"data");
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(&[0, 64, 0, 192]);
        b
    }

    #[test]
    fn a_stored_wav_is_reused_without_singing() {
        let root = tempfile::tempdir().unwrap();
        let first = load_or_sing(root.path(), &score(60), 48000, |json, rate| {
            assert!(json.contains("\"key\":60"));
            Ok(wav(rate))
        })
        .unwrap();
        assert_eq!(first, vec![0.5, -0.5]);
        let again = load_or_sing(root.path(), &score(60), 48000, |_, _| panic!("sung twice"));
        assert_eq!(again.unwrap(), first);
        let mut calls = 0;
        load_or_sing(root.path(), &score(62), 48000, |_, rate| {
            calls += 1;
            Ok(wav(rate))
        })
        .unwrap();
        load_or_sing(root.path(), &score(62), 44100, |_, rate| {
            calls += 1;
            Ok(wav(rate))
        })
        .unwrap();
        assert_eq!(calls, 2);
    }

    #[test]
    fn different_voices_never_reuse_the_same_wav() {
        let root = tempfile::tempdir().unwrap();
        let first = score(60);
        let mut second = first.clone();
        second.voice.style_id = 3001;
        let mut calls = 0;
        for score in [&first, &second, &first] {
            load_or_sing(root.path(), score, 48000, |_, rate| {
                calls += 1;
                Ok(wav(rate))
            })
            .unwrap();
        }
        assert_eq!(calls, 2);
        second.voice.speaker_uuid = "different-engine-character".into();
        assert_ne!(key(&first, 48000), key(&second, 48000));
    }

    #[test]
    fn failures_and_wrong_rates_are_not_stored() {
        let root = tempfile::tempdir().unwrap();
        let error = load_or_sing(root.path(), &score(60), 48000, |_, _| Err("400".into()));
        assert_eq!(error, Err("400".into()));
        assert!(load_or_sing(root.path(), &score(60), 48000, |_, _| Ok(wav(24000))).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
