use super::*;
use crate::play::audio_sequence::{Note, Sequence};
use std::sync::Arc;
#[test]
fn volume_scales_audio_and_mute_leaves_other_tracks_unchanged() {
    let mut mixed = [0.0; 2];
    mix_into(&mut mixed, &[0.8, -0.4], 0.25);
    assert_eq!(mixed, [0.2, -0.1]);
    mix_into(&mut mixed, &[0.9, 0.9], 0.0);
    assert_eq!(mixed, [0.2, -0.1]);
    mix_into(&mut mixed, &[0.3, 0.3], 1.0);
    assert_eq!(mixed[0], 0.5);
    assert!((mixed[1] - 0.2).abs() < 1e-6);
    assert!(!mix_into(&mut mixed, &[f32::MAX, f32::MAX], 2.0));
    assert_eq!(mixed, [0.0, 0.0]);
}
#[test]
fn each_part_splits_only_at_its_own_events_and_same_pitch_durations_stay_independent() {
    let parts = [0.1, 0.2].map(|start| Part {
        sequence: Some(Arc::new(Sequence::new(
            vec![Note {
                start,
                duration: start,
                pitch: 60,
                velocity: 100,
            }],
            start * 2.0,
        ))),
        ..Part::default()
    });
    let mut starts = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        let mut state = State::default();
        let mut part_starts = Vec::new();
        while state.cursor.frame <= 400 {
            let frame = state.cursor.frame;
            part_starts.push(frame);
            let frames = boundary(&state, part, 1000, 1024);
            let (actual, changed) = state.cursor.block(
                part.sequence.as_ref().unwrap(),
                1000,
                frames,
                &mut state.velocities,
                &mut state.durations,
            );
            assert_eq!(actual, frames);
            if frame == 100 * (index as u64 + 1) {
                assert_eq!(changed, 2);
                assert_eq!(state.durations[60], 100 * (index as u64 + 1));
            }
        }
        starts.push(part_starts);
    }
    assert_eq!(starts, [vec![0, 100, 200], vec![0, 200, 400]]);
}
