//! Optimal-stopping reset table. At each split the human resets when the value of continuing,
//! V(split, elapsed), is not positive: V is the expected `lambda * P(finish <= T)` minus the expected
//! wall time still to be spent (resets included, off-timer costs included), with `lambda` the
//! self-consistent expected wall time per finish (EHT) of the policy itself (a renewal-reward fixed
//! point, Dinkelbach iteration). V depends only on the run clock already elapsed at the split, so the
//! policy is "reset if my split time at X is slower than T_X". V is estimated from simulated
//! attempts that play on past the split (no resets beyond a slack above the previous table) by
//! backward value iteration: regress the realised cost to the next split plus the already fitted
//! `max(0, V_next(elapsed_next))` on the elapsed time at this split (binned, monotone).

use crate::eval::{evaluate_blocks, totals};
use crate::sim::{attempt, Ctx, Probe, FPS};
use rayon::prelude::*;

pub struct Cfg {
    pub target: u64,
    pub train: u64,
    pub eval: u64,
    pub iters: u32,
    pub slack_s: f64,
    pub seed: u64,
    /// Fit a second variable (dex count or lead level) per split.
    pub adj: bool,
    /// Keep the cutoffs of the starting table and fit only the second-variable slopes, locally
    /// around each cutoff (`win_s` seconds either side).
    pub slope_only: bool,
    pub win_s: f64,
}

/// A split's rule: reset if `clock > cut + slope * (x - refx)`, x = species caught (kind 1) or lead
/// level (kind 2); kind 0 = elapsed time only. `cut` and `slope` in frames.
#[derive(Clone, Copy, Default)]
pub struct Rule {
    pub cut: f64,
    pub kind: u8,
    pub refx: f64,
    pub slope: f64,
}

struct Traj {
    x: Vec<(u32, u32)>,
    log: Vec<(u32, u64, u64)>,
    wall: u64,
    succ: bool,
}

/// Non-increasing isotonic fit (pool adjacent violators) of weighted points in x order.
fn pav_down(y: &[f64], w: &[f64]) -> Vec<f64> {
    let mut blocks: Vec<(f64, f64, usize)> = vec![];
    for (&yy, &ww) in y.iter().zip(w) {
        blocks.push((yy, ww, 1));
        while blocks.len() >= 2 {
            let n = blocks.len();
            if blocks[n - 2].0 < blocks[n - 1].0 {
                let (b, a) = (blocks[n - 1], blocks[n - 2]);
                let wt = a.1 + b.1;
                blocks.truncate(n - 2);
                blocks.push(((a.0 * a.1 + b.0 * b.1) / wt, wt, a.2 + b.2));
            } else {
                break;
            }
        }
    }
    blocks
        .iter()
        .flat_map(|b| std::iter::repeat_n(b.0, b.2))
        .collect()
}

/// A fitted value function: monotone, piecewise linear in elapsed frames.
#[derive(Clone, Default)]
pub struct Vfn {
    pts: Vec<(f64, f64)>,
}

impl Vfn {
    fn at(&self, e: f64) -> f64 {
        let p = &self.pts;
        match p.len() {
            0 => 0.0,
            1 => p[0].1,
            n => {
                if e <= p[0].0 {
                    return p[0].1;
                }
                if e >= p[n - 1].0 {
                    let (a, b) = (p[n - 2], p[n - 1]);
                    let slope = ((b.1 - a.1) / (b.0 - a.0).max(1.0)).min(0.0);
                    return b.1 + slope * (e - b.0);
                }
                let i = p.partition_point(|q| q.0 <= e);
                let (a, b) = (p[i - 1], p[i]);
                a.1 + (b.1 - a.1) * (e - a.0) / (b.0 - a.0).max(1.0)
            }
        }
    }

    /// Largest elapsed with V > 0, and whether the data were censored (V stayed positive to the end).
    fn cutoff(&self) -> (f64, bool) {
        self.cutoff_at(0.0)
    }

    /// The zero crossing of `V + d`.
    fn cutoff_at(&self, d: f64) -> (f64, bool) {
        let p = &self.pts;
        if p.is_empty() {
            return (f64::NEG_INFINITY, false);
        }
        if p[0].1 + d <= 0.0 {
            return (p[0].0, false);
        }
        for w in p.windows(2) {
            if w[1].1 + d <= 0.0 {
                let f = (w[0].1 + d) / (w[0].1 - w[1].1);
                return (w[0].0 + f * (w[1].0 - w[0].0), false);
            }
        }
        (p[p.len() - 1].0, true)
    }
}

fn fit(mut pts: Vec<(f64, f64)>) -> Vfn {
    if pts.is_empty() {
        return Vfn::default();
    }
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let n = pts.len();
    let bins = (n / 300).clamp(1, 80);
    let size = n.div_ceil(bins);
    let (mut xs, mut ys, mut ws) = (vec![], vec![], vec![]);
    for c in pts.chunks(size) {
        let k = c.len() as f64;
        xs.push(c.iter().map(|p| p.0).sum::<f64>() / k);
        ys.push(c.iter().map(|p| p.1).sum::<f64>() / k);
        ws.push(k);
    }
    let fitted = pav_down(&ys, &ws);
    Vfn {
        pts: xs.into_iter().zip(fitted).collect(),
    }
}

/// Joint fit of `p ~ f(e) + c (x - xbar)` by back-fitting; returns (f, c, xbar, sd of x, t of c).
fn fit_joint(mut pts: Vec<(f64, f64, f64)>) -> (Vfn, f64, f64, f64, f64) {
    let n = pts.len();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let nf = n as f64;
    let xbar = pts.iter().map(|q| q.1).sum::<f64>() / nf;
    let sd = (pts.iter().map(|q| (q.1 - xbar).powi(2)).sum::<f64>() / nf).sqrt();
    let bins = (n / 300).clamp(1, 80);
    let size = n.div_ceil(bins);
    let chunks: Vec<&[(f64, f64, f64)]> = pts.chunks(size).collect();
    let ws: Vec<f64> = chunks.iter().map(|c| c.len() as f64).collect();
    let xs: Vec<f64> = chunks
        .iter()
        .map(|c| c.iter().map(|q| q.0).sum::<f64>() / c.len() as f64)
        .collect();
    let bx: Vec<f64> = chunks
        .iter()
        .map(|c| c.iter().map(|q| q.1).sum::<f64>() / c.len() as f64)
        .collect();
    let (mut c, mut fitted, mut t) = (0.0f64, vec![], 0.0f64);
    for _ in 0..5 {
        let ys: Vec<f64> = chunks
            .iter()
            .map(|ch| ch.iter().map(|q| q.2 - c * (q.1 - xbar)).sum::<f64>() / ch.len() as f64)
            .collect();
        fitted = pav_down(&ys, &ws);
        let (mut sxy, mut sxx) = (0.0, 0.0);
        for (k, ch) in chunks.iter().enumerate() {
            for q in ch.iter() {
                let xr = q.1 - bx[k];
                sxy += (q.2 - fitted[k]) * xr;
                sxx += xr * xr;
            }
        }
        c = if sxx > 0.0 { sxy / sxx } else { 0.0 };
        let mut rss = 0.0;
        for (k, ch) in chunks.iter().enumerate() {
            for q in ch.iter() {
                rss += (q.2 - fitted[k] - c * (q.1 - bx[k])).powi(2);
            }
        }
        let se = if sxx > 0.0 {
            (rss / nf / sxx).sqrt()
        } else {
            f64::INFINITY
        };
        t = c / se;
    }
    (
        Vfn {
            pts: xs.into_iter().zip(fitted).collect(),
        },
        c,
        xbar,
        sd,
        t,
    )
}

/// A fitted split value: elapsed-time function plus an optional linear term in a second variable.
#[derive(Clone, Default)]
struct V2 {
    f: Vfn,
    kind: u8,
    xbar: f64,
    c: f64,
}

impl V2 {
    fn at(&self, e: f64, xs: (u32, u32)) -> f64 {
        let x = match self.kind {
            1 => xs.0 as f64,
            2 => xs.1 as f64,
            _ => return self.f.at(e),
        };
        self.f.at(e) + self.c * (x - self.xbar)
    }
}

type Info = Vec<(usize, usize, bool)>;

/// One backward pass: the rule per segment index for the trajectories and `lambda` (frames).
fn backward(
    ctx: &Ctx,
    trajs: &[Traj],
    lambda: f64,
    target: u64,
    adj: bool,
    start: &[Rule],
    cfg: &Cfg,
) -> (Vec<Rule>, Info) {
    let n = ctx.segs.len();
    let mut by_seg: Vec<Vec<(u32, u32)>> = vec![vec![]; n];
    for (k, t) in trajs.iter().enumerate() {
        for (j, e) in t.log.iter().enumerate() {
            by_seg[e.0 as usize].push((k as u32, j as u32));
        }
    }
    let mut rules = vec![
        Rule {
            cut: f64::NAN,
            ..Default::default()
        };
        n
    ];
    let mut vfn: Vec<V2> = vec![V2::default(); n];
    let mut info = vec![];
    let splits: Vec<usize> = (0..n).filter(|&i| !by_seg[i].is_empty()).collect();
    for &s in splits.iter().rev() {
        let mut pts = Vec::with_capacity(by_seg[s].len());
        let mut xs = Vec::with_capacity(by_seg[s].len());
        for &(k, j) in &by_seg[s] {
            let t = &trajs[k as usize];
            let (_, e, w) = t.log[j as usize];
            let p = match t.log.get(j as usize + 1) {
                Some(&(sn, en, wn)) => {
                    let v = vfn[sn as usize].at(en as f64, t.x[j as usize + 1]).max(0.0);
                    // The next split's rule also applies the floor; a state past it is worth 0.
                    let ok = (en + ctx.floor_after[sn as usize]) <= target;
                    -((wn - w) as f64) + if ok { v } else { 0.0 }
                }
                None => -((t.wall - w) as f64) + if t.succ { lambda } else { 0.0 },
            };
            pts.push((e as f64, p));
            xs.push(t.x[j as usize]);
        }
        let n_pts = pts.len();
        let floor_c = target as f64 - ctx.floor_after[s] as f64;
        let base = fit(pts.clone());
        let (c0, censored) = base.cutoff();
        let c0 = if cfg.slope_only && !start[s].cut.is_nan() {
            start[s].cut
        } else {
            c0
        };
        let mut rule = Rule {
            cut: c0.min(floor_c),
            ..Default::default()
        };
        let mut v2 = V2 {
            f: base,
            ..Default::default()
        };
        if adj {
            // Best of the two candidate variables by t statistic.
            let mut best: Option<(u8, Vfn, f64, f64, f64, f64)> = None;
            for kind in [1u8, 2] {
                let win = if cfg.slope_only {
                    cfg.win_s * FPS
                } else {
                    f64::INFINITY
                };
                let jp: Vec<(f64, f64, f64)> = pts
                    .iter()
                    .zip(&xs)
                    .filter(|(q, _)| (q.0 - c0).abs() <= win)
                    .map(|(q, x)| (q.0, if kind == 1 { x.0 as f64 } else { x.1 as f64 }, q.1))
                    .collect();
                if jp.len() < 600 {
                    continue;
                }
                let (f, c, xbar, sd, t) = fit_joint(jp);
                if best.as_ref().is_none_or(|b| t.abs() > b.5.abs()) {
                    best = Some((kind, f, c, xbar, sd, t));
                }
            }
            if let Some((kind, f, c, xbar, sd, t)) = best {
                if t.abs() >= 3.0 && sd > 0.0 {
                    let refx = xbar.round();
                    let (mut c_ref, cens) = f.cutoff_at(c * (refx - xbar));
                    if cfg.slope_only {
                        // Cutoff at the reference: the starting table's cutoff, moved by the local slope
                        // from the mean x of the window to the (rounded) reference.
                        c_ref = c0 + (c_ref - f.cutoff_at(0.0).0);
                    }
                    let (hi, _) = f.cutoff_at(c * sd);
                    let (lo, _) = f.cutoff_at(-c * sd);
                    // Whole seconds per unit of the second variable (the shift for `+sd` vs `-sd`).
                    let slope = ((hi - lo) / (2.0 * sd) / FPS).round() * FPS;
                    if slope != 0.0 {
                        rule = Rule {
                            cut: c_ref.min(floor_c),
                            kind,
                            refx,
                            slope,
                        };
                        v2 = if cfg.slope_only {
                            V2 {
                                f: v2.f.clone(),
                                kind,
                                xbar,
                                c,
                            }
                        } else {
                            V2 { f, kind, xbar, c }
                        };
                        let _ = cens;
                    }
                }
            }
        }
        rules[s] = rule;
        vfn[s] = v2;
        info.push((s, n_pts, censored));
    }
    info.reverse();
    (rules, info)
}

fn formula_cut(ctx: &Ctx, target: u64) -> Vec<f64> {
    (0..ctx.segs.len())
        .map(|i| {
            let rem = ctx.floor_after[i] as f64
                + ctx.aggr[i] * (ctx.mean_after[i] - ctx.floor_after[i] as f64).max(0.0);
            target as f64 - rem
        })
        .collect()
}

fn set_rules(ctx: &mut Ctx, target: u64, rules: &[Rule], extra: f64) {
    ctx.cut_tables.retain(|(t, _)| *t != target);
    ctx.cut_tables
        .push((target, rules.iter().map(|r| r.cut + extra).collect()));
    ctx.cut_adj.retain(|(t, _)| *t != target);
    ctx.cut_adj.push((
        target,
        rules.iter().map(|r| (r.kind, r.refx, r.slope)).collect(),
    ));
}

#[derive(Clone)]
pub struct Iter {
    pub lambda_h: f64,
    pub finishes: u64,
    /// (segment, cutoff seconds, trajectories, censored)
    pub table: Vec<(String, f64, usize, bool)>,
    /// (segment, kind, reference, seconds per unit) for the splits with a second variable
    pub adj: Vec<(String, u8, f64, f64)>,
}

pub fn run(ctx: &mut Ctx, cfg: &Cfg, mut log: impl FnMut(&Iter)) -> Vec<Iter> {
    let n = ctx.segs.len();
    let split_ids: Vec<usize> = (0..n).filter(|&i| ctx.segs[i].split).collect();
    let existing = ctx
        .cut_tables
        .iter()
        .find(|(t, _)| *t == cfg.target)
        .map(|(_, v)| v.clone());
    let init = existing.unwrap_or_else(|| formula_cut(ctx, cfg.target));
    let mut cur: Vec<Rule> = (0..n)
        .map(|i| Rule {
            cut: if ctx.segs[i].split { init[i] } else { f64::NAN },
            ..Default::default()
        })
        .collect();
    for &i in &split_ids {
        eprintln!("  start {:40} {:8.0} s", ctx.segs[i].name, cur[i].cut / FPS);
    }
    set_rules(ctx, cfg.target, &cur, 0.0);
    let eval = |ctx: &Ctx, seed: u64| {
        let t = totals(&evaluate_blocks(ctx, seed, cfg.eval, cfg.target), cfg.eval);
        (t.eht_hours().unwrap_or(f64::INFINITY), t.finishes)
    };
    // Every iterate is evaluated on the same seed (paired with the starting table), and the best
    // one is returned: at a low finish rate the fit is noisy and a refit can be worse than the
    // table it started from (journal 2026-11-30), so the last iterate is not trusted.
    let eval_seed = cfg.seed ^ 0xE0;
    let (mut lambda_h, fin0) = eval(ctx, eval_seed);
    let table_of = |ctx: &Ctx, cur: &[Rule], info: &[usize]| -> Vec<(String, f64, usize, bool)> {
        info.iter()
            .map(|&s| (ctx.segs[s].name.clone(), cur[s].cut / FPS, 0, false))
            .collect()
    };
    let start_rec = Iter {
        lambda_h,
        finishes: fin0,
        table: table_of(ctx, &cur, &split_ids),
        adj: vec![],
    };
    log(&start_rec);
    let mut best = start_rec;
    let mut out = vec![];
    if cfg.train > 0 && fin0 * cfg.train / cfg.eval.max(1) < 1500 {
        eprintln!(
            "  warning: about {} finishes in the training set; refits below ~1500 are noise-dominated",
            fin0 * cfg.train / cfg.eval.max(1)
        );
    }
    for it in 0..cfg.iters {
        // Train on a policy slightly looser than the current table.
        set_rules(ctx, cfg.target, &cur, cfg.slack_s * FPS);
        let seed = cfg.seed.wrapping_add(1000 * (it as u64 + 1));
        let ctx_ref: &Ctx = ctx;
        let trajs: Vec<Traj> = (0..cfg.train)
            .into_par_iter()
            .map(|i| {
                let mut pr = Probe::default();
                let o = attempt(
                    ctx_ref,
                    seed,
                    i,
                    Some(cfg.target),
                    None,
                    None,
                    Some(&mut pr),
                );
                Traj {
                    x: pr.split_x,
                    log: pr.split_log,
                    wall: o.wall,
                    succ: o.finished && o.run <= cfg.target,
                }
            })
            .collect();
        let lambda = lambda_h * 3600.0 * FPS;
        let (new_rules, info) = backward(ctx, &trajs, lambda, cfg.target, cfg.adj, &cur, cfg);
        drop(trajs);
        // Whole seconds: the table a human memorises.
        let mut nxt = vec![
            Rule {
                cut: f64::NAN,
                ..Default::default()
            };
            n
        ];
        for &i in &split_ids {
            nxt[i] = if new_rules[i].cut.is_nan() {
                cur[i]
            } else {
                Rule {
                    cut: (new_rules[i].cut / FPS).floor() * FPS,
                    ..new_rules[i]
                }
            };
        }
        cur = nxt;
        set_rules(ctx, cfg.target, &cur, 0.0);
        let (l, fin) = eval(ctx, eval_seed);
        let table = info
            .iter()
            .map(|&(s, np, cens)| (ctx.segs[s].name.clone(), cur[s].cut / FPS, np, cens))
            .collect();
        let adj = info
            .iter()
            .filter(|&&(s, _, _)| cur[s].kind != 0)
            .map(|&(s, _, _)| {
                (
                    ctx.segs[s].name.clone(),
                    cur[s].kind,
                    cur[s].refx,
                    cur[s].slope / FPS,
                )
            })
            .collect();
        let rec = Iter {
            lambda_h: l,
            finishes: fin,
            table,
            adj,
        };
        log(&rec);
        lambda_h = l;
        if rec.lambda_h < best.lambda_h {
            best = rec.clone();
        }
        out.push(rec);
    }
    // The best iterate last (the caller prints the last element).
    out.push(best);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pav_is_nonincreasing_and_keeps_the_mean() {
        let y = [5.0, 1.0, 3.0, 2.0, 4.0];
        let w = [1.0; 5];
        let f = pav_down(&y, &w);
        assert!(f.windows(2).all(|p| p[0] >= p[1]));
        assert!((f.iter().sum::<f64>() - y.iter().sum::<f64>()).abs() < 1e-9);
    }

    #[test]
    fn cutoff_is_the_zero_crossing() {
        let v = Vfn {
            pts: vec![(0.0, 10.0), (100.0, 10.0), (200.0, -10.0)],
        };
        assert_eq!(v.cutoff(), (150.0, false));
        assert!(v.at(300.0) < -10.0 || v.at(300.0) <= -10.0);
    }
}
