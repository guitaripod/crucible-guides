# Crucible Guides

**Which episodes of a long anime adapt the manga, and which the anime made up.** An open dataset, rebuilt every week from published fan-wiki dumps by checks that run without anyone in the loop, and the GPL tool that builds it.

[Crucible](https://apps.apple.com/app/id6822066419), the Plex app for iPhone, iPad and Apple TV, reads it to offer **Essentials**: watch Dragon Ball Super as 110 episodes instead of 131, or 83 if you have seen the two films it opens by retelling. With Crucible's chapter skips that is **26 hours instead of 50**.

| Show | Episodes | Anime-only (hidden by Skip Anime-Only) | Notes |
|---|---|---|---|
| Dragon Ball | 153 | 20 | |
| Dragon Ball Z | 291 | 30 | |
| Dragon Ball Super | 131 | 21 | Episodes 1-27 retell *Battle of Gods* and *Resurrection 'F'* (Wikipedia); "I've seen the films" hides them too: 83 left |

## How an episode is called anime-only

Two independent signals must agree, both from the show's wiki:

1. the episode infobox names **no adapted manga chapter** (empty, "None" or "None (filler)"), and
2. the page's trivia says **the entire episode is exclusive to the anime**, with no exception carved out on that line.

One signal alone makes an episode `unknown`, and Crucible shows it. A single edit to a wiki therefore hides nothing. A partial statement ("the scene where … is exclusive to the anime") makes it `mixed`, also shown. Every hidden episode carries both signals, the page and the revision they were read from.

Episodes inside a range a source says retells something (Dragon Ball Super 1-27) are only hidden when the viewer says they have seen it.

## Checks, instead of reviewers

A build is published only when every check passes: every episode of every show present, two signals behind every hidden episode, ranges inside the show and quoted, no show losing its ids, no show disappearing, and no show changing the kind of more than 10% of its episodes in one week. A failing build is not published; the last good one stays live.

## Sources and licence

- **Data** (`data/guides.json`): [CC BY-SA 4.0](data/LICENSE). Adapted from the [Dragon Ball Wiki](https://dragonball.fandom.com/) (CC BY-SA, [Fandom licensing](https://www.fandom.com/licensing)) and [Wikipedia](https://en.wikipedia.org/) (CC BY-SA 4.0). Each show lists its sources, the dump date and each page's revision.
- **Code**: [GPL-3.0](LICENSE).
- The builder reads each wiki's **published database dump** (`pages_current.xml.7z`, which Fandom hosts for reuse), never the live site: Fandom's terms bar automated access to its services. Episode titles and air dates come from the same dumps.

## Using it

```bash
cargo build --release
./target/release/guides build --fetch --cache dumps --generated 2026-11-07
./target/release/guides check --previous data/guides.json
./target/release/guides summary
```

Crucible downloads `https://api.midgarcorp.cc/v1/crucible/guides`: the bundle inside an envelope signed with Ed25519 over `crucible-guides\n{format}\n{keyId}\n{sequence}\n{notAfter}\n` followed by the bundle's bytes. The public keys are in [`keys/`](keys); verify with `guides verify --public "$(cat keys/cg1.pub)"`. Apps refuse an unknown key, a sequence that does not increase, and an expired envelope.

## Adding a show

A show is one TOML file in [`shows/`](shows): its ids (tvdb, imdb, anidb, mal) so a Plex library can match it, its wiki dump and template fields, and any stated ranges. Open a pull request; CI builds it and runs the checks. A show joins only where its wiki carries both signals: Naruto, One Piece and Fairy Tail's wikis carry the chapter field but no whole-episode statement yet, so they wait for a second signal.
