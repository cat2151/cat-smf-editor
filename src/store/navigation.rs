use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Navigation {
    pub last_mid: Option<PathBuf>,
    pub backup_tip_date: Option<String>,
    #[serde(skip)]
    pub backup_tip_visible: bool,
    #[serde(skip)]
    pub departure: Option<crate::view::files::Action>,
    #[serde(skip)]
    pub departure_error: Option<String>,
    #[serde(skip)]
    pub close_approved: bool,
}
