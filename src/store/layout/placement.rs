//! Percentages describe the outer window, including its frame, within a monitor's work area.
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum Monitor {
    Position(Position),
    Number(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Position {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MonitorPlacement {
    pub monitor: Monitor,
    pub left_percent: f32,
    pub top_percent: f32,
    pub width_percent: f32,
    pub height_percent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Placement {
    Desktop([f32; 4]),
    Monitor(MonitorPlacement),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Screen {
    pub number: u32,
    pub bounds: [i32; 4],
    pub work: [i32; 4],
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Resolved {
    pub outer: [i32; 4],
    pub scale: f32,
}

impl MonitorPlacement {
    pub(super) fn validate(&self) -> Result<(), String> {
        let values = [
            self.left_percent,
            self.top_percent,
            self.width_percent,
            self.height_percent,
        ];
        if !values.iter().all(|n| n.is_finite() && *n >= 0.0)
            || self.width_percent <= 0.0
            || self.height_percent <= 0.0
            || self.left_percent + self.width_percent > 100.0
            || self.top_percent + self.height_percent > 100.0
        {
            return Err("monitor_placement requires positive size and percentages within the work area (0..100)".into());
        }
        if self.monitor == Monitor::Number(0) {
            return Err("monitor numbers start at 1".into());
        }
        Ok(())
    }

    pub(crate) fn resolve(&self, screens: &[Screen]) -> Result<Resolved, String> {
        let screen = match self.monitor {
            Monitor::Number(number) => screens
                .iter()
                .find(|s| s.number == number)
                .ok_or_else(|| format!("monitor {number} is not connected"))?,
            Monitor::Position(position) => {
                let mut ordered: Vec<_> = screens.iter().collect();
                ordered.sort_by_key(|s| i64::from(s.bounds[0]) * 2 + i64::from(s.bounds[2]));
                if ordered.is_empty() {
                    return Err("no monitors are connected".into());
                }
                let index = match position {
                    Position::Left => 0,
                    Position::Right => ordered.len() - 1,
                    Position::Center if ordered.len() % 2 == 1 => ordered.len() / 2,
                    Position::Center => return Err(
                        "monitor center is ambiguous with an even number of monitors; use a number"
                            .into(),
                    ),
                };
                let center = |s: &Screen| i64::from(s.bounds[0]) * 2 + i64::from(s.bounds[2]);
                let selected = ordered[index];
                if ordered
                    .iter()
                    .filter(|s| center(s) == center(selected))
                    .count()
                    != 1
                {
                    return Err(
                        "monitor position is ambiguous in this arrangement; use a number".into(),
                    );
                }
                selected
            }
        };
        let [x, y, w, h] = screen.work;
        let edge = |origin: i32, size: i32, percent: f32| {
            origin + (size as f64 * f64::from(percent) / 100.0).round() as i32
        };
        let left = edge(x, w, self.left_percent);
        let top = edge(y, h, self.top_percent);
        let right = edge(x, w, self.left_percent + self.width_percent);
        let bottom = edge(y, h, self.top_percent + self.height_percent);
        if right <= left || bottom <= top {
            return Err("monitor_placement is smaller than one physical pixel".into());
        }
        Ok(Resolved {
            outer: [left, top, right - left, bottom - top],
            scale: screen.scale,
        })
    }
}

#[cfg(test)]
mod tests;
