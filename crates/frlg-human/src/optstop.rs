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
}

struct Traj {
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
        let p = &self.pts;
        if p.is_empty() {
            return (f64::NEG_INFINITY, false);
        }
        if p[0].1 <= 0.0 {
            return (p[0].0, false);
        }
        for w in p.windows(2) {
            if w[1].1 <= 0.0 {
                let f = w[0].1 / (w[0].1 - w[1].1);
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

type Info = Vec<(usize, usize, bool)>;

/// One backward pass: the cutoff (frames) per segment index for the trajectories and `lambda` (frames).
fn backward(ctx: &Ctx, trajs: &[Traj], lambda: f64, target: u64) -> (Vec<f64>, Info) {
    let n = ctx.segs.len();
    let mut by_seg: Vec<Vec<(u32, u32)>> = vec![vec![]; n];
    for (k, t) in trajs.iter().enumerate() {
        for (j, e) in t.log.iter().enumerate() {
            by_seg[e.0 as usize].push((k as u32, j as u32));
        }
    }
    let mut cut = vec![f64::NAN; n];
    let mut vfn: Vec<Vfn> = vec![Vfn::default(); n];
    let mut info = vec![];
    let splits: Vec<usize> = (0..n).filter(|&i| !by_seg[i].is_empty()).collect();
    for &s in splits.iter().rev() {
        let mut pts = Vec::with_capacity(by_seg[s].len());
        for &(k, j) in &by_seg[s] {
            let t = &trajs[k as usize];
            let (_, e, w) = t.log[j as usize];
            let p = match t.log.get(j as usize + 1) {
                Some(&(sn, en, wn)) => {
                    let v = vfn[sn as usize].at(en as f64).max(0.0);
                    // The next split's rule also applies the floor; a state past it is worth 0.
                    let ok = (en + ctx.floor_after[sn as usize]) <= target;
                    -((wn - w) as f64) + if ok { v } else { 0.0 }
                }
                None => -((t.wall - w) as f64) + if t.succ { lambda } else { 0.0 },
            };
            pts.push((e as f64, p));
        }
        let n_pts = pts.len();
        let f = fit(pts);
        let (c, censored) = f.cutoff();
        let floor_c = target as f64 - ctx.floor_after[s] as f64;
        cut[s] = c.min(floor_c);
        vfn[s] = f;
        info.push((s, n_pts, censored));
    }
    info.reverse();
    (cut, info)
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

fn set_table(ctx: &mut Ctx, target: u64, cut: Vec<f64>) {
    ctx.cut_tables.retain(|(t, _)| *t != target);
    ctx.cut_tables.push((target, cut));
}

pub struct Iter {
    pub lambda_h: f64,
    pub finishes: u64,
    /// (segment, cutoff seconds, trajectories, censored)
    pub table: Vec<(String, f64, usize, bool)>,
}

pub fn run(ctx: &mut Ctx, cfg: &Cfg, mut log: impl FnMut(&Iter)) -> Vec<Iter> {
    let n = ctx.segs.len();
    let split_ids: Vec<usize> = (0..n).filter(|&i| ctx.segs[i].split).collect();
    let mut cur = formula_cut(ctx, cfg.target);
    for (i, c) in cur.iter_mut().enumerate() {
        if !ctx.segs[i].split {
            *c = f64::NAN;
        }
    }
    for &i in &split_ids {
        eprintln!("  formula {:40} {:8.0} s", ctx.segs[i].name, cur[i] / FPS);
    }
    set_table(ctx, cfg.target, cur.clone());
    let eval = |ctx: &Ctx, seed: u64| {
        let t = totals(&evaluate_blocks(ctx, seed, cfg.eval, cfg.target), cfg.eval);
        (t.eht_hours().unwrap_or(f64::INFINITY), t.finishes)
    };
    let (mut lambda_h, fin0) = eval(ctx, cfg.seed ^ 0xE0);
    log(&Iter {
        lambda_h,
        finishes: fin0,
        table: vec![],
    });
    let mut out = vec![];
    for it in 0..cfg.iters {
        // Train on a policy slightly looser than the current table.
        let slack = cfg.slack_s * FPS;
        let train: Vec<f64> = cur.iter().map(|c| c + slack).collect();
        set_table(ctx, cfg.target, train);
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
                    log: pr.split_log,
                    wall: o.wall,
                    succ: o.finished && o.run <= cfg.target,
                }
            })
            .collect();
        let lambda = lambda_h * 3600.0 * FPS;
        let (new_cut, info) = backward(ctx, &trajs, lambda, cfg.target);
        drop(trajs);
        // Whole seconds: the table a human memorises.
        let mut cutv = vec![f64::NAN; n];
        for &i in &split_ids {
            cutv[i] = if new_cut[i].is_nan() {
                cur[i]
            } else {
                (new_cut[i] / FPS).floor() * FPS
            };
        }
        cur = cutv;
        set_table(ctx, cfg.target, cur.clone());
        let (l, fin) = eval(ctx, seed ^ 0xE7A1);
        let table = info
            .iter()
            .map(|&(s, np, cens)| (ctx.segs[s].name.clone(), cur[s] / FPS, np, cens))
            .collect();
        let rec = Iter {
            lambda_h: l,
            finishes: fin,
            table,
        };
        log(&rec);
        lambda_h = l;
        out.push(rec);
    }
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
