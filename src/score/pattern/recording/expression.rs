//! Velocity of each press and CC1 over time, as cat-plugin-player plays its sequences.
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Velocity {
    V100,
    V127,
    /// Presses sweep 40 -> 100 over [`Velocity::STEPS`] presses, then back down.
    Range40To100,
    Range80To127,
}

impl Velocity {
    pub const ALL: [Self; 4] = [
        Self::V100,
        Self::V127,
        Self::Range40To100,
        Self::Range80To127,
    ];
    /// Presses per sweep up, and then per sweep down.
    pub const STEPS: usize = 8;

    pub fn label(self) -> &'static str {
        match self {
            Self::V100 => "100",
            Self::V127 => "127",
            Self::Range40To100 => "40-100",
            Self::Range80To127 => "80-127",
        }
    }

    /// The velocity of the press after `presses` earlier ones.
    pub fn of_press(self, presses: usize) -> u8 {
        let (low, high) = match self {
            Self::V100 => return 100,
            Self::V127 => return 127,
            Self::Range40To100 => (40, 100),
            Self::Range80To127 => (80, 127),
        };
        let index = presses % Self::STEPS;
        let offset = ((high - low) as usize * index / (Self::STEPS - 1)) as u8;
        if (presses / Self::STEPS) % 2 == 1 {
            high - offset
        } else {
            low + offset
        }
    }
}

/// CC1 of the keyboard, a function of the time since the sweep started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modulation {
    /// Nothing is sent, so the patch keeps its own modulation wheel.
    Off,
    Fixed(u8),
    /// 0 -> 127 in 2 seconds, then 127 -> 0 in 2 seconds; repeat.
    Sweep,
}

const HALF_PERIOD_MICROS: u128 = 2_000_000;

impl Modulation {
    pub const ALL: [Self; 4] = [Self::Off, Self::Fixed(0), Self::Fixed(127), Self::Sweep];

    pub fn label(self) -> String {
        match self {
            Self::Off => "-".into(),
            Self::Fixed(value) => value.to_string(),
            Self::Sweep => "sweep".into(),
        }
    }

    /// The value `at` after the sweep started; `None` sends nothing.
    pub fn value(self, at: Duration) -> Option<u8> {
        match self {
            Self::Off => None,
            Self::Fixed(value) => Some(value.min(127)),
            Self::Sweep => Some(sweep(step(at))),
        }
    }

    /// The value at `from`, then each change before `to`, as (time, value), of a sweep started
    /// at `zero`; before `zero` the sweep holds its first value.
    pub fn changes(self, zero: Duration, from: Duration, to: Duration) -> Vec<(Duration, u8)> {
        let Some(first) = self.value(from.saturating_sub(zero)) else {
            return vec![];
        };
        let mut out = vec![(from, first)];
        if self == Self::Sweep {
            let mut k = step(from.saturating_sub(zero)) + 1;
            loop {
                let at = zero
                    + Duration::from_micros(
                        (k * HALF_PERIOD_MICROS).div_ceil(127).min(u64::MAX as u128) as u64,
                    );
                if at >= to {
                    break;
                }
                out.push((at, sweep(k)));
                k += 1;
            }
        }
        out
    }
}

/// Sweep steps from its start; one step per value.
fn step(at: Duration) -> u128 {
    at.as_micros() * 127 / HALF_PERIOD_MICROS
}

fn sweep(step: u128) -> u8 {
    let phase = (step % 254) as u8;
    if phase <= 127 {
        phase
    } else {
        254 - phase
    }
}

#[cfg(test)]
#[path = "expression_tests.rs"]
mod tests;
