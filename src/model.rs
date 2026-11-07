use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub format: u32,
    pub generated: String,
    pub licence: String,
    pub shows: Vec<Show>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub slug: String,
    pub title: String,
    pub titles: Vec<String>,
    pub ids: Ids,
    pub episodes: Vec<Episode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranges: Vec<Range>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arcs: Vec<Arc>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ids {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tvdb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tmdb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imdb: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anidb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mal: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Canon,
    Mixed,
    AnimeOnly,
    Filler,
    Recap,
    Unknown,
}

impl Kind {
    /// Kinds that some Essentials level hides. Each needs two independent signals.
    pub fn hides(self) -> bool {
        matches!(self, Kind::AnimeOnly | Kind::Filler | Kind::Recap)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub n: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
    pub title: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub source: String,
    pub signal: String,
    pub page: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub from: u32,
    pub to: u32,
    pub retells: Vec<String>,
    pub source: String,
    pub url: String,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Arc {
    pub name: String,
    pub from: u32,
    pub to: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    pub licence: String,
    pub licence_url: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dump_date: Option<String>,
}
