pub mod check;
pub mod classify;
pub mod config;
pub mod dump;
pub mod model;
pub mod sign;
pub mod wikitext;

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Builds every configured show from dumps already in `cache` (or fetched into it), one pass per wiki.
pub fn build(
    shows_dir: &Path,
    cache: &Path,
    fetch: bool,
    generated: &str,
) -> Result<model::Bundle> {
    let configs = config::load_all(shows_dir)?;
    let mut by_wiki: HashMap<String, Vec<&config::ShowConfig>> = HashMap::new();
    for c in &configs {
        by_wiki.entry(c.wiki.dump.clone()).or_default().push(c);
    }
    let mut shows = Vec::new();
    let mut wikis: Vec<_> = by_wiki.keys().cloned().collect();
    wikis.sort();
    for wiki in wikis {
        let group = &by_wiki[&wiki];
        let (path, last_modified) = if fetch {
            let f = dump::fetch(&wiki, cache)?;
            (f.path, f.last_modified)
        } else {
            let xml = cache.join(format!("{wiki}_pages_current.xml"));
            let sz = cache.join(format!("{wiki}_pages_current.xml.7z"));
            let stamp = std::fs::read_to_string(cache.join(format!("{wiki}.last-modified")))
                .ok()
                .map(|s| s.trim().to_string());
            (if xml.exists() { xml } else { sz }, stamp)
        };
        let mut readings: Vec<Vec<classify::Reading>> = vec![Vec::new(); group.len()];
        dump::pages(&path, |page| {
            for (i, c) in group.iter().enumerate() {
                if let Some(r) = classify::read(c, &page) {
                    readings[i].push(r);
                }
            }
        })?;
        for (i, c) in group.iter().enumerate() {
            shows.push(classify::show(c, &readings[i], last_modified.clone()));
        }
    }
    shows.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(model::Bundle {
        format: model::FORMAT,
        generated: generated.to_string(),
        licence: "CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/); each show lists its sources".to_string(),
        shows,
    })
}

pub fn expected_counts(shows_dir: &Path) -> Result<HashMap<String, u32>> {
    Ok(config::load_all(shows_dir)?
        .into_iter()
        .map(|c| (c.slug, c.episodes))
        .collect())
}
