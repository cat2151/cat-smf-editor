//! engine が返す 16bit PCM WAV を f32 の stereo sample 列へ読む。

/// 16bit PCM・stereo の WAV を、sample rate とインターリーブ済み f32 sample にする。
pub fn decode(bytes: &[u8]) -> Result<(u32, Vec<f32>), String> {
    let invalid = |why: &str| format!("VOICEVOX WAV: {why}");
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err(invalid("not RIFF/WAVE"));
    }
    let u16_at = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut at = 12;
    let mut rate = None;
    while let (Some(id), Some(size)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at + 8;
        let size = size as usize;
        match id {
            b"fmt " => {
                if u16_at(body) != Some(1)
                    || u16_at(body + 2) != Some(2)
                    || u16_at(body + 14) != Some(16)
                {
                    return Err(invalid("expected 16-bit PCM stereo"));
                }
                rate = u32_at(body + 4);
            }
            b"data" => {
                let rate = rate.ok_or_else(|| invalid("data before fmt"))?;
                let data = bytes
                    .get(body..body + size)
                    .ok_or_else(|| invalid("truncated data"))?;
                let samples = data
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| f32::from(i16::from_le_bytes(*b)) / 32768.0)
                    .collect();
                return Ok((rate, samples));
            }
            _ => {}
        }
        at = body + size + size % 2;
    }
    Err(invalid("no data chunk"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(format: u16, channels: u16, bits: u16, rate: u32, data: &[i16]) -> Vec<u8> {
        let mut b = Vec::new();
        let size = (data.len() * 2) as u32;
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + size).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&format.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        b.extend_from_slice(&(channels * 2).to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&size.to_le_bytes());
        for s in data {
            b.extend_from_slice(&s.to_le_bytes());
        }
        b
    }

    #[test]
    fn stereo_pcm16_becomes_unit_range_floats() {
        let bytes = wav(1, 2, 16, 48000, &[0, 16384, -32768, 32767]);
        let (rate, samples) = decode(&bytes).unwrap();
        assert_eq!(rate, 48000);
        assert_eq!(samples, vec![0.0, 0.5, -1.0, 32767.0 / 32768.0]);
    }

    #[test]
    fn other_layouts_are_rejected() {
        assert!(decode(&wav(1, 1, 16, 48000, &[0])).is_err());
        assert!(decode(&wav(3, 2, 16, 48000, &[0, 0])).is_err());
        assert!(decode(b"RIFF....WAVE").is_err());
    }
}
