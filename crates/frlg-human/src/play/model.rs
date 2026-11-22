//! The model's numbers the controller logs against: per-segment no-reset clock quantiles (split p50
//! / p10) and the reset verdict at a split (`sim.rs` reset check: `elapsed + best_case_remaining`
//! against T, plus the plan's optimal-stopping table, `optstop.rs`).
use rayon::prelude::*;
use serde_json::json;

use crate::eval::{rung_frames, rung_name, Plan, LADDER_MINUTES};
use crate::sim::{attempt, Ctx, FPS};

pub struct Model {
    /// Segment names, in plan order.
    pub names: Vec<String>,
    /// No-reset run clock after each segment, seconds: (p10, p50, p90); NaN when never reached.
    pub q: Vec<(f64, f64, f64)>,
    /// Best-case frames from just after segment i to the finish (`Ctx::floor_after`).
    pub floor_after: Vec<u64>,
    pub mean_after: Vec<f64>,
    pub aggr: Vec<f64>,
    /// (target frames, per-segment optstop cutoff in frames, NaN = none).
    pub cuts: Vec<(u64, Vec<f64>)>,
    pub attempts: u64,
}

impl Model {
    pub fn build(plan: &Plan, seed: u64, attempts: u64) -> Self {
        let ctx: Ctx = plan.ctx();
        let n = plan.segs.len();
        let rows: Vec<Vec<u64>> = (0..attempts)
            .into_par_iter()
            .map(|i| {
                let mut sc = vec![u64::MAX; n];
                attempt(&ctx, seed, i, None, None, Some(&mut sc), None);
                sc
            })
            .collect();
        let q = (0..n)
            .map(|i| {
                let mut v: Vec<f64> = rows
                    .iter()
                    .filter(|r| r[i] != u64::MAX)
                    .map(|r| r[i] as f64 / FPS)
                    .collect();
                if v.is_empty() {
                    return (f64::NAN, f64::NAN, f64::NAN);
                }
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let at = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
                (at(0.1), at(0.5), at(0.9))
            })
            .collect();
        Model {
            names: plan.segs.iter().map(|s| s.name.clone()).collect(),
            q,
            floor_after: ctx.floor_after.clone(),
            mean_after: ctx.mean_after.clone(),
            aggr: ctx.aggr.clone(),
            cuts: ctx.cut_tables.clone(),
            attempts,
        }
    }

    pub fn index(&self, seg: &str) -> Option<usize> {
        self.names.iter().position(|s| s == seg)
    }

    /// Model p50 (seconds) of the run clock after segment `seg`.
    pub fn p50(&self, seg: &str) -> Option<f64> {
        self.index(seg)
            .map(|i| self.q[i].1)
            .filter(|v| v.is_finite())
    }

    /// The reset verdict the plan would give at the end of segment `seg` with `elapsed` seconds on
    /// the run timer, for every rung that has an optstop table plus the WR floor. Returns
    /// (sentence, nums).
    pub fn verdict(&self, seg: &str, elapsed: f64) -> Option<(String, serde_json::Value)> {
        let i = self.index(seg)?;
        let e = elapsed * FPS;
        let floor = self.floor_after[i] as f64;
        let mut rows = vec![];
        let mut parts = vec![];
        let mut targets: Vec<(u64, Option<f64>)> =
            self.cuts.iter().map(|(t, c)| (*t, Some(c[i]))).collect();
        let wr = rung_frames(0);
        if !targets.iter().any(|(t, _)| *t == wr) {
            targets.push((wr, None));
        }
        targets.sort_by_key(|t| t.0);
        for (t, cut) in targets {
            let cut = cut.filter(|c| !c.is_nan());
            let remaining = floor
                + match cut {
                    Some(_) => 0.0,
                    None => self.aggr[i] * (self.mean_after[i] - floor).max(0.0),
                };
            let over_cut = cut.is_some_and(|c| e > c);
            let reset = over_cut || e + remaining > t as f64;
            let rung = (0..LADDER_MINUTES.len())
                .find(|&r| rung_frames(r) == t)
                .map(rung_name)
                .unwrap_or_else(|| format!("{:.0}s", t as f64 / FPS));
            parts.push(format!(
                "{rung}: {}{}",
                if reset { "RESET" } else { "go on" },
                match cut {
                    Some(c) => format!(" (cut {:.0}s)", c / FPS),
                    None => String::new(),
                }
            ));
            rows.push(json!({
                "T": rung,
                "T_s": t as f64 / FPS,
                "cut_s": cut.map(|c| c / FPS),
                "best_case_remaining_s": remaining / FPS,
                "projected_s": (e + remaining) / FPS,
                "reset": reset,
            }));
        }
        let text = format!(
            "reset verdict at {seg}: {:.0}s + best case {:.0}s = {:.0}s; {}",
            elapsed,
            floor / FPS,
            elapsed + floor / FPS,
            parts.join(", ")
        );
        Some((
            text,
            json!({"seg": seg, "elapsed": elapsed, "floor_remaining_s": floor / FPS, "rungs": rows}),
        ))
    }
}
