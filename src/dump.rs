use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Page {
    pub title: String,
    pub namespace: i64,
    pub revision: u64,
    pub text: String,
}

/// Fandom publishes each wiki's current pages as `pages_current.xml.7z` on S3 for reuse under the
/// wiki's licence. Reading the dump, never the live site, keeps the builder off Fandom's services,
/// whose terms bar automated access.
pub fn dump_url(wiki: &str) -> String {
    let a = &wiki[..1];
    let ab = &wiki[..2.min(wiki.len())];
    format!("https://s3.amazonaws.com/wikia_xml_dumps/{a}/{ab}/{wiki}_pages_current.xml.7z")
}

pub struct Fetched {
    pub path: PathBuf,
    pub last_modified: Option<String>,
}

/// Downloads the wiki's dump into `cache` unless the copy there is as new as the published one.
pub fn fetch(wiki: &str, cache: &Path) -> Result<Fetched> {
    fs::create_dir_all(cache)?;
    let path = cache.join(format!("{wiki}_pages_current.xml.7z"));
    let stamp = cache.join(format!("{wiki}.last-modified"));
    let agent = ureq::AgentBuilder::new().redirects(0).build();
    let mut request = agent.get(&dump_url(wiki)).set(
        "User-Agent",
        concat!(
            "crucible-guides/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/guitaripod/crucible-guides)"
        ),
    );
    let known = fs::read_to_string(&stamp).ok();
    if path.exists() {
        if let Some(known) = &known {
            request = request.set("If-Modified-Since", known.trim());
        }
    }
    match request.call() {
        Ok(response) => {
            let last_modified = response.header("Last-Modified").map(str::to_string);
            let mut body = response.into_reader().take(2_000_000_000);
            let tmp = path.with_extension("part");
            let mut out = fs::File::create(&tmp)?;
            std::io::copy(&mut body, &mut out)?;
            fs::rename(&tmp, &path)?;
            if let Some(lm) = &last_modified {
                fs::write(&stamp, lm)?;
            }
            Ok(Fetched {
                path,
                last_modified,
            })
        }
        Err(ureq::Error::Status(304, _)) => Ok(Fetched {
            path,
            last_modified: known.map(|s| s.trim().to_string()),
        }),
        Err(e) => Err(e).with_context(|| format!("downloading the {wiki} dump")),
    }
}

/// Calls `visit` for every page in a `.7z` or plain `.xml` dump, streaming, so a 500 MB dump never sits in memory.
pub fn pages(path: &Path, mut visit: impl FnMut(Page)) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()) == Some("xml") {
        let file = fs::File::open(path)?;
        return parse(BufReader::with_capacity(1 << 20, file), &mut visit);
    }
    let mut child = Command::new("bsdtar")
        .arg("-xOf")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("bsdtar is needed to read .7z dumps")?;
    let stdout = child.stdout.take().unwrap();
    parse(BufReader::with_capacity(1 << 20, stdout), &mut visit)?;
    let status = child.wait()?;
    if !status.success() {
        bail!("bsdtar failed on {}", path.display());
    }
    Ok(())
}

fn parse<R: BufRead>(input: R, visit: &mut impl FnMut(Page)) -> Result<()> {
    let mut reader = Reader::from_reader(input);
    let mut buf = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let (mut title, mut ns, mut rev, mut text) = (String::new(), 0i64, 0u64, String::new());
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                if name == "page" {
                    title.clear();
                    text.clear();
                    ns = 0;
                    rev = 0;
                }
                path.push(name);
            }
            Event::End(_) => {
                if path.pop().as_deref() == Some("page") {
                    visit(Page {
                        title: std::mem::take(&mut title),
                        namespace: ns,
                        revision: rev,
                        text: std::mem::take(&mut text),
                    });
                }
            }
            Event::Text(t) => {
                let value = t.unescape()?.into_owned();
                match (
                    path.iter().rev().nth(1).map(String::as_str),
                    path.last().map(String::as_str),
                ) {
                    (Some("page"), Some("title")) => title.push_str(&value),
                    (Some("page"), Some("ns")) => ns = value.trim().parse().unwrap_or(0),
                    (Some("revision"), Some("id")) => rev = value.trim().parse().unwrap_or(0),
                    (Some("revision"), Some("text")) => text.push_str(&value),
                    _ => {}
                }
            }
            Event::CData(t) => {
                if path.last().map(String::as_str) == Some("text") {
                    text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pages_from_xml() {
        let xml = r#"<mediawiki><siteinfo><sitename>W</sitename></siteinfo>
<page><title>A &amp; B</title><ns>0</ns><id>1</id><revision><id>77</id><text xml:space="preserve">{{X
|Number = 1
}}</text></revision></page>
<page><title>Talk:C</title><ns>1</ns><revision><id>78</id><text>t</text></revision></page></mediawiki>"#;
        let mut seen = Vec::new();
        parse(BufReader::new(xml.as_bytes()), &mut |p: Page| {
            seen.push((p.title, p.namespace, p.revision, p.text))
        })
        .unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].0, "A & B");
        assert_eq!(seen[0].2, 77);
        assert!(seen[0].3.contains("|Number = 1"));
        assert_eq!(seen[1].1, 1);
    }

    #[test]
    fn dump_urls() {
        assert_eq!(
            dump_url("dragonball"),
            "https://s3.amazonaws.com/wikia_xml_dumps/d/dr/dragonball_pages_current.xml.7z"
        );
    }
}
