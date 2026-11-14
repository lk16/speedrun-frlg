//! Evaluating a plan: Expected Hours to Target on the ladder, paired comparison, reports.

use crate::data::Dex;
use crate::params::Params;
use crate::route::{build, Seg};
use crate::sim::{attempt, Ctx, Probe, SegStats, FPS};
use rayon::prelude::*;

/// The target ladder, seconds: WR (3:25:25) then WR plus 1, 2, 5, 10, 20, 40, 80, 160 minutes.
/// `human-run/PROMPT.md`.
pub const WR_SECONDS: f64 = 12325.0;
pub const LADDER_MINUTES: [u32; 9] = [0, 1, 2, 5, 10, 20, 40, 80, 160];

/// Index of the segment with this name (`None` for an empty name; a name matching nothing is a plan error).
fn seg_index(segs: &[Seg], name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    Some(
        segs.iter()
            .position(|s| s.name == name)
            .unwrap_or_else(|| panic!("segment {name} matches nothing")),
    )
}

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
            if params.skip_powerplant && h.after == "powerplant-out" {
                continue;
            }
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
        let keep_ids = self
            .params
            .keep_moves
            .iter()
            .map(|m| self.dex.move_id(m))
            .collect();
        let x_from = |s: &str| match s {
            "attack" => crate::battle::XItem::Attack,
            "defend" => crate::battle::XItem::Defend,
            "speed" => crate::battle::XItem::Speed,
            "special" => crate::battle::XItem::Special,
            "dire" => crate::battle::XItem::DireHit,
            other => panic!("unknown x item {other}"),
        };
        let tactics = self
            .params
            .fight_tactics
            .iter()
            .map(|(k, t)| {
                (
                    k.clone(),
                    crate::sim::TacCfg {
                        x: t.x.iter().map(|s| x_from(s)).collect(),
                        heal_below: t.heal_below,
                        sacs: t.sacs,
                        torrent: t.torrent,
                        entry: t.entry,
                        power: t.power,
                    },
                )
            })
            .collect();
        let mut ctx = Ctx {
            dex: self.dex,
            p: &self.params,
            segs: &self.segs,
            keep_ids,
            tactics,
            floor_after: vec![0; n],
            mean_after: vec![0.0; n],
            oak_idx: self
                .segs
                .iter()
                .position(|s| matches!(s.kind, crate::route::Kind::OakCheck))
                .unwrap_or(0),
            fill_base: 0.0,
            fill_slope: 0.0,
            gain_after: vec![0.0; n],
            mean_nofill_after: vec![0.0; n],
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
            power_early_end: seg_index(&self.segs, &self.params.power_early_until),
            torrent_end: seg_index(&self.segs, &self.params.torrent_until),
            no_salvage_end: seg_index(&self.segs, &self.params.no_salvage_until),
            salvage: self
                .segs
                .iter()
                .map(|s| {
                    *self
                        .params
                        .salvage_at
                        .get(&s.name)
                        .unwrap_or(&self.params.salvage_default)
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
        let oak = ctx.oak_idx;
        let mut gain = vec![0.0f64; n];
        // (missing species at Oak, fill frames) per finished attempt, for the linear fit.
        let mut pairs: Vec<(f64, f64)> = vec![];
        for i in 0..PILOT_ATTEMPTS {
            let mut probe = Probe::default();
            let o = attempt(
                &ctx,
                PILOT_SEED,
                i,
                None,
                None,
                Some(&mut sc),
                Some(&mut probe),
            );
            if o.finished {
                cnt += 1;
                for (acc, c) in sum.iter_mut().zip(&sc) {
                    *acc += (o.run - c) as f64;
                }
                if oak > 0 && probe.caught_at.len() > oak {
                    let fill = (sc[oak] - sc[oak - 1]) as f64;
                    pairs.push(((60.0 - probe.oak_count as f64).max(0.0), fill));
                    for (g, c) in gain.iter_mut().zip(&probe.caught_at).take(oak) {
                        *g += (probe.oak_count as f64 - *c as f64).max(0.0);
                    }
                }
            }
        }
        if cnt > 0 {
            for (m, s) in ctx.mean_after.iter_mut().zip(&sum) {
                *m = s / cnt as f64;
            }
            if oak > 0 && pairs.len() > 10 {
                let k = pairs.len() as f64;
                let (mx, my) = (
                    pairs.iter().map(|p| p.0).sum::<f64>() / k,
                    pairs.iter().map(|p| p.1).sum::<f64>() / k,
                );
                let sxx: f64 = pairs.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
                let sxy: f64 = pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
                ctx.fill_slope = if sxx > 0.0 { (sxy / sxx).max(0.0) } else { 0.0 };
                ctx.fill_base = (my - ctx.fill_slope * mx).max(0.0);
                let mean_fill = my;
                for (i, g) in gain.iter().enumerate().take(oak) {
                    ctx.gain_after[i] = g / pairs.len() as f64;
                    ctx.mean_nofill_after[i] = (ctx.mean_after[i] - mean_fill).max(0.0);
                }
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
                    x[i].money_sum += y[i].money_sum;
                    x[i].balls_sum += y[i].balls_sum;
                }
                x
            },
        );
    Report {
        segs: merged,
        attempts,
    }
}

/// The starter phase of `r` attempts (only meaningful for `starter_model = "manip"`).
pub fn phase_report(ctx: &Ctx, seed: u64, r: u64) -> crate::manip::PhaseReport {
    let samples: Vec<crate::manip::PhaseSample> = (0..r)
        .into_par_iter()
        .filter_map(|i| {
            let mut pr = Probe::default();
            attempt(ctx, seed, i, None, None, None, Some(&mut pr));
            pr.phase
        })
        .collect();
    let mut rep = crate::manip::summarize(&samples, ctx.p.manip.press_p(), seed);
    rep.idle_s = ctx.p.manip.mean_idle_frames() / crate::manip::FPS;
    rep
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
    fn every_model_option_builds_and_runs() {
        let dex = DEX.get_or_init(Dex::load);
        let text = PLAN.replace(
            "\"name\":\"t\"",
            "\"name\":\"t\",\"audited_forced\":true,\"safari_model\":true,\"money_model\":true,\"calib_enc\":true,\"calib_battle\":true,\"type_strength\":1.0,\"type_floor\":0.9,\"rod\":\"super\",\"repel_on\":[\"route3\"],\"ball\":\"great\",\"weaken\":true",
        );
        let plan = Plan::load(dex, &text).unwrap();
        let ctx = plan.ctx();
        let t = totals(&evaluate_blocks(&ctx, 3, 200, rung_frames(8)), 200);
        assert_eq!(t.attempts, 200);
        // Another seed gives another score; the same seed the same one.
        let u = totals(&evaluate_blocks(&ctx, 3, 200, rung_frames(8)), 200);
        assert_eq!(t.wall_frames, u.wall_frames);
    }

    #[test]
    fn manip_starter_is_reproducible_and_quiet_seed_runs() {
        let dex = DEX.get_or_init(Dex::load);
        let text = PLAN.replace(
            "\"name\":\"t\"",
            "\"name\":\"t\",\"starter_model\":\"manip\",\"manip\":{\"quiet_max_hits\":4}",
        );
        let plan = Plan::load(dex, &text).unwrap();
        let ctx = plan.ctx();
        let a = totals(&evaluate_blocks(&ctx, 5, 300, rung_frames(8)), 300);
        let b = totals(&evaluate_blocks(&ctx, 5, 300, rung_frames(8)), 300);
        assert_eq!(a.wall_frames, b.wall_frames);
        let r = phase_report(&ctx, 5, 500);
        assert!(r.p_pass > 0.1 && r.p_pass < 0.9);
        assert!(r.mean_fail_s > r.mean_pass_s * 0.5);
    }

    #[test]
    fn fidelity_audit_knobs_add_their_segments_and_only_when_on() {
        let dex = DEX.get_or_init(Dex::load);
        let base = "\"name\":\"t\",\"route_style\":\"wr\",\"audited_forced\":true,\"calib_wr\":true,\"stones\":[\"MOON\"]";
        let off = Plan::load(dex, &PLAN.replace("\"name\":\"t\"", base)).unwrap();
        let on_text = PLAN.replace(
            "\"name\":\"t\"",
            &format!("{base},\"league_transit\":true,\"postgame_calib\":true,\"item_swaps\":true"),
        );
        let on = Plan::load(dex, &on_text).unwrap();
        let has = |p: &Plan, n: &str| p.segs.iter().any(|s| s.name == n);
        for n in [
            "e4-transit-lorelei",
            "e4-transit-bruno-2",
            "mt-ember-ruby-climb",
            "icefall-ice-slides",
            "five-island-resort",
            "warehouse-interior",
            "persim-and-potions",
            "black-glasses-swap",
        ] {
            assert!(has(&on, n), "{n} missing with the knobs on");
            assert!(!has(&off, n), "{n} present with the knobs off");
        }
    }

    #[test]
    fn wr_route_style_orders_the_run_like_the_wr_splits() {
        let dex = DEX.get_or_init(Dex::load);
        let text = PLAN.replace(
            "\"name\":\"t\"",
            "\"name\":\"t\",\"route_style\":\"wr\",\"audited_forced\":true,\"calib_wr\":true,\"stones\":[\"MOON\"]",
        );
        let plan = Plan::load(dex, &text).unwrap();
        let pos = |n: &str| {
            plan.segs
                .iter()
                .position(|s| s.name == n)
                .unwrap_or_else(|| panic!("segment {n} missing"))
        };
        // The WR split order: Brock, Misty, Bill, Bike, Rock Tunnel, Giovanni 1, Snorlax (Surge folded
        // in), Koga, Blaine, Sevii, Sabrina, Erika, Giovanni 3, then all 60 before the League.
        let order = [
            "brock",
            "misty-badge",
            "bill-cottage",
            "bike-redeem",
            "rock-tunnel",
            "scope-and-dig",
            "surge-badge",
            "snorlax",
            "koga-badge",
            "blaine-badge",
            "blaine-badge-and-sevii-scene",
            "sabrina-badge",
            "erika-badge",
            "giovanni-badge",
            "dex-fill",
            "indigo-heal",
            "hall-of-fame-1",
            "icefall-cave",
            "hall-of-fame-2",
        ];
        let at: Vec<usize> = order.iter().map(|n| pos(n)).collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "{at:?}");
        // No post-Hall-of-Fame dex check in this order.
        assert!(!plan.segs.iter().any(|s| s.name == "oak-dex-check"));
        // The pre-League fill runs the plan's `fill` list, so the attempt completes or is abandoned
        // before the League; and a seeded evaluation is reproducible.
        let ctx = plan.ctx();
        let a = totals(&evaluate_blocks(&ctx, 5, 300, rung_frames(8)), 300);
        let b = totals(&evaluate_blocks(&ctx, 5, 300, rung_frames(8)), 300);
        assert_eq!(a.wall_frames, b.wall_frames);
    }

    #[test]
    fn evolutions_need_a_body_and_a_stock() {
        let dex = DEX.get_or_init(Dex::load);
        // Moon Stone evolution of Clefairy (bought at the Game Corner): with no stone in the bag the
        // event does nothing; with a pickup before it Clefable is in the dex afterwards.
        let base = "\"name\":\"t\",\"route_style\":\"wr\",\"audited_forced\":true,\"calib_wr\":true,\"version\":\"LG\",\
            \"specials\":[{\"after\":\"koga-badge\",\"kind\":\"buy\",\"species\":[\"SPECIES_CLEFAIRY\"],\"cost\":0,\"frames\":600,\"level\":12}]";
        let run = |extra: &str| {
            let text = PLAN.replace("\"name\":\"t\"", &format!("{base},{extra}"));
            let plan = Plan::load(dex, &text).unwrap();
            let ctx = plan.ctx();
            let at = plan
                .segs
                .iter()
                .position(|s| s.name == "evo-clefable@koga-badge")
                .expect("evolution segment");
            // The first attempt (same seeds on every call) that survives to the segment.
            for i in 0..3000 {
                let mut pr = Probe {
                    snap_at: Some(at),
                    ..Default::default()
                };
                attempt(&ctx, 11, i, None, None, None, Some(&mut pr));
                if let Some(sn) = pr.snap {
                    return sn.mons;
                }
            }
            panic!("no attempt reached the segment")
        };
        let evo = "\"evolutions\":[{\"after\":\"koga-badge\",\"species\":\"SPECIES_CLEFABLE\",\"via\":\"stone\",\"item\":\"MOON\"}]";
        let none = run(evo);
        assert!(none.iter().any(|m| m.0 == 35), "Clefairy bought");
        assert!(!none.iter().any(|m| m.0 == 36), "no stone, no Clefable");
        let with = run(&format!(
            "{evo},\"elixirs\":[{{\"after\":\"koga-badge\",\"kind\":\"moon\",\"frames\":60}}]"
        ));
        assert!(
            with.iter().any(|m| m.0 == 36),
            "stone in the bag evolves it"
        );
    }

    #[test]
    fn fatal_hazards_end_attempts_and_slow_events_cost_time() {
        let dex = DEX.get_or_init(Dex::load);
        let run = |extra: &str| {
            let text = PLAN.replace("\"name\":\"t\"", &format!("\"name\":\"t\"{extra}"));
            let plan = Plan::load(dex, &text).unwrap();
            let ctx = plan.ctx();
            totals(&evaluate_blocks(&ctx, 9, 300, rung_frames(8)), 300)
        };
        let base = run("");
        // Every attempt dies at Brock: none finishes and the same seeds reproduce the totals.
        let dead = run(",\"fatal\":{\"brock\":1.0}");
        assert_eq!(dead.finishes, 0);
        assert_eq!(
            dead.wall_frames,
            run(",\"fatal\":{\"brock\":1.0}").wall_frames
        );
        // A certain one-minute event at the start of the route adds about a minute to a finish.
        let slow = run(",\"slow\":[{\"seg\":\"to-lab\",\"p\":1.0,\"mean_s\":60}]");
        assert!(slow.wall_frames > base.wall_frames);
        // The scale knob switches the events off.
        let off =
            run(",\"slow\":[{\"seg\":\"to-lab\",\"p\":1.0,\"mean_s\":60}],\"slow_scale\":0.0");
        assert_eq!(off.wall_frames, base.wall_frames);
    }

    #[test]
    fn plan_without_json_block_is_an_error() {
        let dex = DEX.get_or_init(Dex::load);
        assert!(Plan::load(dex, "no block here").is_err());
    }
}
