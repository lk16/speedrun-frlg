//! Plan parameters: the machine-readable block of a `human-run/plans/NNN-*.md` file.

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hunt {
    /// Segment name this hunt is inserted after.
    pub after: String,
    pub site: String,
    /// `land`, `water` or `fish`.
    pub area: String,
    /// Step budget (fishing: casts).
    pub steps: u32,
    /// Species the human leaves alone even if new (too rare to be worth the steps).
    pub skip: Vec<String>,
    /// Frames to get to the hunting spot and back (walking out of the way), run speed.
    pub detour_tiles: u32,
}

impl Default for Hunt {
    fn default() -> Self {
        Hunt {
            after: String::new(),
            site: String::new(),
            area: "land".into(),
            steps: 200,
            skip: vec![],
            detour_tiles: 0,
        }
    }
}

/// A gift mon or a one-off wild battle (`givemon` / `setwildbattle` in the decomp scripts).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Special {
    pub after: String,
    /// `gift` or `static`.
    pub kind: String,
    pub species: Vec<String>,
    pub level: u32,
    /// Frames for the whole event (walking out of the way, cutscene, menus), before text_factor.
    pub frames: u32,
    /// Static catches: wear it down to this HP fraction first.
    pub hp_frac: f64,
    pub ball: String,
    pub max_throws: u32,
    /// Species that must already be caught or the event is skipped for free (in-game trades).
    pub requires: Vec<String>,
}

impl Default for Special {
    fn default() -> Self {
        Special {
            after: String::new(),
            kind: "gift".into(),
            species: vec![],
            level: 30,
            frames: 600,
            hp_frac: 0.2,
            ball: "ultra".into(),
            max_throws: 30,
            requires: vec![],
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Params {
    pub name: String,
    pub starter: String,
    /// Reset the starter unless the sum of its six level-5 stats reaches this.
    pub starter_min_total: u32,
    /// Each mashed input is late by U{0..jitter_n} frames.
    pub jitter_n: u32,
    /// Reaction delay after an unpredictable event: U{react_lo..react_hi} frames.
    pub react_lo: u32,
    pub react_hi: u32,
    /// Human slowdown on text/menu frames relative to the frame-perfect floor.
    pub text_factor: f64,
    /// Human slowdown on walking relative to a straight tile count.
    pub path_overhead: f64,
    pub bike: bool,
    pub teleport_abra: bool,
    /// Split-reset aggressiveness: 0 resets only when even the best case cannot make the
    /// target, 1 resets whenever the mean remaining time cannot.
    pub reset_aggr: f64,
    /// `poke`, `great` or `ultra`.
    pub ball: String,
    pub weaken: bool,
    /// Catch new Kanto species met while walking (otherwise flee everything).
    pub catch_on_pass: bool,
    /// Fight (instead of fleeing) wild encounters that are not catch targets.
    pub fight_wild: bool,
    /// Fight non-target wild encounters while the carry is below this level (else flee).
    pub fight_until_level: u32,
    /// Effective-level multiplier of a full team over the lone carry (guess).
    pub team_bonus: f64,
    pub hunts: Vec<Hunt>,
    pub specials: Vec<Special>,
    /// Fallback hunts, tried in order when Oak's check finds fewer than 60 caught.
    pub fill: Vec<Hunt>,
    pub stones: Vec<String>,
    pub mewtwo: bool,
    /// Extra wild battles for exp before the first Elite Four / before the rematch.
    pub grind_pre_e4: u32,
    pub grind_pre_rematch: u32,
    /// Level and base exp yield of what the human grinds on.
    pub grind_level: u32,
    pub grind_yield: u32,
    /// Effective-level multiplier for a caught Mewtwo (guess).
    pub power_bonus: f64,
    /// Scales battle turn and per-mon overhead frames (sensitivity knob; 1.0 is the guess).
    pub battle_scale: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            name: "unnamed".into(),
            starter: "squirtle".into(),
            starter_min_total: 0,
            jitter_n: 4,
            react_lo: 15,
            react_hi: 30,
            text_factor: 1.25,
            path_overhead: 1.12,
            bike: true,
            teleport_abra: false,
            reset_aggr: 0.0,
            ball: "poke".into(),
            weaken: false,
            catch_on_pass: true,
            fight_wild: false,
            fight_until_level: 30,
            team_bonus: 1.08,
            hunts: vec![],
            specials: vec![],
            fill: vec![],
            stones: vec![],
            mewtwo: false,
            grind_pre_e4: 0,
            grind_pre_rematch: 0,
            grind_level: 45,
            grind_yield: 90,
            power_bonus: 1.12,
            battle_scale: 1.0,
        }
    }
}

impl Params {
    /// Reads the first ```json fenced block of a plan file.
    pub fn from_plan_text(text: &str) -> Result<Params, String> {
        let start = text.find("```json").ok_or("plan has no ```json block")? + "```json".len();
        let rest = &text[start..];
        let end = rest.find("```").ok_or("unterminated ```json block")?;
        serde_json::from_str(&rest[..end]).map_err(|e| format!("plan-params: {e}"))
    }
}
