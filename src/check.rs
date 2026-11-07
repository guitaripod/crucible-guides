use crate::model::{Bundle, Kind};
use std::collections::{HashMap, HashSet};

/// The share of a show's episodes whose kind may change in one build before the build is refused.
pub const MAX_CHURN: f64 = 0.10;

/// Every reason a build may not be published. Empty means CI accepts it with no human review.
pub fn problems(
    current: &Bundle,
    previous: Option<&Bundle>,
    expected: &HashMap<String, u32>,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut slugs = HashSet::new();
    for show in &current.shows {
        if !slugs.insert(show.slug.clone()) {
            out.push(format!("{}: listed twice", show.slug));
        }
        if show.ids.tvdb.is_none() && show.ids.tmdb.is_none() && show.ids.imdb.is_none() {
            out.push(format!(
                "{}: no tvdb, tmdb or imdb id, so no library can match it",
                show.slug
            ));
        }
        let want = expected.get(&show.slug).copied().unwrap_or(0);
        let numbers: HashSet<u32> = show.episodes.iter().map(|e| e.n).collect();
        let missing: Vec<u32> = (1..=want).filter(|n| !numbers.contains(n)).collect();
        if !missing.is_empty() {
            out.push(format!(
                "{}: {} of {} episodes missing ({:?}…)",
                show.slug,
                missing.len(),
                want,
                &missing[..missing.len().min(8)]
            ));
        }
        for e in &show.episodes {
            if e.kind.hides() {
                let signals: HashSet<&str> = e.evidence.iter().map(|x| x.signal.as_str()).collect();
                if signals.len() < 2 {
                    out.push(format!(
                        "{} {}: hidden kind {:?} with {} signal(s)",
                        show.slug,
                        e.n,
                        e.kind,
                        signals.len()
                    ));
                }
            }
        }
        for r in &show.ranges {
            if r.from == 0 || r.to < r.from || r.to > want.max(1) || r.quote.trim().is_empty() {
                out.push(format!(
                    "{}: range {}-{} is out of bounds or has no quote",
                    show.slug, r.from, r.to
                ));
            }
        }
        let hidden = show.episodes.iter().filter(|e| e.kind.hides()).count();
        if want > 0 && hidden as f64 > want as f64 * 0.6 {
            out.push(format!(
                "{}: {} of {} episodes would be hidden, which no real show has",
                show.slug, hidden, want
            ));
        }
    }
    if let Some(previous) = previous {
        for old in &previous.shows {
            let Some(new) = current.shows.iter().find(|s| s.slug == old.slug) else {
                out.push(format!("{}: disappeared from the bundle", old.slug));
                continue;
            };
            let before: HashMap<u32, Kind> = old.episodes.iter().map(|e| (e.n, e.kind)).collect();
            let changed = new
                .episodes
                .iter()
                .filter(|e| before.get(&e.n).is_some_and(|k| *k != e.kind))
                .count();
            let total = new.episodes.len().max(1);
            if changed as f64 > total as f64 * MAX_CHURN {
                out.push(format!(
                    "{}: {} of {} episodes changed kind in one build",
                    new.slug, changed, total
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn show(kinds: &[Kind]) -> Show {
        Show {
            slug: "s".into(),
            title: "S".into(),
            titles: vec![],
            ids: Ids {
                tvdb: Some(1),
                ..Default::default()
            },
            episodes: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| Episode {
                    n: i as u32 + 1,
                    aired: None,
                    title: String::new(),
                    kind: *k,
                    evidence: if k.hides() {
                        vec![
                            Evidence {
                                source: "w".into(),
                                signal: "a".into(),
                                page: "p".into(),
                                revision: 1,
                            },
                            Evidence {
                                source: "w".into(),
                                signal: "b".into(),
                                page: "p".into(),
                                revision: 1,
                            },
                        ]
                    } else {
                        vec![]
                    },
                })
                .collect(),
            ranges: vec![],
            arcs: vec![],
            sources: vec![],
        }
    }

    fn bundle(s: Show) -> Bundle {
        Bundle {
            format: FORMAT,
            generated: "x".into(),
            licence: "CC BY-SA 4.0".into(),
            shows: vec![s],
        }
    }

    #[test]
    fn accepts_a_sound_build() {
        let b = bundle(show(&[Kind::Canon, Kind::AnimeOnly, Kind::Canon]));
        assert!(problems(&b, Some(&b), &HashMap::from([("s".into(), 3)])).is_empty());
    }

    #[test]
    fn refuses_one_signal_and_missing_episodes_and_churn() {
        let mut s = show(&[Kind::Canon, Kind::AnimeOnly, Kind::Canon]);
        s.episodes[1].evidence.pop();
        let b = bundle(s);
        let p = problems(&b, None, &HashMap::from([("s".into(), 4)]));
        assert!(p.iter().any(|x| x.contains("1 signal")));
        assert!(p.iter().any(|x| x.contains("missing")));
        let old = bundle(show(&[Kind::Canon; 10]));
        let new = bundle(show(&[
            Kind::AnimeOnly,
            Kind::AnimeOnly,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
            Kind::Canon,
        ]));
        assert!(
            problems(&new, Some(&old), &HashMap::from([("s".into(), 10)]))
                .iter()
                .any(|x| x.contains("changed kind"))
        );
    }
}
