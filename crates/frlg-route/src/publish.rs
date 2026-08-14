//! What has to be true before a run may leave the project, and the text that
//! goes with it when it does.
//!
//! A video is the one artifact here that gets watched by people who will never
//! open the ledger, and it cannot be re-checked once it is uploaded. So the
//! gate is a refusal, not a warning, and it asks two things:
//!
//! 1. **Tier 2 has passed.** Not tier 1: the sandbox's own emulator agreeing
//!    with itself is not acceptance. BizHawk replaying the exported `.bk2` is
//!    (`docs/rival-1/route.md`), and that verdict lives in every segment's
//!    `tier2` field.
//! 2. **The run is committed.** The title claims a frame count and the
//!    description links a commit; both are worthless if the logs on disk are
//!    not the logs in git. So the ledger and every log it names must be
//!    tracked and unmodified, and the commit that recorded the tier-2 verdict
//!    is found by walking the ledger's own history rather than assumed to be
//!    HEAD.
//!
//! Everything here shells out to `git`. That is deliberate: the alternative is
//! a git library this project cannot install, and the questions being asked
//! ("is this file dirty", "which commit introduced this line") are exactly the
//! ones the CLI answers well.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ledger::Ledger;
use crate::segments::{Target, Version};
use crate::video::{Report, FRAME_RATE_DEN, FRAME_RATE_NUM};

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("git {command} failed: {message}")]
    Git { command: String, message: String },
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// A commit, as much of it as a description needs.
#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    /// Committer date, `YYYY-MM-DD`.
    pub date: String,
    pub subject: String,
}

/// The tier-2 evidence behind a publishable run.
#[derive(Debug, Clone)]
pub struct Tier2 {
    /// The commit that first recorded this verdict -- the one worth linking,
    /// rather than whatever HEAD happens to be.
    pub commit: Commit,
    /// The verify-runner's id for the result, e.g. `route-9658f-269d169cd6db`,
    /// when the ledger's sentence names one.
    pub result_id: Option<String>,
    /// The ledger's own sentence, as written.
    pub claim: String,
}

/// Refuse unless the run is tier-2 verified *and* committed, and report the
/// commit that verified it.
pub fn gate(ledger: &Ledger, ledger_path: &Path) -> Result<Tier2, PublishError> {
    require_repo_root()?;

    let unverified: Vec<&str> = ledger
        .segments
        .iter()
        .filter(|s| !s.tier2.starts_with("passed"))
        .map(|s| s.name.as_str())
        .collect();
    if !unverified.is_empty() {
        return Err(PublishError::Refused(format!(
            "tier 2 has not passed for {} of {} segments ({}). A video is an acceptance \
             artifact: queue the movie with `frlg route export`, have the host run \
             tools/verify-runner.sh --headless, and write the verdict back into the ledger \
             first (docs/rival-1/route.md).",
            unverified.len(),
            ledger.segments.len(),
            unverified.join(", "),
        )));
    }
    let untested: Vec<&str> = ledger
        .segments
        .iter()
        .filter(|s| !s.tier1)
        .map(|s| s.name.as_str())
        .collect();
    if !untested.is_empty() {
        return Err(PublishError::Refused(format!(
            "the ledger claims tier 2 but not tier 1 for {}; that combination means the \
             ledger is stale -- re-run `frlg route verify --write`",
            untested.join(", "),
        )));
    }

    let ledger_rel = repo_path(ledger_path);
    let mut paths = vec![ledger_rel.clone()];
    paths.extend(ledger.segments.iter().map(|s| s.log.clone()));

    for path in &paths {
        if git(&["ls-files", "--error-unmatch", "--", path]).is_err() {
            return Err(PublishError::Refused(format!(
                "{path} is not committed. The description links a commit for the tier-2 \
                 verification, so everything it vouches for has to be in git first."
            )));
        }
    }
    let mut status = vec![
        "status".to_string(),
        "--porcelain".to_string(),
        "--".to_string(),
    ];
    status.extend(paths.iter().cloned());
    let dirty = git_owned(&status)?;
    if !dirty.trim().is_empty() {
        return Err(PublishError::Refused(format!(
            "the run has uncommitted changes; commit them and re-verify before publishing:\n{}",
            dirty.trim()
        )));
    }

    let commit = tier2_commit(ledger, &ledger_rel)?;
    Ok(Tier2 {
        commit,
        result_id: result_id(&ledger.segments[0].tier2),
        claim: ledger.segments[0].tier2.clone(),
    })
}

/// The commit that introduced the tier-2 verdict now in the ledger.
///
/// Walks the ledger's history newest-first and keeps going while the file
/// there still carries exactly these verdicts; the last commit that does is
/// the one that recorded them. Linking HEAD instead would point a viewer at
/// whatever unrelated work happened to land afterwards.
fn tier2_commit(ledger: &Ledger, ledger_rel: &str) -> Result<Commit, PublishError> {
    let current: Vec<&str> = ledger.segments.iter().map(|s| s.tier2.as_str()).collect();
    let history = git_owned(&[
        "log".to_string(),
        "--format=%H".to_string(),
        "--".to_string(),
        ledger_rel.to_string(),
    ])?;

    let mut introducing: Option<String> = None;
    for sha in history.lines() {
        let blob = match git(&["show", &format!("{sha}:{ledger_rel}")]) {
            Ok(text) => text,
            Err(_) => break,
        };
        let stamps: Option<Vec<String>> = serde_json::from_str::<serde_json::Value>(&blob)
            .ok()
            .and_then(|value| {
                Some(
                    value
                        .get("segments")?
                        .as_array()?
                        .iter()
                        .map(|s| {
                            s.get("tier2")
                                .and_then(|t| t.as_str())
                                .unwrap_or_default()
                                .to_string()
                        })
                        .collect(),
                )
            });
        match stamps {
            Some(stamps) if stamps == current => introducing = Some(sha.to_string()),
            _ => break,
        }
    }

    let sha = introducing.ok_or_else(|| {
        PublishError::Refused(format!(
            "the tier-2 verdict in {ledger_rel} is not in any commit yet -- commit the \
             verified ledger before publishing a video of it"
        ))
    })?;
    commit_meta(&sha)
}

fn commit_meta(sha: &str) -> Result<Commit, PublishError> {
    let text = git(&["show", "-s", "--format=%H%n%h%n%cs%n%s", sha])?;
    let mut lines = text.lines();
    let mut next = |what: &str| -> Result<String, PublishError> {
        lines
            .next()
            .map(str::to_string)
            .ok_or_else(|| PublishError::Git {
                command: format!("show -s {sha}"),
                message: format!("no {what} in the output"),
            })
    };
    Ok(Commit {
        sha: next("sha")?,
        short: next("short sha")?,
        date: next("date")?,
        subject: next("subject")?,
    })
}

/// The verify-runner's id out of a ledger sentence like
/// "passed 2026-08-12 as part of route-9658f-269d169cd6db: ...".
fn result_id(claim: &str) -> Option<String> {
    claim
        .split(|c: char| c.is_whitespace() || c == ':' || c == ',' || c == '(' || c == ')')
        .find(|word| word.starts_with("route-") && word.len() > "route-".len())
        .map(str::to_string)
}

/// The project's web URL, for links a viewer can follow.
///
/// `origin` only: this repo also carries a `git://127.0.0.1` remote for moving
/// commits out of the sandbox, and no link should ever point at that.
pub fn web_url(explicit: Option<&str>) -> Result<String, PublishError> {
    if let Some(url) = explicit {
        return Ok(url.trim_end_matches('/').to_string());
    }
    let origin = git(&["remote", "get-url", "origin"]).map_err(|_| {
        PublishError::Refused(
            "no `origin` remote to build links from; pass --repo-url https://host/owner/repo"
                .into(),
        )
    })?;
    to_web_url(origin.trim()).ok_or_else(|| {
        PublishError::Refused(format!(
            "cannot turn origin {origin:?} into a web URL; pass --repo-url"
        ))
    })
}

/// `git@host:owner/repo.git`, `ssh://git@host/owner/repo.git` and
/// `https://host/owner/repo.git` all name the same page.
fn to_web_url(remote: &str) -> Option<String> {
    let rest = match remote.strip_prefix("git@") {
        // scp-style: the colon after the host is a path separator.
        Some(rest) => rest.replacen(':', "/", 1),
        None => remote
            .strip_prefix("ssh://git@")
            .or_else(|| remote.strip_prefix("https://"))
            .or_else(|| remote.strip_prefix("http://"))?
            .to_string(),
    };
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    if !rest.contains('/') {
        return None;
    }
    Some(format!("https://{rest}"))
}

fn require_repo_root() -> Result<(), PublishError> {
    let prefix = git(&["rev-parse", "--show-prefix"])?;
    if !prefix.trim().is_empty() {
        return Err(PublishError::Refused(format!(
            "run this from the repository root: the ledger names its logs repo-relative, \
             and this is {}",
            prefix.trim()
        )));
    }
    Ok(())
}

/// A path as git wants to hear about it: repo-relative, forward slashes.
fn repo_path(path: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    path.strip_prefix(&cwd)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn git(args: &[&str]) -> Result<String, PublishError> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(PublishError::Git {
            command: args.join(" "),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn git_owned(args: &[String]) -> Result<String, PublishError> {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    git(&borrowed)
}

/// The video's title, its description, and where they go.
pub struct Publication {
    /// The YouTube title.
    pub title: String,
    /// The whole markdown file: title, description to paste, and the
    /// provenance behind both.
    pub markdown: String,
    /// `<stamp>-<target>-<frames>f`, the name of the folder and of both files
    /// inside it. Dated like the journal entries (`docs/<target>/journal/`).
    pub slug: String,
}

/// Everything the description quotes, gathered by the caller so this function
/// stays a pure formatter -- which is what makes it testable without a repo,
/// an emulator or ffmpeg.
pub struct Facts<'a> {
    pub ledger: &'a Ledger,
    pub target: Target,
    pub version: Version,
    /// What the run achieves, in title case: "Defeat Rival 1".
    pub goal: &'a str,
    pub tier2: &'a Tier2,
    pub web_url: &'a str,
    pub encode: &'a Report,
    /// `YYYY-MM-DD-HH-MM`, UTC.
    pub stamp: &'a str,
}

/// What a target achieves, as it should read in a title.
pub fn default_goal(target: Target) -> &'static str {
    match target {
        Target::Rival1 => "Defeat Rival 1",
        Target::DefeatBrock => "Defeat Brock",
    }
}

pub fn version_name(version: Version) -> &'static str {
    match version {
        Version::FireRed => "FireRed",
        Version::LeafGreen => "LeafGreen",
    }
}

/// The video's title. The frame count is in it because frames, not seconds,
/// are what this project optimises and what another run can be compared
/// against; the seconds are in the description.
pub fn title(version: Version, goal: &str, frames: usize) -> String {
    format!(
        "TAS Pokemon {} - {goal} - {frames} frames",
        version_name(version)
    )
}

/// The folder and file name: dated like the journal entries
/// (`docs/<target>/journal/`), then what the run is.
pub fn slug(stamp: &str, target: Target, frames: usize) -> String {
    format!("{stamp}-{}-{frames}f", target.name())
}

pub fn publication(facts: &Facts) -> Publication {
    let frames = facts.ledger.total_frames;
    let version = version_name(facts.version);
    let title = title(facts.version, facts.goal, frames);
    let slug = slug(facts.stamp, facts.target, frames);
    let commit_url = format!("{}/commit/{}", facts.web_url, facts.tier2.commit.sha);
    let readme_url = format!("{}#readme", facts.web_url);
    let description = description(facts, version, &commit_url, &readme_url);
    let markdown = markdown(facts, &title, &description, &commit_url);
    Publication {
        title,
        markdown,
        slug,
    }
}

fn description(facts: &Facts, version: &str, commit_url: &str, readme_url: &str) -> String {
    let frames = facts.ledger.total_frames;
    let mut out = String::new();
    out.push_str(&format!(
        "Tool-assisted speedrun of Pokemon {version}: {} in {frames} frames ({}), \
         from power-on with no save file.\n\n",
        lower_first(facts.goal),
        clock(frames),
    ));
    out.push_str(
        "Every input is verified twice: a headless libmgba harness replays the run from \
         reset and checks the game's own memory, and BizHawk replays the exported .bk2 \
         movie frame for frame. This is the commit that records that second verification:\n",
    );
    out.push_str(commit_url);
    out.push_str("\n\nWhat the project is, and how the route was derived:\n");
    out.push_str(readme_url);
    out.push_str("\n\nTools used:\n");
    out.push_str(
        "- mGBA (libmgba) - the headless emulator the route is built and checked against\n",
    );
    out.push_str("- BizHawk - replays the .bk2 movie for the final verification\n");
    out.push_str("- pret/pokefirered - the decompilation every routing decision is derived from\n");
    out.push_str(
        "- Rust - the routing toolchain in the repo above: RNG and battle models, route \
         builder, movie and video export\n",
    );
    out.push_str(&format!(
        "- ffmpeg - this recording. {}\n",
        facts.encode.format.describe()
    ));
    out.push_str(
        "\nThe routing was done with the help of a modern AI tool, used under a personal \
         licence only.\n",
    );
    out
}

fn markdown(facts: &Facts, title: &str, description: &str, commit_url: &str) -> String {
    let led = facts.ledger;
    let encode = facts.encode;
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title);
    out.push_str(
        "\n\nThe line above is the video's title; the block below is its description. \
                  Both are generated by `frlg video`, from the ledger and from git -- edit them \
                  here rather than in the upload form, so the file stays what was published.\n",
    );

    out.push_str("\n## Description\n\n```\n");
    out.push_str(description);
    out.push_str("```\n");

    out.push_str("\n## Provenance\n\n");
    out.push_str("| | |\n| --- | --- |\n");
    let mut row = |key: &str, value: String| {
        out.push_str(&format!("| {key} | {value} |\n"));
    };
    row("target", format!("`{}`", led.target));
    row(
        "version",
        match facts.version {
            Version::FireRed => "FireRed".to_string(),
            Version::LeafGreen => "LeafGreen".to_string(),
        },
    );
    row("starter", led.starter.clone());
    row(
        "frames",
        format!("{} ({})", led.total_frames, clock(led.total_frames)),
    );
    row("rom sha1", format!("`{}`", led.rom_sha1));
    row("boot", format!("`{}`", led.bios));
    row(
        "tier 1",
        format!("{} segments, all replayed", led.segments.len()),
    );
    row(
        "tier 2",
        format!(
            "{}{}",
            facts
                .tier2
                .result_id
                .as_deref()
                .map(|id| format!("`{id}`, "))
                .unwrap_or_default(),
            facts.tier2.commit.date
        ),
    );
    row(
        "tier-2 commit",
        format!(
            "[`{}`]({}) {}",
            facts.tier2.commit.short, commit_url, facts.tier2.commit.subject
        ),
    );
    row("container", encode.format.describe().to_string());
    row(
        "picture",
        format!(
            "{}x{} at {}/{} fps ({} frames, {} of tail)",
            encode.width,
            encode.height,
            FRAME_RATE_NUM,
            FRAME_RATE_DEN,
            encode.frames,
            encode.frames.saturating_sub(led.total_frames),
        ),
    );
    row(
        "sound",
        format!(
            "{} Hz, {} sample frames, {} rate change(s) during the run",
            encode.audio_rate,
            encode.audio_frames,
            encode.rate_changes.len().saturating_sub(1),
        ),
    );
    row("file", format!("{} MiB", encode.bytes / (1 << 20)));
    row("encoder", encode.ffmpeg_version.clone());
    row("generated", format!("{} UTC", facts.stamp));

    out.push_str("\nThe ledger's own tier-2 sentence, as committed:\n\n> ");
    out.push_str(&facts.tier2.claim);
    out.push('\n');
    out
}

/// Frames as a wall-clock time, at the rate every published number in this
/// repo uses.
pub fn clock(frames: usize) -> String {
    let seconds = frames as f64 * FRAME_RATE_DEN as f64 / FRAME_RATE_NUM as f64;
    let minutes = (seconds / 60.0).floor() as u64;
    format!("{minutes}m{:05.2}s", seconds - minutes as f64 * 60.0)
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `YYYY-MM-DD-HH-MM` in UTC, the shape the journal entries use
/// (`docs/<target>/journal/`).
///
/// UTC rather than local time, and hand-rolled rather than a date crate: the
/// sandbox cannot add a dependency, and a folder name only has to be unique
/// and sortable.
pub fn stamp(now: SystemTime) -> String {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}-{:02}-{:02}",
        rest / 3600,
        (rest % 3600) / 60
    )
}

/// Howard Hinnant's days-from-civil, inverted: days since 1970-01-01 to a
/// calendar date. Public-domain algorithm, and short enough to check by hand
/// against a couple of known dates (see the tests).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_are_journal_shaped() {
        assert_eq!(
            stamp(UNIX_EPOCH + std::time::Duration::from_secs(0)),
            "1970-01-01-00-00"
        );
        assert_eq!(
            stamp(UNIX_EPOCH + std::time::Duration::from_secs(1_755_155_700)),
            "2025-08-14-07-15"
        );
    }

    #[test]
    fn result_ids_come_out_of_the_ledger_sentence() {
        assert_eq!(
            result_id("passed 2026-08-12 as part of route-9658f-269d169cd6db: BizHawk"),
            Some("route-9658f-269d169cd6db".to_string())
        );
        assert_eq!(
            result_id("not replayed: queue with `frlg route export`"),
            None
        );
    }

    #[test]
    fn remotes_become_web_urls() {
        assert_eq!(
            to_web_url("git@github.com:lk16/speedrun-frlg.git").as_deref(),
            Some("https://github.com/lk16/speedrun-frlg")
        );
        assert_eq!(
            to_web_url("https://github.com/lk16/speedrun-frlg").as_deref(),
            Some("https://github.com/lk16/speedrun-frlg")
        );
        // The sandbox's own transport remote is not a web page.
        assert_eq!(to_web_url("git://127.0.0.1:32814/speedrun-frlg"), None);
    }

    #[test]
    fn clock_matches_the_published_times() {
        // docs/rival-1/route.md publishes 9658 frames as ~2m42s.
        assert_eq!(clock(9658), "2m41.70s");
    }
}
