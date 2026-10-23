//! Reference-only plugin automation. Runtime indices and patch states never persist.
mod metadata;
use super::scalar_curve::{self, ScalarPoint};
pub use metadata::{
    create, group_event, owned, read, write, Collection, Group, GROUP_PREFIX, PREFIX,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub group: u64,
    pub layer: u64,
    pub plugin_format: String,
    pub plugin_id: String,
    pub parameter_id: String,
}

pub use crate::host::parameters::domain::ValueSpace;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub tick: u64,
    pub value: f64,
}
impl ScalarPoint for Point {
    fn tick(self) -> u64 {
        self.tick
    }
    fn value(self) -> f64 {
        self.value
    }
    fn new(tick: u64, value: f64) -> Self {
        Self { tick, value }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub version: u8,
    pub target: Target,
    pub value_space: ValueSpace,
    pub minimum: f64,
    pub maximum: f64,
    pub name: String,
    pub path: String,
    pub points: Vec<Point>,
    pub controls: Vec<Point>,
}

impl Curve {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.target.group == 0
            || self.target.layer == 0
            || self.target.plugin_id.is_empty()
            || self.target.parameter_id.is_empty()
        {
            return Err("Invalid parameter curve version or target identity".into());
        }
        match (&*self.target.plugin_format, self.value_space) {
            ("CLAP", ValueSpace::Plain) => (),
            ("VST3", ValueSpace::Normalized) if self.minimum == 0.0 && self.maximum == 1.0 => (),
            _ => return Err("Parameter format/value space mismatch".into()),
        }
        self.domain()?;
        if self.controls.len() != self.points.len().saturating_sub(1) {
            return Err("A parameter curve needs one control per segment".into());
        }
        if self.points.windows(2).any(|p| p[0].tick >= p[1].tick) {
            return Err("Parameter points must have unique increasing ticks".into());
        }
        if self
            .points
            .windows(2)
            .zip(&self.controls)
            .any(|(p, c)| !(p[0].tick..=p[1].tick).contains(&c.tick))
        {
            return Err("A parameter control must stay inside its segment".into());
        }
        if self
            .points
            .iter()
            .any(|p| !p.value.is_finite() || !(self.minimum..=self.maximum).contains(&p.value))
        {
            return Err("Parameter value is outside its native range".into());
        }
        if self.controls.iter().any(|p| !p.value.is_finite()) {
            return Err("Parameter control must be finite".into());
        }
        Ok(())
    }

    pub fn domain(&self) -> Result<crate::host::parameters::domain::Domain, String> {
        crate::host::parameters::domain::Domain::new(self.value_space, self.minimum, self.maximum)
    }

    pub fn value_at(&self, tick: f64) -> Option<f64> {
        scalar_curve::value_at(&self.points, &self.controls, tick)
            .map(|value| value.clamp(self.minimum, self.maximum))
    }

    pub fn add(&mut self, tick: u64, value: f64) -> Result<usize, String> {
        self.check_value(value)?;
        scalar_curve::insert(&mut self.points, &mut self.controls, Point { tick, value })
    }

    pub fn remove(&mut self, index: usize) -> Result<(), String> {
        scalar_curve::remove(&mut self.points, &mut self.controls, index)
    }

    pub fn move_point(&mut self, index: usize, tick: u64, value: f64) -> Result<(), String> {
        self.check_value(value)?;
        scalar_curve::move_point(&mut self.points, &mut self.controls, index, tick, value)
    }

    pub fn move_control(&mut self, index: usize, tick: u64, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Parameter control must be finite".into());
        }
        let Some(pair) = self.points.get(index..index.saturating_add(2)) else {
            return Err("No such parameter segment".into());
        };
        if pair.len() != 2 || index >= self.controls.len() {
            return Err("No such parameter control".into());
        }
        self.controls[index] = Point {
            tick: tick.clamp(pair[0].tick, pair[1].tick),
            value,
        };
        Ok(())
    }

    fn check_value(&self, value: f64) -> Result<(), String> {
        if value.is_finite() && (self.minimum..=self.maximum).contains(&value) {
            Ok(())
        } else {
            Err("Parameter value is outside its native range".into())
        }
    }
}

#[cfg(test)]
mod tests;
