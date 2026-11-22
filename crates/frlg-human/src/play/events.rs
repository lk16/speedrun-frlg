//! `events.jsonl`: one line per event, `{frame, t, kind, text, nums}` (`human-run/PROMPT.md`,
//! *The decision log*). The log is a run artifact and enough to rebuild the overlay.
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::FPS;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Absolute emulator frame (frames since power-on).
    pub frame: u64,
    /// Run timer in seconds, `None` before New Game.
    pub t: Option<f64>,
    /// `split`, `decision`, `battle`, `catch`, `encounter`, `reset-verdict`, `rollback`,
    /// `model-vs-actual`, `warn`.
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub nums: Value,
}

/// The decision log of one branch. Events of a branch that dies are discarded with it: the
/// controller [`truncate`](EventLog::truncate)s back to the checkpoint, keeping only the
/// `rollback` line it writes afterwards.
#[derive(Clone, Debug, Default)]
pub struct EventLog {
    pub events: Vec<Event>,
    /// Frame at which New Game was selected (the timer start), once known.
    pub new_game_frame: Option<u64>,
}

impl EventLog {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut log = EventLog::default();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let e: Event = serde_json::from_str(line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if log.new_game_frame.is_none() && e.t == Some(0.0) {
                log.new_game_frame = Some(e.frame);
            }
            log.events.push(e);
        }
        Ok(log)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        for e in &self.events {
            serde_json::to_writer(&mut f, e)?;
            f.write_all(b"\n")?;
        }
        f.flush()
    }

    /// Run timer at `frame`, `None` before New Game.
    pub fn timer(&self, frame: u64) -> Option<f64> {
        self.new_game_frame
            .filter(|&s| frame >= s)
            .map(|s| (frame - s) as f64 / FPS)
    }

    pub fn push(&mut self, frame: u64, kind: &str, text: impl Into<String>, nums: Value) {
        let text = text.into();
        eprintln!("[{frame:>7}] {kind:<15} {text}");
        self.events.push(Event {
            frame,
            t: self.timer(frame),
            kind: kind.to_string(),
            text,
            nums,
        });
    }

    /// Drop every event after `frame` (a rollback discards the dead branch).
    pub fn truncate(&mut self, frame: u64) {
        self.events.retain(|e| e.frame <= frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_starts_at_new_game() {
        let mut log = EventLog::default();
        log.push(10, "decision", "boot", Value::Null);
        log.new_game_frame = Some(100);
        log.push(100 + 5973, "split", "x", serde_json::json!({"a": 1}));
        assert_eq!(log.events[0].t, None);
        assert!((log.events[1].t.unwrap() - 100.0).abs() < 0.01);
        log.truncate(50);
        assert_eq!(log.events.len(), 1);
    }
}
