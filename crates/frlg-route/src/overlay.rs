//! The review-video overlay: the side panels of `frlg video --overlay`, built
//! from a run's decision log (`events.jsonl`) as one ASS subtitle track.
//!
//! Layout (1920x1080, `human-run/PROMPT.md`, "The video layout"):
//!
//! | Region      | Box (x,y w x h)   | Content                                        |
//! | ----------- | ----------------- | ---------------------------------------------- |
//! | Game        | 360,16 1200x800   | the GBA picture, 5x nearest-neighbour          |
//! | Left column | 16,16 328x1048    | run timer, splits table, attempt line, starter |
//! | Dex bar     | 360,832 1200x232  | planned species grid, `CAUGHT n / N`           |
//! | Chat column | 1576,16 328x1048  | the decision feed, newest line at the bottom   |
//!
//! The ffmpeg side (`video.rs`) scales and pads the picture, burns this track
//! in with libass and draws the run timer with `drawtext`, whose pts offset
//! makes it count from the New Game frame without an event per frame. This
//! module only decides *what* is on the panels and *when*; it never touches
//! the emulator, so the overlay can be rebuilt from the log alone.
//!
//! The chat text is wrapped here, not by libass (`WrapStyle: 2` turns libass
//! wrapping off): only a fixed-column wrap can promise that a continuation
//! line sits indented under the message text and that nothing overflows.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;

use crate::video::{VideoError, FRAME_RATE_DEN, FRAME_RATE_NUM};

/// Chat column width in characters. DejaVu Sans Mono at ASS size 16 has a
/// 0.602 em advance on a ~13.75 px em: 8.3 px a column, 38 columns in 316 px of
/// the 328 px box. Measured on a rendered still, not just computed.
pub const CHAT_COLS: usize = 38;
/// Lines kept on screen: 1048 px of column at a 16 px line height.
pub const CHAT_LINES: usize = 65;
/// Left-column table width; same font and box width as the chat.
pub const LEFT_COLS: usize = 38;

const FONT: &str = "DejaVu Sans Mono";
const FONT_FILE_BOLD: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf";

/// One line of `events.jsonl` (schema: `crates/frlg-human/src/play/events.rs`).
#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub frame: u64,
    #[serde(default)]
    pub t: Option<f64>,
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub nums: serde_json::Value,
}

impl Event {
    fn num(&self, key: &str) -> Option<f64> {
        self.nums.get(key).and_then(|v| v.as_f64())
    }
    fn str(&self, key: &str) -> Option<&str> {
        self.nums.get(key).and_then(|v| v.as_str())
    }
}

/// Reads an events log. Blank lines are skipped; a malformed line is an
/// error, since the log is a run artifact and never hand-edited.
pub fn read_events(path: &Path) -> Result<Vec<Event>, VideoError> {
    let text = std::fs::read_to_string(path)?;
    parse_events(&text).map_err(|e| VideoError::Message(format!("{}: {e}", path.display())))
}

pub fn parse_events(text: &str) -> Result<Vec<Event>, String> {
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: Event =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", index + 1))?;
        events.push(event);
    }
    // The log is written in order, but a checkpoint may be logged after the
    // split it closes; a stable sort keeps same-frame events in log order.
    events.sort_by_key(|e| e.frame);
    Ok(events)
}

/// What the overlay takes from a plan file: its name, and the species it
/// mentions, as the dex grid's fallback when the log carries no
/// `dex_planned` list.
#[derive(Debug, Clone, Default)]
pub struct PlanInfo {
    pub name: String,
    pub species: Vec<String>,
}

pub fn read_plan(path: &Path) -> Result<PlanInfo, VideoError> {
    let text = std::fs::read_to_string(path)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(parse_plan(&text, &stem))
}

pub fn parse_plan(text: &str, fallback_name: &str) -> PlanInfo {
    let name = text
        .lines()
        .find_map(|l| l.strip_prefix("# Plan "))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| fallback_name.to_string());
    let mut species = Vec::new();
    for token in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if let Some(rest) = token.strip_prefix("SPECIES_") {
            let s = species_key(rest);
            if !s.is_empty() && s != "NONE" && !species.contains(&s) {
                species.push(s);
            }
        }
    }
    PlanInfo { name, species }
}

/// `SPECIES_MR_MIME`, `mr_mime` and `MR_MIME` all name one grid cell.
pub fn species_key(name: &str) -> String {
    name.trim()
        .trim_start_matches("SPECIES_")
        .to_ascii_uppercase()
}

fn species_label(key: &str) -> String {
    key.replace('_', " ")
}

/// Seconds of video at which (0-based) frame `frame` is shown.
pub fn frame_seconds(frame: u64) -> f64 {
    frame as f64 * FRAME_RATE_DEN as f64 / FRAME_RATE_NUM as f64
}

/// `h:mm:ss`, floored, the way the chat stamps and the splits table print a
/// run time.
pub fn hms(seconds: f64) -> String {
    let s = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Signed `+m:ss` / `-h:mm:ss` for a delta against the model.
fn delta(seconds: f64) -> String {
    let sign = if seconds < 0.0 { '-' } else { '+' };
    let s = seconds.abs().round() as u64;
    if s >= 3600 {
        format!("{sign}{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{sign}{}:{:02}", s / 60, s % 60)
    }
}

/// ASS timestamp `h:mm:ss.cc`.
fn ass_time(seconds: f64) -> String {
    let cs = (seconds.max(0.0) * 100.0).round() as u64;
    format!(
        "{}:{:02}:{:02}.{:02}",
        cs / 360_000,
        cs / 6000 % 60,
        cs / 100 % 60,
        cs % 100
    )
}

/// Text as libass will show it literally: braces open override blocks and a
/// backslash starts an escape, so neither may pass through.
fn ass_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '{' => '(',
            '}' => ')',
            '\\' => '/',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}

/// Leading spaces as `\h` hard spaces, which libass never collapses.
fn hard_indent(line: &str) -> String {
    let body = line.trim_start_matches(' ');
    let n = line.len() - body.len();
    format!("{}{}", "\\h".repeat(n), body)
}

/// Wraps one chat message at `width` columns: the first line is
/// `prefix + " " + text`, continuation lines are indented to sit under the
/// text. Words wider than the room left are split. Every returned line is at
/// most `width` characters (counted as chars, the unit of a monospace font).
pub fn wrap_message(prefix: &str, text: &str, width: usize) -> Vec<String> {
    let indent = prefix.chars().count() + 1;
    let room = width.saturating_sub(indent).max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    let push_word = |word: &str, lines: &mut Vec<String>, cur: &mut String, len: &mut usize| {
        let mut chars: Vec<char> = word.chars().collect();
        loop {
            let need = if *len == 0 {
                chars.len()
            } else {
                chars.len() + 1
            };
            if *len + need <= room {
                if *len > 0 {
                    cur.push(' ');
                    *len += 1;
                }
                cur.extend(chars.iter());
                *len += chars.len();
                return;
            }
            if *len > 0 {
                lines.push(std::mem::take(cur));
                *len = 0;
                continue;
            }
            // An empty line and still too long: hard split.
            let rest = chars.split_off(room);
            cur.extend(chars.iter());
            lines.push(std::mem::take(cur));
            chars = rest;
        }
    };
    for word in text.split_whitespace() {
        push_word(word, &mut lines, &mut current, &mut current_len);
    }
    if current_len > 0 || lines.is_empty() {
        lines.push(current);
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(i, body)| {
            if i == 0 {
                format!("{prefix} {body}")
            } else {
                format!("{}{body}", " ".repeat(indent))
            }
        })
        .collect()
}

/// ASS colour (`&HBBGGRR&`) for an event kind.
fn kind_colour(kind: &str) -> &'static str {
    match kind {
        "split" => "&H7CE87C&",           // green
        "decision" => "&HE0E0E0&",        // light grey
        "battle" => "&HFFC89C&",          // light blue
        "catch" => "&H5FD7FF&",           // gold
        "encounter" => "&HD0D08F&",       // teal
        "warn" => "&H40A0FF&",            // orange
        "reset-verdict" => "&HFF80FF&",   // magenta
        "rollback" => "&H6060FF&",        // red
        "model-vs-actual" => "&HFFA0C0&", // violet
        _ => "&HB0B0B0&",
    }
}

const DIM: &str = "&H505050&";
const LABEL: &str = "&H909090&";
const BRIGHT: &str = "&HF0F0F0&";
const GOOD: &str = "&H7CE87C&";
const BAD: &str = "&H6070FF&";
/// Canvas colours, as ffmpeg `0xRRGGBB` (video.rs builds the panels from
/// pads in these).
pub const GUTTER_RGB: &str = "0x0C0C10";
pub const PANEL_RGB: &str = "0x1A1A20";

/// What the overlay needs besides the log.
pub struct OverlayInput<'a> {
    pub events: &'a [Event],
    pub plan: Option<&'a PlanInfo>,
    /// Video frames, tail included: the overlay runs to the last of them.
    pub video_frames: u64,
    /// Frames of the run itself; the timer stops on the last one.
    pub run_frames: u64,
}

/// The built overlay: the ASS track, and where the `drawtext` timer starts
/// and stops (video seconds).
pub struct Overlay {
    pub ass: String,
    /// Video time of the New Game frame (the event with `t == 0`), if the log
    /// reaches it.
    pub timer_start: Option<f64>,
    /// Video time of the run's last frame; the timer freezes there.
    pub timer_end: f64,
}

/// A run of Dialogue states: each entry holds from its start to the next
/// entry's start (the last one to the end of the video).
struct Track {
    states: Vec<(f64, String)>,
}

impl Track {
    fn new() -> Self {
        Track { states: Vec::new() }
    }
    fn set(&mut self, at: f64, text: String) {
        if let Some(last) = self.states.last_mut() {
            if last.1 == text {
                return;
            }
            if (last.0 - at).abs() < 0.005 {
                last.1 = text;
                return;
            }
        }
        self.states.push((at, text));
    }
    fn emit(&self, out: &mut String, layer: u32, style: &str, end: f64) {
        for (i, (start, text)) in self.states.iter().enumerate() {
            let stop = self.states.get(i + 1).map(|s| s.0).unwrap_or(end);
            if stop <= *start || text.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "Dialogue: {layer},{},{},{style},,0,0,0,,{text}",
                ass_time(*start),
                ass_time(stop)
            );
        }
    }
}

struct SplitRow {
    name: String,
    actual: f64,
    p50: Option<f64>,
}

pub fn build(input: &OverlayInput) -> Overlay {
    let events = input.events;
    let end = frame_seconds(input.video_frames);
    let timer_end = frame_seconds(input.run_frames);
    let timer_start = events
        .iter()
        .find(|e| e.t == Some(0.0))
        .map(|e| frame_seconds(e.frame));

    // ---- the plan's species grid -----------------------------------------
    let planned: Vec<String> = events
        .iter()
        .find_map(|e| e.nums.get("dex_planned").and_then(|v| v.as_array()))
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str())
                .map(species_key)
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .or_else(|| input.plan.map(|p| p.species.clone()))
        .unwrap_or_default();
    let plan_name = input
        .plan
        .map(|p| p.name.clone())
        .or_else(|| {
            events.iter().find_map(|e| {
                e.text
                    .strip_prefix("plan ")
                    .and_then(|r| r.split(':').next())
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_default();

    let mut chat = Track::new();
    let mut splits_track = Track::new();
    let mut attempt_track = Track::new();
    let mut starter_track = Track::new();
    let mut count_track = Track::new();

    let mut chat_lines: Vec<String> = Vec::new();
    let mut rows: Vec<SplitRow> = Vec::new();
    let mut obtained: BTreeMap<String, f64> = BTreeMap::new(); // species -> run time
    let mut obtained_at: HashMap<String, f64> = HashMap::new(); // species -> video time
    let mut dex_count_reported = 0u64;
    let mut attempt: Option<u64> = None;
    let mut seed: Option<u64> = None;
    let mut rollbacks = 0u64;

    let attempt_line = |attempt: Option<u64>, seed: Option<u64>, rollbacks: u64| {
        let a = attempt.map(|a| a.to_string()).unwrap_or("-".into());
        let s = seed.map(|s| s.to_string()).unwrap_or("-".into());
        format!(
            "{{\\an7\\pos(24,914)\\fs16\\c{LABEL}}}PLAN {}\\N{{\\c{BRIGHT}}}attempt {a}  seed {s}  rollbacks {rollbacks}",
            ass_escape(&truncate(&plan_name, LEFT_COLS - 5))
        )
    };
    attempt_track.set(0.0, attempt_line(None, None, 0));
    starter_track.set(
        0.0,
        format!("{{\\an7\\pos(24,960)\\fs16\\c{LABEL}}}STARTER\\N{{\\c{DIM}}}not picked yet"),
    );
    splits_track.set(0.0, splits_text(&rows));
    count_track.set(0.0, count_text(0, planned.len()));

    for event in events {
        let at = frame_seconds(event.frame).min(end);

        // chat
        let stamp = match event.t {
            Some(t) => format!("[{}]", hms(t)),
            None => "[--:--]".to_string(),
        };
        let colour = kind_colour(&event.kind);
        for line in wrap_message(&stamp, &ass_escape(&event.text), CHAT_COLS) {
            chat_lines.push(format!("{{\\c{colour}}}{}", hard_indent(&line)));
        }
        if chat_lines.len() > CHAT_LINES {
            let drop = chat_lines.len() - CHAT_LINES;
            chat_lines.drain(..drop);
            // A message scrolls off whole: no orphaned continuation lines
            // at the top of the column.
            let orphans = chat_lines.iter().take_while(|l| l.contains("}\\h")).count();
            chat_lines.drain(..orphans);
        }
        chat.set(
            at,
            format!("{{\\an1\\pos(1582,1064)}}{}", chat_lines.join("\\N")),
        );

        // attempt / seed / rollbacks
        if let Some(a) = event.num("attempt") {
            attempt = Some(a as u64);
        }
        if let Some(s) = event.num("seed") {
            seed = Some(s as u64);
        }
        if event.kind == "rollback" {
            rollbacks += 1;
        }
        attempt_track.set(at, attempt_line(attempt, seed, rollbacks));

        // starter box
        if let Some(species) = event.str("starter") {
            let key = species_key(species);
            let nature = event.str("nature").unwrap_or("?");
            let ivs = event.str("ivs").unwrap_or("?");
            let resets = event.num("resets").map(|r| r as u64).unwrap_or(0);
            let off = event.num("off_timer").unwrap_or(0.0);
            let iv_lines = wrap_message("IVs", &ass_escape(ivs), LEFT_COLS)
                .iter()
                .map(|l| hard_indent(l))
                .collect::<Vec<_>>()
                .join("\\N");
            starter_track.set(
                at,
                format!(
                    "{{\\an7\\pos(24,960)\\fs16\\c{LABEL}}}STARTER\\N{{\\c{BRIGHT}\\b1}}{}{{\\b0}}  {}\\N{{\\c{BRIGHT}}}{}\\N{{\\c{LABEL}}}{resets} reset(s), {} off-timer",
                    species_label(&key),
                    ass_escape(nature),
                    iv_lines,
                    hms(off),
                ),
            );
            if let Some(t) = event.t {
                obtained.entry(key.clone()).or_insert(t);
                obtained_at.entry(key).or_insert(at);
            }
        }

        // splits
        if event.kind == "split" {
            let name = event
                .str("name")
                .map(|s| s.to_string())
                .unwrap_or_else(|| event.text.clone());
            let actual = event.num("actual").or(event.t).unwrap_or(0.0);
            let duplicate = rows
                .iter()
                .any(|r| r.name == name && (r.actual - actual).abs() < 0.01);
            if !duplicate {
                rows.push(SplitRow {
                    name,
                    actual,
                    p50: event.num("p50"),
                });
                splits_track.set(at, splits_text(&rows));
            }
        }

        // dex
        if event.kind == "catch" {
            let named = event.str("species").map(species_key);
            let got = event.nums.get("caught").is_some()
                || event.nums.get("gift").and_then(|v| v.as_bool()) == Some(true)
                || event.text.starts_with("caught ")
                || event.text.starts_with("got ");
            if let (Some(key), true) = (named, got) {
                obtained
                    .entry(key.clone())
                    .or_insert(event.t.unwrap_or(0.0));
                obtained_at.entry(key).or_insert(at);
            }
            if let Some(n) = event.num("caught") {
                dex_count_reported = dex_count_reported.max(n as u64);
            }
        }
        let count = dex_count_reported.max(obtained.len() as u64);
        count_track.set(at, count_text(count, planned.len()));
    }

    // ---- write the file --------------------------------------------------
    let mut out = String::new();
    let _ = write!(
        out,
        "[Script Info]\n\
         ; frlg video --overlay: review panels, layout in human-run/PROMPT.md\n\
         ScriptType: v4.00+\n\
         PlayResX: 1920\n\
         PlayResY: 1080\n\
         WrapStyle: 2\n\
         ScaledBorderAndShadow: yes\n\
         \n\
         [V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: Panel,{FONT},16,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n\
         Style: Chat,{FONT},16,&H00E0E0E0,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,1,0,0,0,1\n\
         Style: Dex,{FONT},14,&H00E0E0E0,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n\
         \n\
         [Events]\n\
         Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
    );

    // Panel backgrounds are not here: video.rs pads the canvas in
    // GUTTER_RGB / PANEL_RGB, which costs a copy instead of a blend.
    // static labels
    let _ = writeln!(
        out,
        "Dialogue: 1,{},{},Panel,,0,0,0,,{{\\an7\\pos(24,22)\\c{LABEL}}}RUN TIMER  (from New Game)",
        ass_time(0.0),
        ass_time(end)
    );
    let _ = writeln!(
        out,
        "Dialogue: 1,{},{},Dex,,0,0,0,,{{\\an7\\pos(370,838)\\fs16\\c{LABEL}}}DEX  plan {}",
        ass_time(0.0),
        ass_time(end),
        ass_escape(&plan_name)
    );

    chat.emit(&mut out, 1, "Chat", end);
    splits_track.emit(&mut out, 1, "Panel", end);
    attempt_track.emit(&mut out, 1, "Panel", end);
    starter_track.emit(&mut out, 1, "Panel", end);
    count_track.emit(&mut out, 1, "Dex", end);

    // dex grid: 10 columns of 118 px, 6 rows of 34 px under the header
    let cols = 10usize;
    let rows_n = planned.len().div_ceil(cols).max(1);
    let row_h = (196 / rows_n).clamp(16, 34);
    for (i, key) in planned.iter().enumerate() {
        let x = 370 + (i % cols) * 118;
        let y = 862 + (i / cols) * row_h;
        let name = ass_escape(&truncate(&species_label(key), 15));
        let pos = format!("{{\\an7\\pos({x},{y})}}");
        match obtained_at.get(key) {
            Some(&shown) => {
                let when = obtained.get(key).copied().unwrap_or(0.0);
                if shown > 0.0 {
                    let _ = writeln!(
                        out,
                        "Dialogue: 1,{},{},Dex,,0,0,0,,{pos}{{\\c{DIM}}}{name}\\N--",
                        ass_time(0.0),
                        ass_time(shown)
                    );
                }
                let _ = writeln!(
                    out,
                    "Dialogue: 1,{},{},Dex,,0,0,0,,{pos}{{\\c{GOOD}\\b1}}{name}{{\\b0}}\\N{{\\c{BRIGHT}}}{}",
                    ass_time(shown),
                    ass_time(end),
                    hms(when)
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "Dialogue: 1,{},{},Dex,,0,0,0,,{pos}{{\\c{DIM}}}{name}\\N--",
                    ass_time(0.0),
                    ass_time(end)
                );
            }
        }
    }

    Overlay {
        ass: out,
        timer_start,
        timer_end,
    }
}

fn truncate(text: &str, cols: usize) -> String {
    text.chars().take(cols).collect()
}

fn count_text(count: u64, planned: usize) -> String {
    format!(
        "{{\\an9\\pos(1550,836)\\fs22\\b1\\c{BRIGHT}}}CAUGHT {count} / {}",
        planned.max(1)
    )
}

/// Rows of the splits table, newest at the bottom; when the table outgrows
/// its box the oldest rows scroll off the top.
const SPLIT_ROWS: usize = 46;

fn splits_text(rows: &[SplitRow]) -> String {
    let mut lines = vec![format!(
        "{{\\c{LABEL}}}{:<11} {:>7} {:>7} {:>8}",
        "SPLIT", "ACTUAL", "P50", "DELTA"
    )
    .replace(' ', "\\h")];
    let skip = rows.len().saturating_sub(SPLIT_ROWS);
    for row in &rows[skip..] {
        let name = truncate(&ass_escape(&row.name), 11);
        let (p50, d, colour) = match row.p50 {
            Some(p) => {
                let diff = row.actual - p;
                (hms(p), delta(diff), if diff > 0.0 { BAD } else { GOOD })
            }
            None => ("-".to_string(), String::new(), BRIGHT),
        };
        let left = format!("{name:<11} {:>7} {p50:>7} ", hms(row.actual)).replace(' ', "\\h");
        lines.push(format!(
            "{{\\c{BRIGHT}}}{left}{{\\c{colour}}}{}",
            format!("{d:>8}").replace(' ', "\\h")
        ));
    }
    format!("{{\\an7\\pos(24,100)}}{}", lines.join("\\N"))
}

/// The `drawtext` filters that draw the big run timer: a dash placeholder
/// before New Game, a pts clock (offset so New Game reads zero) during the
/// run, and the frozen final time over the tail.
pub fn timer_filters(overlay: &Overlay) -> Vec<String> {
    let common = format!("fontfile={FONT_FILE_BOLD}:fontsize=40:x=336-tw:y=46:fontcolor=0xF0F0F0");
    let mut filters = Vec::new();
    match overlay.timer_start {
        None => filters.push(format!("drawtext={common}:text='--\\:--\\:--.---'")),
        Some(start) => {
            filters.push(format!(
                "drawtext={common}:text='--\\:--\\:--.---':enable='lt(t,{start:.6})'"
            ));
            filters.push(format!(
                "drawtext={common}:text='%{{pts\\:hms\\:{:.6}}}':enable='between(t,{start:.6},{:.6})'",
                -start, overlay.timer_end
            ));
            let total = (overlay.timer_end - start).max(0.0);
            let ms = (total * 1000.0).round() as u64;
            let frozen = format!(
                "{:02}\\:{:02}\\:{:02}.{:03}",
                ms / 3_600_000,
                ms / 60_000 % 60,
                ms / 1000 % 60,
                ms % 1000
            );
            filters.push(format!(
                "drawtext={common}:fontcolor=0x7CE87C:text='{frozen}':enable='gt(t,{:.6})'",
                overlay.timer_end
            ));
        }
    }
    filters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_never_overflows_and_indents_under_the_text() {
        let text = "rollback #23 at tutorial: waiting for the battle's action menu timed out \
                    after 4000 frames (log is 55510 frames long) \
                    SUPERCALIFRAGILISTICEXPIALIDOCIOUS_AND_THEN_SOME_MORE_CHARACTERS";
        for prefix in ["[--:--]", "[0:00:29]", "[12:34:56]"] {
            let lines = wrap_message(prefix, text, CHAT_COLS);
            assert!(lines.len() > 2);
            let indent = prefix.len() + 1;
            for (i, line) in lines.iter().enumerate() {
                assert!(line.chars().count() <= CHAT_COLS, "{line:?} overflows");
                if i == 0 {
                    assert!(line.starts_with(prefix));
                } else {
                    assert!(line.starts_with(&" ".repeat(indent)));
                    assert_ne!(line.chars().nth(indent), Some(' '));
                }
            }
            // nothing lost
            let joined: String = lines
                .iter()
                .enumerate()
                .map(|(i, l)| if i == 0 { &l[indent..] } else { l.trim_start() })
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(
                joined.replace(' ', ""),
                text.replace(' ', ""),
                "wrapping lost text"
            );
        }
    }

    #[test]
    fn worst_case_38_column_lines() {
        // A message of exactly the room left fits on one line; one more char
        // wraps.
        let prefix = "[0:00:00]";
        let room = CHAT_COLS - prefix.len() - 1;
        let exact = "x".repeat(room);
        assert_eq!(wrap_message(prefix, &exact, CHAT_COLS).len(), 1);
        let over = "x".repeat(room + 1);
        assert_eq!(wrap_message(prefix, &over, CHAT_COLS).len(), 2);
        assert_eq!(
            wrap_message(prefix, "", CHAT_COLS),
            vec!["[0:00:00] ".to_string()]
        );
    }

    #[test]
    fn times_format() {
        assert_eq!(hms(5532.02), "1:32:12");
        assert_eq!(hms(29.9), "0:00:29");
        assert_eq!(ass_time(3661.234), "1:01:01.23");
        assert_eq!(delta(1878.0), "+31:18");
        assert_eq!(delta(-65.0), "-1:05");
        assert_eq!(delta(3700.0), "+1:01:40");
    }

    fn sample() -> Vec<Event> {
        parse_events(
            r#"{"frame":0,"t":null,"kind":"decision","text":"plan x: dex","nums":{"attempt":1,"seed":11,"dex_planned":["PIDGEY","SQUIRTLE","EEVEE"]}}
{"frame":639,"t":0.0,"kind":"split","text":"New Game selected: timer starts","nums":{"actual":0.0,"name":"new game"}}
{"frame":7091,"t":108.0,"kind":"decision","text":"starter accepted","nums":{"ivs":"hp27 atk31","nature":"Rash","off_timer":2816.0,"resets":22,"starter":"squirtle"}}
{"frame":29335,"t":480.4,"kind":"catch","text":"caught PIDGEY (dex 2 of 60) with 2 ball(s)","nums":{"balls":2,"caught":2,"species":"PIDGEY"}}
{"frame":64779,"t":1073.9,"kind":"split","text":"split Brock","nums":{"actual":1073.9,"name":"Brock","p50":876.7}}
{"frame":64779,"t":1073.9,"kind":"split","text":"split Brock","nums":{"actual":1073.9,"name":"Brock","p50":876.7}}
{"frame":70000,"t":1160.0,"kind":"catch","text":"got Eevee (gift)","nums":{"gift":true,"species":"SPECIES_EEVEE"}}"#,
        )
        .unwrap()
    }

    #[test]
    fn overlay_tracks_dex_splits_and_timer() {
        let events = sample();
        let o = build(&OverlayInput {
            events: &events,
            plan: None,
            video_frames: 80_000,
            run_frames: 79_880,
        });
        assert!((o.timer_start.unwrap() - frame_seconds(639)).abs() < 1e-9);
        assert!(o.ass.contains("CAUGHT 3 / 3"), "starter + pidgey + eevee");
        assert!(o.ass.contains("+3:17"), "Brock delta");
        // the duplicate split is one row
        let last_split = o.ass.lines().rfind(|l| l.contains("SPLIT")).unwrap();
        assert_eq!(last_split.matches("Brock").count(), 1);
        assert!(o.ass.contains("[0:08:00]"));
        let filters = timer_filters(&o);
        assert_eq!(filters.len(), 3);
        assert!(filters[1].contains("pts\\:hms\\:-10.69"));
    }

    #[test]
    fn plan_species_fallback() {
        let p = parse_plan(
            "# Plan 1122-lgdescent\n```json\n{\"species\":[\"SPECIES_EEVEE\",\"SPECIES_MR_MIME\",\"SPECIES_EEVEE\"]}\n```",
            "x",
        );
        assert_eq!(p.name, "1122-lgdescent");
        assert_eq!(p.species, vec!["EEVEE", "MR_MIME"]);
    }
}
