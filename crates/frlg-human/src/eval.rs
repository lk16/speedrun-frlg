//! Evaluating a plan: Expected Hours to Target on the ladder, paired comparison, reports.

use crate::data::Dex;
use crate::params::Params;
use crate::route::{build, Seg};
use crate::sim::{attempt, Ctx, SegStats, FPS};
use rayon::prelude::*;

/// The target ladder, seconds: WR (3:25:25) then WR plus 1, 2, 5, 10, 20, 40, 80, 160 minutes.
/// `human-run/PROMPT.md`.
pub const WR_SECONDS: f64 = 12325.0;
pub const LADDER_MINUTES: [u32; 9] = [0, 1, 2, 5, 10, 20, 40, 80, 160];

pub fn rung_name(i: usize) -> String {
    if LADDER_MINUTES[i] == 0 {
        "WR".into()
    } else {
        format!("WR+{}m", LADDER_MINUTES[i])
    }
}

pub fn rung_frames(i: usize) -> u64 {
    ((WR_SECONDS + LADDER_MINUTES[i] as f64 * 60.0) * FPS) as u64
}

pub const BLOCK: u64 = 100;
const PILOT_ATTEMPTS: u64 = 4000;
const PILOT_SEED: u64 = 0xFEED_F00D;

pub struct Plan {
    pub dex: &'static Dex,
    pub params: Params,
    pub segs: Vec<Seg>,
}

impl Plan {
    pub fn load(dex: &'static Dex, text: &str) -> Result<Plan, String> {
        let params = Params::from_plan_text(text)?;
        let segs = build(dex, &params);
        for h in params.hunts.iter() {
            if !segs.iter().any(|s| s.name == h.after) {
                return Err(format!("hunt anchor {} matches no segment", h.after));
            }
        }
        Ok(Plan { dex, params, segs })
    }

    pub fn ctx(&self) -> Ctx<'_> {
        let n = self.segs.len();
        let line_names = match self.params.starter.as_str() {
            "bulbasaur" => ["BULBASAUR", "IVYSAUR", "VENUSAUR"],
            "charmander" => ["CHARMANDER", "CHARMELEON", "CHARIZARD"],
            _ => ["SQUIRTLE", "WARTORTLE", "BLASTOISE"],
        };
        let line = line_names.map(|s| self.dex.id(&format!("SPECIES_{s}")));
        let evo_at = [0, 1].map(|i| {
            self.dex.species[line[i]]
                .evo
                .iter()
                .find(|e| e.0 == "EVO_LEVEL")
                .and_then(|e| e.1.parse().ok())
                .expect("starter evolves by level")
        });
        let mut ctx = Ctx {
            dex: self.dex,
            p: &self.params,
            segs: &self.segs,
            floor_after: vec![0; n],
            mean_after: vec![0.0; n],
            aggr: self
                .segs
                .iter()
                .map(|s| {
                    *self
                        .params
                        .reset_aggr_at
                        .get(&s.name)
                        .unwrap_or(&self.params.reset_aggr)
                })
                .collect(),
            starter_line: line,
            evo_at,
            stab_from: if self.params.starter == "bulbasaur" {
                10
            } else {
                7
            },
        };
        let floors: Vec<f64> = self
            .segs
            .iter()
            .map(|s| {
                if s.off_timer {
                    0.0
                } else {
                    ctx.floor_frames(s)
                }
            })
            .collect();
        let mut acc = 0.0;
        for i in (0..n).rev() {
            ctx.floor_after[i] = acc as u64;
            acc += floors[i];
        }
        // Pilot without resets for the mean remaining time at each split.
        let mut sum = vec![0.0f64; n];
        let mut cnt = 0u64;
        let mut sc = vec![0u64; n];
        for i in 0..PILOT_ATTEMPTS {
            let o = attempt(&ctx, PILOT_SEED, i, None, None, Some(&mut sc), None);
            if o.finished {
                cnt += 1;
                for (acc, c) in sum.iter_mut().zip(&sc) {
                    *acc += (o.run - c) as f64;
                }
            }
        }
        if cnt > 0 {
            for (m, s) in ctx.mean_after.iter_mut().zip(&sum) {
                *m = s / cnt as f64;
            }
        }
        ctx
    }
}

#[derive(Clone, Default, Debug)]
pub struct Totals {
    pub attempts: u64,
    pub finishes: u64,
    pub wall_frames: f64,
    pub run_frames_finished: f64,
}

impl Totals {
    pub fn eht_hours(&self) -> Option<f64> {
        (self.finishes > 0).then(|| self.wall_frames / FPS / 3600.0 / self.finishes as f64)
    }
}

/// Per-block sums so plans can be compared on the same blocks of attempts.
pub fn evaluate_blocks(ctx: &Ctx, seed: u64, attempts: u64, target: u64) -> Vec<(f64, u64, f64)> {
    let blocks = attempts.div_ceil(BLOCK);
    (0..blocks)
        .into_par_iter()
        .map(|b| {
            let (mut wall, mut fin, mut run) = (0.0, 0u64, 0.0);
            for i in (b * BLOCK)..((b + 1) * BLOCK).min(attempts) {
                let o = attempt(ctx, seed, i, Some(target), None, None, None);
                wall += o.wall as f64;
                if o.finished && o.run <= target {
                    fin += 1;
                    run += o.run as f64;
                }
            }
            (wall, fin, run)
        })
        .collect()
}

pub fn totals(blocks: &[(f64, u64, f64)], attempts: u64) -> Totals {
    Totals {
        attempts,
        finishes: blocks.iter().map(|b| b.1).sum(),
        wall_frames: blocks.iter().map(|b| b.0).sum(),
        run_frames_finished: blocks.iter().map(|b| b.2).sum(),
    }
}

pub struct RungResult {
    pub rung: usize,
    pub totals: Totals,
}

/// Tightest rung with at least `min_finishes` finishes, growing the attempt count by 4x up to
/// `cap`. Returns `None` if even the loosest rung falls short.
pub fn find_rung(
    ctx: &Ctx,
    seed: u64,
    r0: u64,
    cap: u64,
    min_finishes: u64,
    from_rung: usize,
) -> Option<RungResult> {
    for rung in from_rung..LADDER_MINUTES.len() {
        let mut r = r0;
        loop {
            let blocks = evaluate_blocks(ctx, seed, r, rung_frames(rung));
            let t = totals(&blocks, r);
            if t.finishes >= min_finishes {
                return Some(RungResult { rung, totals: t });
            }
            if r >= cap {
                break;
            }
            r = (r * 4).min(cap);
        }
    }
    None
}

pub struct Diff {
    pub a: Totals,
    pub b: Totals,
    pub diff_hours: Option<f64>,
    pub ci: Option<(f64, f64)>,
}

/// Paired comparison at one rung on shared seeds: bootstrap over blocks for the CI of
/// `EHT(a) - EHT(b)` (negative means `a` is better).
pub fn compare(a: &Ctx, b: &Ctx, seed: u64, attempts: u64, rung: usize) -> Diff {
    let t = rung_frames(rung);
    let ba = evaluate_blocks(a, seed, attempts, t);
    let bb = evaluate_blocks(b, seed, attempts, t);
    let (ta, tb) = (totals(&ba, attempts), totals(&bb, attempts));
    let eht = |idx: &[usize], v: &[(f64, u64, f64)]| -> Option<f64> {
        let (w, f) = idx
            .iter()
            .fold((0.0, 0u64), |(w, f), &i| (w + v[i].0, f + v[i].1));
        (f > 0).then(|| w / FPS / 3600.0 / f as f64)
    };
    let diff_hours = ta.eht_hours().zip(tb.eht_hours()).map(|(x, y)| x - y);
    let n = ba.len();
    let mut rng = crate::rng::Rng::new(seed ^ 0xB007, 0, 1);
    let mut diffs = vec![];
    for _ in 0..400 {
        let idx: Vec<usize> = (0..n).map(|_| rng.below(n as u32) as usize).collect();
        if let (Some(x), Some(y)) = (eht(&idx, &ba), eht(&idx, &bb)) {
            diffs.push(x - y);
        }
    }
    diffs.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let ci = (diffs.len() >= 200).then(|| {
        (
            diffs[diffs.len() * 25 / 1000],
            diffs[diffs.len() * 975 / 1000],
        )
    });
    Diff {
        a: ta,
        b: tb,
        diff_hours,
        ci,
    }
}

pub struct Report {
    pub segs: Vec<SegStats>,
    pub attempts: u64,
}

/// Per-segment time and reset statistics at a rung.
pub fn report(ctx: &Ctx, seed: u64, attempts: u64, target: u64) -> Report {
    let n = ctx.segs.len();
    let merged = (0..attempts.div_ceil(BLOCK))
        .into_par_iter()
        .map(|b| {
            let mut rec = vec![SegStats::default(); n];
            for i in (b * BLOCK)..((b + 1) * BLOCK).min(attempts) {
                attempt(ctx, seed, i, Some(target), Some(&mut rec), None, None);
            }
            rec
        })
        .reduce(
            || vec![SegStats::default(); n],
            |mut x, y| {
                for i in 0..n {
                    x[i].n += y[i].n;
                    x[i].sum += y[i].sum;
                    x[i].sumsq += y[i].sumsq;
                    x[i].resets += y[i].resets;
                    x[i].whiteouts += y[i].whiteouts;
                    x[i].abandons += y[i].abandons;
                    x[i].level_sum += y[i].level_sum;
                    x[i].caught_sum += y[i].caught_sum;
                }
                x
            },
        );
    Report {
        segs: merged,
        attempts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    static DEX: OnceLock<Dex> = OnceLock::new();

    const PLAN: &str = r#"```json
{"name":"t","starter_min_total":72,"fill":[
 {"site":"MAP_SAFARI_ZONE_CENTER","steps":300},{"site":"MAP_SAFARI_ZONE_NORTH","steps":300},
 {"site":"MAP_SAFARI_ZONE_EAST","steps":300},{"site":"MAP_SAFARI_ZONE_WEST","steps":300},
 {"site":"MAP_ROUTE12","area":"fish","steps":80},{"site":"MAP_ROUTE13","steps":200},
 {"site":"MAP_POWER_PLANT","steps":250},{"site":"MAP_SEAFOAM_ISLANDS_B4F","steps":250},
 {"site":"MAP_CERULEAN_CAVE_B1F","steps":300}]}
```"#;

    #[test]
    fn same_seed_same_score() {
        let dex = DEX.get_or_init(Dex::load);
        let plan = Plan::load(dex, PLAN).unwrap();
        let ctx = plan.ctx();
        let a = totals(&evaluate_blocks(&ctx, 7, 400, rung_frames(8)), 400);
        let b = totals(&evaluate_blocks(&ctx, 7, 400, rung_frames(8)), 400);
        assert_eq!(a.finishes, b.finishes);
        assert_eq!(a.wall_frames, b.wall_frames);
        let c = totals(&evaluate_blocks(&ctx, 8, 400, rung_frames(8)), 400);
        assert_ne!(a.wall_frames, c.wall_frames);
    }

    #[test]
    fn plan_without_json_block_is_an_error() {
        let dex = DEX.get_or_init(Dex::load);
        assert!(Plan::load(dex, "no block here").is_err());
    }
}
