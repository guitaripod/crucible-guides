use crate::config::ShowConfig;
use crate::dump::Page;
use crate::model::{Episode, Evidence, Kind, Show, Source};
use crate::wikitext;
use std::collections::BTreeMap;

/// One wiki page's reading for one episode, before the pages are folded into a show.
#[derive(Debug, Clone)]
pub struct Reading {
    pub n: u32,
    pub title: String,
    pub aired: Option<String>,
    pub page: String,
    pub revision: u64,
    pub no_chapter: bool,
    pub whole_statement: Option<String>,
    pub partial_statement: bool,
}

/// Reads one page if it is an episode of the configured series.
pub fn read(config: &ShowConfig, page: &Page) -> Option<Reading> {
    if page.namespace != 0 {
        return None;
    }
    let wiki = &config.wiki;
    let series = wikitext::field(&page.text, &wiki.template, &wiki.series_field)?;
    if !wiki.series.iter().any(|s| s == series) {
        return None;
    }
    let n: u32 = wikitext::plain(wikitext::field(
        &page.text,
        &wiki.template,
        &wiki.number_field,
    )?)
    .parse()
    .ok()?;
    if n == 0 || n > config.episodes {
        return None;
    }
    let title = wikitext::field(&page.text, &wiki.template, &wiki.title_field)
        .map(wikitext::plain)
        .unwrap_or_else(|| page.title.clone());
    Some(Reading {
        n,
        title,
        aired: wikitext::field(&page.text, &wiki.template, &wiki.aired_field)
            .and_then(wikitext::iso_date),
        page: page.title.clone(),
        revision: page.revision,
        no_chapter: wikitext::names_no_chapter(wikitext::field(
            &page.text,
            &wiki.template,
            &wiki.chapter_field,
        )),
        whole_statement: wikitext::whole_episode_statement(&page.text),
        partial_statement: wikitext::partial_statement(&page.text),
    })
}

/// The kind two independent signals support. Hiding kinds need both the chapter field and the
/// trivia statement, which are separate edits of the page, so one vandal edit hides nothing.
pub fn kind(reading: &Reading, source: &str) -> (Kind, Vec<Evidence>) {
    let mut evidence = Vec::new();
    if reading.no_chapter {
        evidence.push(Evidence {
            source: source.to_string(),
            signal: "no adapted chapter".to_string(),
            page: reading.page.clone(),
            revision: reading.revision,
        });
    }
    if let Some(statement) = &reading.whole_statement {
        evidence.push(Evidence {
            source: source.to_string(),
            signal: statement.clone(),
            page: reading.page.clone(),
            revision: reading.revision,
        });
    }
    let kind = match (
        reading.no_chapter,
        reading.whole_statement.is_some(),
        reading.partial_statement,
    ) {
        (true, true, _) => Kind::AnimeOnly,
        (true, false, _) => Kind::Unknown,
        (false, true, _) | (false, false, true) => Kind::Mixed,
        (false, false, false) => Kind::Canon,
    };
    if !kind.hides() {
        evidence.clear();
    }
    (kind, evidence)
}

/// Folds every reading of a show into its guide. When two pages claim one number (a recut or a
/// duplicate), the one with the newer revision wins.
pub fn show(config: &ShowConfig, readings: &[Reading], dump_date: Option<String>) -> Show {
    let mut by_number: BTreeMap<u32, &Reading> = BTreeMap::new();
    for r in readings {
        let keep = by_number
            .get(&r.n)
            .is_none_or(|existing| r.revision > existing.revision);
        if keep {
            by_number.insert(r.n, r);
        }
    }
    let source_id = config.wiki.dump.clone();
    let episodes = by_number
        .values()
        .map(|r| {
            let (kind, evidence) = kind(r, &source_id);
            Episode {
                n: r.n,
                aired: r.aired.clone(),
                title: r.title.clone(),
                kind,
                evidence,
            }
        })
        .collect();
    let mut sources = vec![Source {
        id: source_id,
        name: config.wiki.name.clone(),
        licence: config.wiki.licence.clone(),
        licence_url: config.wiki.licence_url.clone(),
        url: config.wiki.url.clone(),
        dump_date,
    }];
    for range in &config.ranges {
        if !sources.iter().any(|s| s.id == range.source) {
            sources.push(Source {
                id: range.source.clone(),
                name: "Wikipedia".to_string(),
                licence: "CC BY-SA 4.0".to_string(),
                licence_url: "https://creativecommons.org/licenses/by-sa/4.0/".to_string(),
                url: range.url.clone(),
                dump_date: None,
            });
        }
    }
    Show {
        slug: config.slug.clone(),
        title: config.title.clone(),
        titles: config.titles.clone(),
        ids: config.ids.clone(),
        episodes,
        ranges: config.ranges.clone(),
        arcs: config.arcs.clone(),
        sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(no_chapter: bool, whole: bool, partial: bool) -> Reading {
        Reading {
            n: 1,
            title: "t".into(),
            aired: None,
            page: "p".into(),
            revision: 1,
            no_chapter,
            whole_statement: whole.then(|| "entire episode exclusive to the anime".to_string()),
            partial_statement: partial,
        }
    }

    #[test]
    fn hiding_needs_both_signals() {
        assert_eq!(kind(&reading(true, true, false), "w").0, Kind::AnimeOnly);
        assert_eq!(kind(&reading(true, true, false), "w").1.len(), 2);
        assert_eq!(kind(&reading(true, false, false), "w").0, Kind::Unknown);
        assert_eq!(kind(&reading(false, true, false), "w").0, Kind::Mixed);
        assert_eq!(kind(&reading(false, false, true), "w").0, Kind::Mixed);
        assert_eq!(kind(&reading(false, false, false), "w").0, Kind::Canon);
        assert!(kind(&reading(true, false, false), "w").1.is_empty());
    }
}
