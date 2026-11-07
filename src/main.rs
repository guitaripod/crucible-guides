use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use crucible_guides::{build, check, expected_counts, model::Bundle, sign};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "guides",
    version,
    about = "Build, check and sign Crucible Guides"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read the wiki dumps and write the bundle.
    Build {
        #[arg(long, default_value = "shows")]
        shows: PathBuf,
        #[arg(long, default_value = "dumps")]
        cache: PathBuf,
        #[arg(long)]
        fetch: bool,
        #[arg(long, default_value = "data/guides.json")]
        out: PathBuf,
        #[arg(long)]
        generated: String,
    },
    /// Refuse a bundle that fails a machine check; compares with the previous bundle when given.
    Check {
        #[arg(long, default_value = "shows")]
        shows: PathBuf,
        #[arg(long, default_value = "data/guides.json")]
        bundle: PathBuf,
        #[arg(long)]
        previous: Option<PathBuf>,
    },
    /// Print a summary of each show's kinds.
    Summary {
        #[arg(long, default_value = "data/guides.json")]
        bundle: PathBuf,
    },
    /// Make a signing key: the secret goes to `out` (0600), the public key is printed.
    Keygen {
        #[arg(long)]
        out: PathBuf,
    },
    /// Seal the bundle into the envelope apps download.
    Sign {
        #[arg(long, default_value = "data/guides.json")]
        bundle: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        key_id: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        not_after: String,
        #[arg(long, default_value = "dist/guides.envelope.json")]
        out: PathBuf,
    },
    /// Verify an envelope against a public key.
    Verify {
        #[arg(long, default_value = "dist/guides.envelope.json")]
        envelope: PathBuf,
        #[arg(long)]
        public: String,
    },
}

fn read_bundle(path: &PathBuf) -> Result<Bundle> {
    serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .context("parsing the bundle")
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Build {
            shows,
            cache,
            fetch,
            out,
            generated,
        } => {
            let bundle = build(&shows, &cache, fetch, &generated)?;
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&bundle)? + "\n")?;
            println!("wrote {} with {} shows", out.display(), bundle.shows.len());
        }
        Command::Check {
            shows,
            bundle,
            previous,
        } => {
            let current = read_bundle(&bundle)?;
            let previous = previous.map(|p| read_bundle(&p)).transpose()?;
            let problems = check::problems(&current, previous.as_ref(), &expected_counts(&shows)?);
            if problems.is_empty() {
                println!("accepted: {} shows pass every check", current.shows.len());
            } else {
                for p in &problems {
                    eprintln!("refused: {p}");
                }
                bail!("{} problem(s)", problems.len());
            }
        }
        Command::Summary { bundle } => {
            let b = read_bundle(&bundle)?;
            for s in &b.shows {
                let mut counts = std::collections::BTreeMap::new();
                for e in &s.episodes {
                    *counts.entry(format!("{:?}", e.kind)).or_insert(0) += 1;
                }
                let in_range = |n: u32| s.ranges.iter().any(|r| (r.from..=r.to).contains(&n));
                let hidden: Vec<u32> = s
                    .episodes
                    .iter()
                    .filter(|e| e.kind.hides() && !in_range(e.n))
                    .map(|e| e.n)
                    .collect();
                let with_ranges = s
                    .episodes
                    .iter()
                    .filter(|e| (e.kind.hides() && !in_range(e.n)) || in_range(e.n))
                    .count();
                println!("{}: {} episodes {:?}", s.slug, s.episodes.len(), counts);
                println!("  Skip Anime-Only hides {}: {:?}", hidden.len(), hidden);
                if !s.ranges.is_empty() {
                    println!("  with every range seen it hides {}", with_ranges);
                }
            }
        }
        Command::Keygen { out } => {
            let (secret, public) = sign::keygen();
            if out.exists() {
                bail!("{} exists; refusing to overwrite a key", out.display());
            }
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&out, secret + "\n")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o600))?;
            }
            println!("{public}");
        }
        Command::Sign {
            bundle,
            key,
            key_id,
            sequence,
            not_after,
            out,
        } => {
            let payload = std::fs::read(&bundle)?;
            read_bundle(&bundle)?;
            let key = sign::signing_key(&std::fs::read_to_string(&key)?)?;
            let envelope = sign::seal(&key, &key_id, sequence, &not_after, &payload);
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&out, serde_json::to_string(&envelope)?)?;
            println!(
                "sealed {} as {key_id} #{sequence}, valid until {not_after}",
                out.display()
            );
        }
        Command::Verify { envelope, public } => {
            let env: sign::Envelope = serde_json::from_slice(&std::fs::read(&envelope)?)?;
            let payload = sign::open(&env, &public)?;
            let bundle: Bundle = serde_json::from_slice(&payload)?;
            println!(
                "valid: {} #{}, {} shows, until {}",
                env.key_id,
                env.sequence,
                bundle.shows.len(),
                env.not_after
            );
        }
    }
    Ok(())
}
