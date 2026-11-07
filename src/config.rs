use crate::model::{Arc, Ids, Range};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ShowConfig {
    pub slug: String,
    pub title: String,
    pub titles: Vec<String>,
    pub ids: Ids,
    pub episodes: u32,
    pub wiki: WikiConfig,
    #[serde(default)]
    pub ranges: Vec<Range>,
    #[serde(default)]
    pub arcs: Vec<Arc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WikiConfig {
    pub dump: String,
    pub name: String,
    pub url: String,
    pub licence: String,
    pub licence_url: String,
    pub template: String,
    pub series_field: String,
    pub series: Vec<String>,
    pub number_field: String,
    pub title_field: String,
    pub aired_field: String,
    pub chapter_field: String,
}

pub fn load_all(dir: &Path) -> Result<Vec<ShowConfig>> {
    let mut configs = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let config: ShowConfig =
            toml::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
        configs.push(config);
    }
    Ok(configs)
}
