use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub fx_rates: PathBuf,
    pub initial_position: PathBuf,
    pub zwischenergebnisse: PathBuf,
    pub jährliche_daten: HashMap<u32, YearlySettings>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YearlySettings {
    pub kontoauszug: PathBuf,
    pub cashbericht: PathBuf,
}
