use regex::Regex;
use std::sync::LazyLock;

static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]").unwrap());
static TEMPLATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[^{}]*\}\}").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static WHOLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:\bthe\s+)?(?:entire(?:ty)?(?:\s+events)?\s+of\s+(?:this|the)\s+episode(?:'s\s+events)?|entire\s+episode|whole\s+episode)[^.\n]*\b(?:exclusive|original|unique)\s+to\s+the\s+anime",
    )
    .unwrap()
});
static EXCEPTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)with\s+the\s+exception|except\s+for|apart\s+from").unwrap());
static PARTIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:exclusive|original|unique)\s+to\s+the\s+anime\b|\bnot\s+(?:in|from)\s+the\s+manga\b").unwrap()
});

/// The value of `|name = value` inside the first `{{template` block of a page, with the line's
/// own wiki markup left in place. Infobox values in these wikis sit on one line each.
pub fn field<'a>(text: &'a str, template: &str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("{{{{{template}"))?;
    let body = &text[start..];
    let end = infobox_end(body);
    for line in body[..end].lines() {
        let Some(rest) = line.trim_start().strip_prefix('|') else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            continue;
        };
        if key.trim() == name {
            return Some(value.trim());
        }
    }
    None
}

/// Where an infobox that begins at the start of `body` closes, counting nested `{{ }}`.
fn infobox_end(body: &str) -> usize {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            depth += 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
            continue;
        }
        i += 1;
    }
    body.len()
}

/// Plain text of a wiki value: links reduced to their label, templates and tags removed.
pub fn plain(value: &str) -> String {
    let mut s = value.to_string();
    for _ in 0..4 {
        let next = TEMPLATE.replace_all(&s, "").into_owned();
        if next == s {
            break;
        }
        s = next;
    }
    let s = LINK.replace_all(&s, "$1");
    let s = TAG.replace_all(&s, "");
    s.replace("'''", "").replace("''", "").trim().to_string()
}

/// Whether an adapted-chapter field names no chapter: empty, "None", "N/A", a dash, or "None (filler)".
pub fn names_no_chapter(value: Option<&str>) -> bool {
    let Some(value) = value else { return true };
    let text = plain(value).to_lowercase();
    let text = text.trim();
    text.is_empty()
        || text.starts_with("none")
        || text.starts_with("n/a")
        || text == "-"
        || text == "—"
        || text == "filler"
}

/// The trivia line that says the whole episode is anime-only, unless the same line carves out an exception.
pub fn whole_episode_statement(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(m) = WHOLE.find(line) {
            if EXCEPTION.is_match(line) {
                continue;
            }
            return Some(plain(m.as_str()));
        }
    }
    None
}

/// Whether any line says part of the episode is anime-only.
pub fn partial_statement(text: &str) -> bool {
    text.lines().any(|line| PARTIAL.is_match(line))
}

/// "August 2, 2015" or "2015-08-02" or "2015/08/02" as an ISO date.
pub fn iso_date(value: &str) -> Option<String> {
    let text = plain(value);
    let months = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let lower = text.to_lowercase();
    let numeric = Regex::new(r"(\d{4})[-/](\d{1,2})[-/](\d{1,2})").unwrap();
    if let Some(c) = numeric.captures(&lower) {
        return Some(format!(
            "{}-{:02}-{:02}",
            &c[1],
            c[2].parse::<u32>().ok()?,
            c[3].parse::<u32>().ok()?
        ));
    }
    let named = Regex::new(r"([a-z]+)\s+(\d{1,2}),\s*(\d{4})").unwrap();
    let c = named.captures(&lower)?;
    let month = months.iter().position(|m| *m == &c[1])? + 1;
    Some(format!(
        "{}-{:02}-{:02}",
        &c[3],
        month,
        c[2].parse::<u32>().ok()?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "{{EpisodeInfobox\n|Name = A Chaotic Victory Party!\n|Series = Dragon Ball Super\n|Number = 42\n|Manga =\n|Airdate = May 8, 2016\n|Note = {{nested|x}}\n}}\nText.\n==Trivia==\n*The entire events of this episode were exclusive to the anime.\n";

    #[test]
    fn reads_fields_inside_the_infobox_only() {
        assert_eq!(field(PAGE, "EpisodeInfobox", "Number"), Some("42"));
        assert_eq!(field(PAGE, "EpisodeInfobox", "Manga"), Some(""));
        assert_eq!(
            field(
                "{{EpisodeInfobox\n|A = 1\n}}\n|B = 2\n",
                "EpisodeInfobox",
                "B"
            ),
            None
        );
    }

    #[test]
    fn chapter_field_variants() {
        assert!(names_no_chapter(Some("")));
        assert!(names_no_chapter(Some("None ([[filler]])")));
        assert!(names_no_chapter(None));
        assert!(!names_no_chapter(Some("[[Life, Training and More]]")));
    }

    #[test]
    fn whole_episode_statement_ignores_exceptions() {
        assert!(whole_episode_statement(PAGE).is_some());
        let carved = "*With the exception of the final scene, the entire events of this episode were exclusive to the anime.";
        assert!(whole_episode_statement(carved).is_none());
        assert!(partial_statement(carved));
        assert!(whole_episode_statement("*Goku's training is exclusive to the anime.").is_none());
        let entirety = "*The entirety of this episode's events is exclusive to the anime with the only similarity being X.";
        assert!(whole_episode_statement(entirety).is_some());
    }

    #[test]
    fn dates() {
        assert_eq!(iso_date("May 8, 2016").as_deref(), Some("2016-05-08"));
        assert_eq!(iso_date("2002/10/03").as_deref(), Some("2002-10-03"));
        assert_eq!(iso_date("[[July 5]], 2015").as_deref(), Some("2015-07-05"));
    }

    #[test]
    fn plain_text() {
        assert_eq!(
            plain("[[The Return of Vegeta|Return]] {{ruby|x|y}}"),
            "Return"
        );
    }
}
