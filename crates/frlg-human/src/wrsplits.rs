//! Per-split calibration against the world-record run: the model's no-reset cumulative clock at each
//! of the WR's LiveSplit splits next to the real time.
//!
//! The WR times are read off the video overlay (`human-run/video-analysis/wr-3.25.25-main97.md`,
//! external evidence, not a decomp citation). Each split is matched to the first of its candidate
//! segment names the plan has; the clock after that segment is the model's split.

use crate::eval::Plan;
use crate::sim::{attempt, Ctx, Probe, FPS};
use rayon::prelude::*;

/// (WR split name, WR cumulative seconds, WR dex count at the split if shown, candidate segments).
pub const WR_SPLITS: &[(&str, f64, i32, &[&str])] = &[
    ("Brock", 880.0, 3, &["brock"]),
    ("Misty", 1681.0, 8, &["misty-badge"]),
    ("Bill", 2290.0, -1, &["bill-cottage", "bill-return"]),
    ("Bike", 2774.0, 9, &["bike-redeem", "bike-redeem"]),
    (
        "Rock Tunnel",
        3339.0,
        -1,
        &["rock-tunnel-trainers", "rock-tunnel"],
    ),
    ("Giovanni 1", 3896.0, 11, &["scope-and-dig"]),
    ("Snorlax", 4787.0, 14, &["snorlax"]),
    ("Koga", 5502.0, 19, &["koga-badge"]),
    ("Blaine", 6174.0, 27, &["blaine-badge"]),
    (
        "Sevii",
        7087.0,
        35,
        &["sevii-return", "blaine-badge-and-sevii-scene"],
    ),
    ("Sabrina", 7914.0, 45, &["sabrina-badge"]),
    ("Erika", 8090.0, 46, &["erika-badge"]),
    ("Giovanni 3", 8633.0, 52, &["giovanni-badge"]),
    ("Start E4R1", 9365.0, 60, &["indigo-heal"]),
    ("Lorelei 1", 9538.0, -1, &["lorelei"]),
    ("Bruno 1", 9663.0, -1, &["bruno"]),
    ("Agatha 1", 9773.0, -1, &["agatha"]),
    ("Lance 1", 9939.0, -1, &["lance"]),
    ("Champ 1", 10130.0, -1, &["hall-of-fame-1"]),
    ("Icefall Cave", 10704.0, -1, &["icefall-cave"]),
    (
        "Rocket Warehouse",
        11268.0,
        -1,
        &["warehouse-gideon", "sapphire-and-sail-back"],
    ),
    ("Lorelei 2", 11639.0, -1, &["lorelei-2"]),
    ("Bruno 2", 11763.0, -1, &["bruno-2"]),
    ("Agatha 2", 11876.0, -1, &["agatha-2"]),
    ("Lance 2", 12030.0, -1, &["lance-2"]),
    ("Champ 2", 12252.0, -1, &["champion-2"]),
    ("End", 12325.3, -1, &["hall-of-fame-2"]),
];

fn hms(s: f64) -> String {
    let neg = s < 0.0;
    let s = s.abs() as i64;
    format!(
        "{}{}:{:02}:{:02}",
        if neg { "-" } else { "" },
        s / 3600,
        s / 60 % 60,
        s % 60
    )
}

/// Segment index for each WR split (None when the plan has no matching segment).
pub fn split_indices(plan: &Plan) -> Vec<Option<usize>> {
    WR_SPLITS
        .iter()
        .map(|(_, _, _, cands)| {
            cands
                .iter()
                .find_map(|c| plan.segs.iter().position(|s| s.name == *c))
        })
        .collect()
}

pub fn print_table(plan: &Plan, ctx: &Ctx, seed: u64, r: u64, all: bool) {
    let n = plan.segs.len();
    let rows: Vec<(Vec<u64>, Vec<u32>, bool)> = (0..r)
        .into_par_iter()
        .map(|i| {
            let mut sc = vec![u64::MAX; n];
            let mut pr = Probe::default();
            let o = attempt(ctx, seed, i, None, None, Some(&mut sc), Some(&mut pr));
            (sc, pr.caught_at, o.finished)
        })
        .collect();
    let stat = |i: usize| -> Option<(f64, f64, f64, f64, f64)> {
        let mut v: Vec<f64> = rows
            .iter()
            .filter(|(sc, _, _)| sc[i] != u64::MAX)
            .map(|(sc, _, _)| sc[i] as f64 / FPS)
            .collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let m = v.iter().sum::<f64>() / v.len() as f64;
        let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt();
        let q = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
        Some((m, sd, q(0.1), q(0.9), v.len() as f64))
    };
    let dex_at = |i: usize| -> f64 {
        let v: Vec<f64> = rows
            .iter()
            .filter(|(sc, c, _)| sc[i] != u64::MAX && c.len() > i)
            .map(|(_, c, _)| c[i] as f64)
            .collect();
        v.iter().sum::<f64>() / v.len().max(1) as f64
    };
    if all {
        let mut prev = 0.0;
        println!("{:<34}{:>10}{:>10}{:>7}", "segment", "cum", "seg", "dex");
        for (i, s) in plan.segs.iter().enumerate() {
            if let Some((m, ..)) = stat(i) {
                println!(
                    "{:<34}{:>10}{:>10}{:>7.1}",
                    s.name,
                    hms(m),
                    hms(m - prev),
                    dex_at(i)
                );
                prev = m;
            }
        }
        println!();
    }
    println!(
        "{:<18}{:>9}{:>9}{:>8}{:>8}{:>8}{:>8}{:>5}{:>5}  segment",
        "split", "WR", "model", "err", "err%", "p10", "p90", "dexM", "dexW"
    );
    let mut prev_wr = 0.0;
    let mut prev_m = 0.0;
    for (k, ix) in split_indices(plan).into_iter().enumerate() {
        let (name, wr, wdex, _) = WR_SPLITS[k];
        let Some(i) = ix else {
            continue;
        };
        let Some((m, _sd, p10, p90, _)) = stat(i) else {
            continue;
        };
        println!(
            "{:<18}{:>9}{:>9}{:>+8.0}{:>+7.1}%{:>8}{:>8}{:>5.0}{:>5}  {}   (seg model {} vs WR {})",
            name,
            hms(wr),
            hms(m),
            m - wr,
            (m - wr) / wr * 100.0,
            hms(p10),
            hms(p90),
            dex_at(i),
            if wdex >= 0 {
                wdex.to_string()
            } else {
                "-".into()
            },
            plan.segs[i].name,
            hms(m - prev_m),
            hms(wr - prev_wr)
        );
        prev_wr = wr;
        prev_m = m;
    }
    let mut runs: Vec<f64> = rows
        .iter()
        .zip(0..)
        .filter(|((_, _, fin), _)| *fin)
        .map(|((sc, ..), _)| *sc.iter().filter(|&&x| x != u64::MAX).max().unwrap() as f64 / FPS)
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if !runs.is_empty() {
        let q = |p: f64| hms(runs[((runs.len() - 1) as f64 * p) as usize]);
        println!(
            "no-reset finishes {}/{}: min {} p10 {} p50 {} p90 {} max {} (WR 3:25:25)",
            runs.len(),
            r,
            q(0.0),
            q(0.1),
            q(0.5),
            q(0.9),
            q(1.0)
        );
    }
}
