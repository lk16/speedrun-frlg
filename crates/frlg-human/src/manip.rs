//! Frame-exact starter and wild-seed manipulation (`starter_model = "manip"`).
//!
//! Mechanism and measurements: `docs/tricks/starter-manip.md` (decomp-cited there). Summary of
//! what this module models:
//!
//! - three decisive presses per attempt (title A = wild-encounter seed, naming-screen exit and the A
//!   that dismisses "really energetic" = starter PID/IVs); each lands on the target frame with
//!   probability `p_hit`, derived from a per-press timing jitter `sigma` (frames) and a calibration
//!   offset `mu` or set directly;
//! - a miss on any press gives an independent random starter and, for the title press, a random wild
//!   seed (the starter table is keyed on the title frame, doc 3.1); he finds out at the stats screen
//!   (the existing `starter` check) and hard-resets, which costs `s_reset` off the timer;
//! - a hit picks a (nature, IVs) from the reachable-good set: uniform over natures allowed and over
//!   the IV ranges above the thresholds (the doc's density table is exactly this product,
//!   `0.933` is the reachable fraction of stream indices after NPC draws, doc 1.4);
//! - waiting for a better tier costs on-timer idle frames: the best (naming idle N + ball idle j)
//!   pair with density `rho` has cost Rayleigh with mean `sqrt(pi / (2 * 0.933 * rho))` frames
//!   (doc 4.2: T1 15, T1R 18, T2 65, T3 147, T4 179);
//! - the title press can also target a seed that is quiet in the first `quiet_window` rolls, at
//!   most `quiet_max_hits` encounters there (doc 3.4: P(<=2)=0.37%, P(<=4)=4.98%); the wait for a
//!   title frame with such a seed is off the timer but counts in the wall time.
//!
//! Everything here is a model; constants that are not from the doc or the scan data are guesses and
//! labelled so in the field docs. Fitted values: `human-run/plans/374-manip-starter.md`.

use crate::rng::Rng;
use frlg_mon::stats::NATURE_NAMES;
use serde::Deserialize;

pub const FPS: f64 = 59.7275;
/// Fraction of stream indices not skipped by an NPC draw (doc 1.4: 147 skipped / 2187 spanned).
pub const REACHABLE: f64 = 0.933;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ManipParams {
    /// Natures accepted (names as in `NATURE_NAMES`). Rash and Mild are the common case; Modest is
    /// played too (`STRATEGY.md` 1.5) and adds a third (+SpA, -Atk, free for a Water Pulse plan).
    pub natures: Vec<String>,
    /// Minimum IVs (0 = unconstrained). SpA >= 30 in every kept starter of the scans.
    pub min_spa: u32,
    pub min_spe: u32,
    pub min_hp: u32,
    pub min_def: u32,
    pub min_atk: u32,
    pub min_spd: u32,
    /// Per-press timing jitter (std dev, frames) and calibration offset (frames). The hit
    /// probability of one press is `P(|N(mu, sigma)| < 0.5)`. Fitted, see the plan file.
    pub sigma: f64,
    pub mu: f64,
    /// Dials the table can use to reach the target: 2 = naming idle N and ball idle j (doc 4.2,
    /// needs an emulator-built table of the naming seeds), 1 = ball idle j alone (doc 4.2: T1 143
    /// frames, T2 45 s, T3 3.8 min). The scan data cannot tell which Main97 has; see the plan file.
    pub dials: u32,
    /// Overrides the derived per-press hit probability when >= 0.
    pub p_hit: f64,
    /// P(he keeps a starter that hit every press and meets the tier) -- misses by other causes
    /// (timer program, a blocked NPC, the rival fight) go here. 1 = no other failure.
    pub p_accept: f64,
    /// Seconds off the timer, per attempt segment (all guesses fitted to the scans, see the plan):
    /// power-on to the title press, including the title wait and re-arming the timer program.
    pub s_boot_title: f64,
    /// Extra on-timer seconds (beyond the route's own segments) New Game -> naming exit, naming exit
    /// -> ball dismiss, ball dismiss -> stats page.
    pub s_title_naming: f64,
    pub s_naming_ball: f64,
    pub s_ball_stats: f64,
    /// Off the timer: the soft reset after a miss seen at the stats screen, back to the boot.
    pub s_reset: f64,
    /// Early game quiet seed: 0 = no table, else at most this many encounters in the first
    /// `quiet_window` encounter rolls (rate tests) of the run.
    pub quiet_max_hits: u32,
    pub quiet_window: u32,
    /// Title frames the table can reach before the attract screen restarts the boot
    /// (the scan covered 648; beyond it the attempt reboots, paying `s_boot_title` again).
    pub quiet_wait_cap: u32,
    /// P(a title frame has a quiet seed) for `quiet_max_hits`, from the all-seeds scan (doc 3.4).
    /// Empty = look up the doc table.
    pub quiet_density: f64,
    /// The quiet table also picks the species: encounters the human wants (a new species he would
    /// throw a ball at: the forest bug, the Route 1 kills for exp are `quiet_max_hits`) still happen
    /// inside the quiet window; only the unwanted ones are suppressed (`STRATEGY.md` 2: "a perfect
    /// two encounter forest").
    pub quiet_keeps_wanted: bool,
}

impl Default for ManipParams {
    fn default() -> Self {
        ManipParams {
            natures: vec!["Rash".into(), "Mild".into()],
            min_spa: 30,
            min_spe: 20,
            min_hp: 0,
            min_def: 0,
            min_atk: 0,
            min_spd: 0,
            sigma: 0.49,
            mu: 0.0,
            dials: 2,
            p_hit: -1.0,
            p_accept: 1.0,
            s_boot_title: 45.0,
            s_title_naming: 0.0,
            s_naming_ball: 0.0,
            s_ball_stats: 0.0,
            s_reset: 100.0,
            quiet_max_hits: 0,
            quiet_window: 60,
            quiet_wait_cap: 648,
            quiet_density: -1.0,
            quiet_keeps_wanted: false,
        }
    }
}

/// Standard normal CDF (Abramowitz-Stegun 7.1.26 erf, |error| < 1.5e-7).
fn phi(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * (x.abs() / std::f64::consts::SQRT_2));
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let erf = 1.0 - poly * (-(x * x) / 2.0).exp();
    0.5 * (1.0 + if x >= 0.0 { erf } else { -erf })
}

impl ManipParams {
    /// Doc 4.1: `p(sigma, mu) = Phi((0.5 - mu)/sigma) - Phi((-0.5 - mu)/sigma)`.
    pub fn press_p(&self) -> f64 {
        if self.p_hit >= 0.0 {
            return self.p_hit.min(1.0);
        }
        if self.sigma <= 0.0 {
            return if self.mu.abs() < 0.5 { 1.0 } else { 0.0 };
        }
        phi((0.5 - self.mu) / self.sigma) - phi((-0.5 - self.mu) / self.sigma)
    }

    pub fn validate(&self) -> Result<(), String> {
        for n in &self.natures {
            nature_index(n).ok_or_else(|| format!("manip: unknown nature {n}"))?;
        }
        if self.natures.is_empty() {
            return Err("manip: no natures".into());
        }
        Ok(())
    }

    fn nature_mask(&self) -> u32 {
        self.natures
            .iter()
            .filter_map(|n| nature_index(n))
            .fold(0u32, |m, i| m | 1 << i)
    }

    /// IV minima in the order the sim keeps them: HP, Atk, Def, Spe, SpA, SpD.
    pub fn mins(&self) -> [u32; 6] {
        [
            self.min_hp,
            self.min_atk,
            self.min_def,
            self.min_spe,
            self.min_spa,
            self.min_spd,
        ]
    }

    /// Density of (nature, IVs) satisfying the tier among all stream states (doc 3.2).
    pub fn rho(&self) -> f64 {
        let mut r = self.nature_mask().count_ones() as f64 / 25.0;
        for m in self.mins() {
            r *= (32 - m.min(31)) as f64 / 32.0;
        }
        r
    }

    /// Mean on-timer idle frames (naming N + ball j) to reach the best pair of the tier (doc 4.2).
    pub fn mean_idle_frames(&self) -> f64 {
        if self.dials == 1 {
            1.0 / (REACHABLE * self.rho())
        } else {
            (std::f64::consts::PI / (2.0 * REACHABLE * self.rho())).sqrt()
        }
    }

    /// P(a title frame is quiet) for the configured tier; the doc's all-seeds scan (3.4).
    pub fn quiet_p(&self) -> f64 {
        if self.quiet_density >= 0.0 {
            return self.quiet_density;
        }
        // hits <= 2, 3, 4, 6, 8 in 60 rate tests (doc 3.4). Interpolate in log space between rows.
        const ROWS: [(u32, f64); 6] = [
            (0, 0.0005),
            (2, 0.0037),
            (3, 0.0166),
            (4, 0.0498),
            (6, 0.218),
            (8, 0.503),
        ];
        let h = self.quiet_max_hits;
        if let Some(&(_, p)) = ROWS.iter().find(|r| r.0 == h) {
            return p;
        }
        if h > 8 {
            return 1.0;
        }
        // h == 1 or 5, 7: log-interpolate.
        let hi = ROWS.iter().position(|r| r.0 > h).unwrap();
        let (a, b) = (ROWS[hi - 1], ROWS[hi]);
        let f = (h - a.0) as f64 / (b.0 - a.0) as f64;
        (a.1.ln() * (1.0 - f) + b.1.ln() * f).exp()
    }
}

pub fn nature_index(name: &str) -> Option<u8> {
    NATURE_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| i as u8)
}

/// What one attempt's presses did.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttemptDraw {
    pub title_hit: bool,
    pub naming_hit: bool,
    pub ball_hit: bool,
    /// The wild seed is a table seed (title press landed and a quiet tier is configured).
    pub quiet: bool,
    /// Off-timer seconds before New Game: boot, title wait, timer program.
    pub pre_s: f64,
    /// On-timer idle frames spent waiting for the table's frame (when all presses hit).
    pub idle_frames: f64,
    /// Starter made by a hit (true) or an independent random draw (false).
    pub planned: bool,
}

pub fn draw(p: &ManipParams, rng: &mut Rng) -> AttemptDraw {
    let ph = p.press_p();
    let title_hit = rng.chance(ph);
    let naming_hit = rng.chance(ph);
    let ball_hit = rng.chance(ph);
    let mut d = AttemptDraw {
        title_hit,
        naming_hit,
        ball_hit,
        pre_s: p.s_boot_title,
        planned: title_hit && naming_hit && ball_hit && rng.chance(p.p_accept),
        ..Default::default()
    };
    if p.quiet_max_hits > 0 {
        // He waits at the title for a frame whose seed is quiet: first success of a per-frame
        // Bernoulli(q); a table only reaches `quiet_wait_cap` frames before it reboots.
        let q = p.quiet_p().clamp(1e-9, 1.0);
        let cap = p.quiet_wait_cap.max(1) as u64;
        let mut wait = 0.0;
        loop {
            let k = rng.geometric(q) + 1;
            if k <= cap {
                wait += k as f64 / FPS;
                break;
            }
            wait += cap as f64 / FPS + p.s_boot_title;
        }
        d.pre_s += wait;
        d.quiet = title_hit;
    }
    if d.planned {
        // Two dials: Rayleigh, P(cost > x) = exp(-0.933 rho x^2 / 2). One dial: exponential.
        let u = rng.f64().max(1e-12);
        d.idle_frames = if p.dials == 1 {
            -u.ln() / (REACHABLE * p.rho())
        } else {
            (-2.0 * u.ln() / (REACHABLE * p.rho())).sqrt()
        };
    }
    d
}

/// A starter drawn from the reachable-good set: nature uniform over the allowed ones, each IV
/// uniform in `[min, 31]`. IV order HP, Atk, Def, Spe, SpA, SpD.
pub fn sample_good(p: &ManipParams, rng: &mut Rng) -> ([u32; 6], u8) {
    let mask = p.nature_mask();
    let k = rng.below(mask.count_ones());
    let nat = (0..25)
        .filter(|i| mask >> i & 1 == 1)
        .nth(k as usize)
        .unwrap() as u8;
    let mins = p.mins();
    let mut iv = [0u32; 6];
    for (v, &m) in iv.iter_mut().zip(&mins) {
        let lo = m.min(31);
        *v = lo + rng.below(32 - lo);
    }
    (iv, nat)
}

/// Does a starter meet the tier (nature allowed, IVs above the minima)?
pub fn qualifies(p: &ManipParams, ivs: &[u32; 6], nature: u8) -> bool {
    p.nature_mask() >> nature & 1 == 1 && ivs.iter().zip(p.mins()).all(|(&v, m)| v >= m)
}

/// Phase statistics over attempts: reached via `Probe` (one entry per attempt).
#[derive(Default, Clone, Debug)]
pub struct PhaseSample {
    /// Wall frames from the attempt's start to the stats screen, plus the reset on a miss.
    pub wall_frames: f64,
    pub pass: bool,
    pub hits: u8,
    pub quiet: bool,
    pub planned: bool,
}

pub struct PhaseReport {
    pub n: usize,
    pub press_p: f64,
    pub p_pass: f64,
    pub p_planned: f64,
    pub p_lucky: f64,
    pub mean_fail_s: f64,
    pub mean_pass_s: f64,
    pub mean_attempts: f64,
    pub mean_wall_s: f64,
    /// Quantiles (p10, p50, p90) of attempts and wall seconds to a good starter.
    pub attempts_q: [f64; 3],
    pub wall_q: [f64; 3],
    pub quiet_share: f64,
    /// Mean on-timer idle seconds of the tier (model, not sampled).
    pub idle_s: f64,
}

pub fn summarize(samples: &[PhaseSample], press_p: f64, seed: u64) -> PhaseReport {
    let n = samples.len().max(1);
    let pass: Vec<f64> = samples
        .iter()
        .filter(|s| s.pass)
        .map(|s| s.wall_frames / FPS)
        .collect();
    let fail: Vec<f64> = samples
        .iter()
        .filter(|s| !s.pass)
        .map(|s| s.wall_frames / FPS)
        .collect();
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let q = pass.len() as f64 / n as f64;
    let planned = samples.iter().filter(|s| s.planned).count() as f64 / n as f64;
    let lucky = samples.iter().filter(|s| s.pass && !s.planned).count() as f64 / n as f64;
    let quiet_share = {
        let k = samples.iter().filter(|s| s.pass).count().max(1) as f64;
        samples.iter().filter(|s| s.pass && s.quiet).count() as f64 / k
    };
    // Sequences of attempts until the first pass, drawn from the empirical pools.
    let mut rng = Rng::new(seed ^ 0x5EED, 0, 7);
    let mut atts = vec![];
    let mut walls = vec![];
    if q > 0.0 && !fail.is_empty() {
        for _ in 0..20_000 {
            let (mut k, mut w) = (0.0, 0.0);
            loop {
                k += 1.0;
                if rng.chance(q) {
                    w += pass[rng.below(pass.len() as u32) as usize];
                    break;
                }
                w += fail[rng.below(fail.len() as u32) as usize];
            }
            atts.push(k);
            walls.push(w);
        }
    }
    let quant = |v: &mut Vec<f64>| -> [f64; 3] {
        if v.is_empty() {
            return [0.0; 3];
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        [0.1, 0.5, 0.9].map(|p| v[((v.len() - 1) as f64 * p) as usize])
    };
    let mean_fail_s = mean(&fail);
    let mean_pass_s = mean(&pass);
    PhaseReport {
        n,
        press_p,
        p_pass: q,
        p_planned: planned,
        p_lucky: lucky,
        mean_fail_s,
        mean_pass_s,
        mean_attempts: if q > 0.0 { 1.0 / q } else { f64::INFINITY },
        mean_wall_s: if q > 0.0 {
            (1.0 - q) / q * mean_fail_s + mean_pass_s
        } else {
            f64::INFINITY
        },
        attempts_q: quant(&mut atts),
        wall_q: quant(&mut walls),
        quiet_share,
        idle_s: 0.0,
    }
}

impl PhaseReport {
    pub fn text(&self) -> String {
        format!(
            "starter phase (manip), {} attempts: per-press hit {:.3}; P(good starter kept) {:.3} \
             (planned {:.3}, lucky {:.4}); quiet seed on {:.0}% of kept; tier idle {:.1} s on the timer\n\
             \x20 attempt wall to the stats screen: pass {:.0} s, miss {:.0} s (incl. reset)\n\
             \x20 to a good starter: attempts mean {:.2} (p10/50/90 {:.0}/{:.0}/{:.0}), \
             wall mean {:.0} s = {:.1} min (p10/50/90 {:.0}/{:.0}/{:.0} s)",
            self.n,
            self.press_p,
            self.p_pass,
            self.p_planned,
            self.p_lucky,
            self.quiet_share * 100.0,
            self.idle_s,
            self.mean_pass_s,
            self.mean_fail_s,
            self.mean_attempts,
            self.attempts_q[0],
            self.attempts_q[1],
            self.attempts_q[2],
            self.mean_wall_s,
            self.mean_wall_s / 60.0,
            self.wall_q[0],
            self.wall_q[1],
            self.wall_q[2],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densities_match_the_doc_table() {
        let mut p = ManipParams {
            natures: vec!["Modest".into(), "Mild".into(), "Rash".into()],
            min_spe: 0,
            ..Default::default()
        };
        assert!((p.rho() - 0.0075).abs() < 1e-9); // T1
        assert!((p.mean_idle_frames() - 15.0).abs() < 0.5);
        p.min_spe = 20;
        p.min_hp = 20;
        p.min_def = 20;
        assert!((p.rho() - 0.000396).abs() < 2e-6); // T2
        assert!((p.mean_idle_frames() - 65.0).abs() < 1.5);
        p.dials = 1;
        assert!((p.mean_idle_frames() - 2700.0).abs() < 100.0); // doc 4.2: 2710
    }

    #[test]
    fn press_p_table() {
        let p = ManipParams {
            sigma: 0.30,
            mu: 0.0,
            ..Default::default()
        };
        assert!((p.press_p() - 0.904).abs() < 0.002);
        let p = ManipParams {
            sigma: 1.0,
            ..Default::default()
        };
        assert!((p.press_p() - 0.383).abs() < 0.002);
    }

    #[test]
    fn good_samples_qualify() {
        let p = ManipParams::default();
        let mut rng = Rng::new(1, 2, 3);
        for _ in 0..500 {
            let (iv, nat) = sample_good(&p, &mut rng);
            assert!(qualifies(&p, &iv, nat));
        }
    }
}
