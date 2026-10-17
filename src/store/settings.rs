use eframe::egui::ViewportBuilder;
use serde::Deserialize;
use std::{io::Write, path::Path};

const TEMPLATE: &str = include_str!("../../config/settings.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub window: [f32; 4],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window: [0.0, 0.0, 1800.0, 900.0],
        }
    }
}

impl Settings {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let path = directory.join("settings.toml");
        let result = (|| -> Result<Self, Box<dyn std::error::Error>> {
            std::fs::create_dir_all(directory)?;
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(TEMPLATE.as_bytes())?;
                    file.sync_all()?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
            let settings: Self = toml::from_str(&std::fs::read_to_string(&path)?)?;
            let [x, y, width, height] = settings.window;
            if ![x, y, width, height].iter().all(|n| n.is_finite()) || width <= 0.0 || height <= 0.0
            {
                return Err("window requires finite x/y and positive width/height".into());
            }
            Ok(settings)
        })();
        result.map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn viewport(&self) -> ViewportBuilder {
        let [x, y, width, height] = self.window;
        ViewportBuilder::default()
            .with_position([x, y])
            .with_inner_size([width, height])
    }
}

#[cfg(test)]
mod tests;
