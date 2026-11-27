//! Per-segment no-reset cost distribution of a plan (R3e calibration, `human-run/journal/*-r3e-calibration.md`).
//!
//!     cargo run --release --example seg_dist -- <plan.md> [R] [seed] [--fights]
//!
//! One row per segment: kind, deterministic floor (`Ctx::floor_frames`), and over R no-reset attempts
//! the share that reached it, the mean / p10 / p50 / p90 seconds spent in it, the mean wild encounters
//! met and the mean seconds those took (`Probe::seg_enc`), and the p50 run clock after it (the
//! number the played controller logs as "model p50"). `--fights` adds per trainer segment the mean
//! player actions and the win share (`Probe::fights`).
use std::collections::HashMap;

use frlg_human::data::Dex;
use frlg_human::eval::Plan;
use frlg_human::route::Kind;
use frlg_human::sim::{attempt, Probe, FPS};
use rayon::prelude::*;

fn q(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() - 1) as f64 * p) as usize]
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let text = std::fs::read_to_string(&a[0]).expect("plan");
    let r: u64 = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(4000);
    let seed: u64 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(1);
    let fights = a.iter().any(|x| x == "--fights");
    let dex: &'static Dex = Box::leak(Box::new(Dex::load()));
    let plan = Plan::load(dex, &text).expect("plan");
    let ctx = plan.ctx();
    let n = plan.segs.len();
    type Row = (
        Vec<u64>,
        Vec<(u64, u32, u64)>,
        Vec<(String, u32, bool, f64, u32)>,
    );
    let rows: Vec<Row> = (0..r)
        .into_par_iter()
        .map(|i| {
            let mut sc = vec![u64::MAX; n];
            let mut pr = Probe::default();
            attempt(&ctx, seed, i, None, None, Some(&mut sc), Some(&mut pr));
            (sc, pr.seg_enc, pr.fights)
        })
        .collect();
    println!(
        "{:<38} {:<8} {:>7} {:>6} {:>7} {:>6} {:>6} {:>6} {:>5} {:>6} {:>7}",
        "segment",
        "kind",
        "floor_s",
        "reach",
        "mean_s",
        "p10",
        "p50",
        "p90",
        "enc",
        "enc_s",
        "clk_p50"
    );
    let mut fstat: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    if fights {
        for (_, _, fl) in &rows {
            for (name, acts, won, _, _) in fl {
                let e = fstat.entry(name.clone()).or_default();
                e.0 += 1.0;
                e.1 += *acts as f64;
                e.2 += *won as u8 as f64;
                e.3 += (*acts as f64) * (*acts as f64);
            }
        }
    }
    for (i, s) in plan.segs.iter().enumerate() {
        let kind = match &s.kind {
            Kind::Text { .. } => "text",
            Kind::Walk { .. } => "walk",
            Kind::Wander { .. } => "wander",
            Kind::Trainer { .. } => "trainer",
            Kind::Parties { .. } => "parties",
            Kind::Group { .. } => "group",
            Kind::Hunt { .. } => "hunt",
            Kind::Grind { .. } => "grind",
            Kind::Mart { .. } => "mart",
            Kind::Teach { .. } => "teach",
            Kind::Special { .. } => "special",
            Kind::Elixir { .. } => "elixir",
            Kind::XBuy { .. } => "xbuy",
            Kind::Evolve { .. } => "evolve",
            _ => "other",
        };
        let mut dur = vec![];
        let mut clk = vec![];
        let (mut enc, mut encf, mut m) = (0.0f64, 0.0f64, 0.0f64);
        for (sc, se, _) in &rows {
            if sc[i] == u64::MAX {
                continue;
            }
            clk.push(sc[i] as f64 / FPS);
            if let Some(&(d, en, ef)) = se.get(i) {
                dur.push(d as f64 / FPS);
                enc += en as f64;
                encf += ef as f64 / FPS;
                m += 1.0;
            }
        }
        let mean = dur.iter().sum::<f64>() / m.max(1.0);
        let reach = clk.len() as f64 / r as f64;
        print!(
            "{:<38} {:<8} {:>7.1} {:>6.3} {:>7.1} {:>6.1} {:>6.1} {:>6.1} {:>5.2} {:>6.1} {:>7.0}",
            s.name,
            kind,
            ctx.floor_frames(s) / FPS,
            reach,
            mean,
            q(&mut dur.clone(), 0.1),
            q(&mut dur.clone(), 0.5),
            q(&mut dur, 0.9),
            enc / m.max(1.0),
            encf / m.max(1.0),
            q(&mut clk, 0.5),
        );
        if let Some(f) = fstat.get(&s.name) {
            print!(
                "  fights {:.0} acts {:.2} sd {:.2} won {:.3}",
                f.0,
                f.1 / f.0,
                (f.3 / f.0 - (f.1 / f.0).powi(2)).max(0.0).sqrt(),
                f.2 / f.0
            );
        }
        println!();
    }
}
