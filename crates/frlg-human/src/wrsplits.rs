//! Per-split calibration against the world-record run: the model's no-reset cumulative clock at each
//! of the WR's LiveSplit splits next to the real time.
//!
//! The WR times are read off the video overlay (`human-run/video-analysis/wr-3.25.25-main97.md`,
//! external evidence, not a decomp citation). Each split is matched to the first of its candidate
//! segment names the plan has; the clock after that segment is the model's split.

use crate::eval::Plan;
use crate::sim::{attempt, Ctx, Probe, FPS};
use rayon::prelude::*;

/// (WR split name, WR cumulative seconds, WR dex count at the split if shown, the WR carry's level
/// at the split where the fight analysis saw it (-1 unknown), candidate segments).
pub const WR_SPLITS: &[(&str, f64, i32, i32, &[&str])] = &[
    ("Brock", 880.0, 3, 13, &["brock"]),
    ("Misty", 1681.0, 8, 21, &["misty-badge"]),
    ("Bill", 2290.0, -1, -1, &["bill-cottage", "bill-return"]),
    ("Bike", 2774.0, 9, 27, &["bike-redeem", "bike-redeem"]),
    (
        "Rock Tunnel",
        3339.0,
        -1,
        -1,
        &["rock-tunnel-trainers", "rock-tunnel"],
    ),
    ("Giovanni 1", 3896.0, 11, -1, &["scope-and-dig"]),
    ("Snorlax", 4787.0, 14, -1, &["snorlax"]),
    ("Koga", 5502.0, 19, 41, &["koga-badge"]),
    ("Blaine", 6174.0, 27, 42, &["blaine-badge"]),
    (
        "Sevii",
        7087.0,
        35,
        -1,
        &["sevii-return", "blaine-badge-and-sevii-scene"],
    ),
    ("Sabrina", 7914.0, 45, 46, &["sabrina-badge"]),
    // The runner splits "Erika" after the Warden when the Gold Teeth go in after Erika: in the WR video
    // (run time) Erika's Tangela is on screen at 2:14:00 and "A gave the GOLD TEETH to the WARDEN" at
    // 2:15:00, around the 2:14:50 split (`video-analysis/data/wr-scene-B.jsonl`; time base checked
    // against the 3 s frame facts at Koga and Giovanni 3). A plan whose Warden comes earlier
    // (`warden_early`) has no `warden-strength` after Sabrina and falls back to `erika-badge`.
    ("Erika", 8090.0, 46, -1, &["warden-strength", "erika-badge"]),
    ("Giovanni 3", 8633.0, 52, 49, &["giovanni-badge"]),
    ("Start E4R1", 9365.0, 60, 50, &["indigo-heal"]),
    ("Lorelei 1", 9538.0, -1, -1, &["lorelei"]),
    ("Bruno 1", 9663.0, -1, -1, &["bruno"]),
    ("Agatha 1", 9773.0, -1, -1, &["agatha"]),
    ("Lance 1", 9939.0, -1, -1, &["lance"]),
    ("Champ 1", 10130.0, -1, 56, &["hall-of-fame-1"]),
    ("Icefall Cave", 10704.0, -1, -1, &["icefall-cave"]),
    (
        "Rocket Warehouse",
        11268.0,
        -1,
        -1,
        &["warehouse-gideon", "sapphire-and-sail-back"],
    ),
    ("Lorelei 2", 11639.0, -1, -1, &["lorelei-2"]),
    ("Bruno 2", 11763.0, -1, -1, &["bruno-2"]),
    ("Agatha 2", 11876.0, -1, -1, &["agatha-2"]),
    ("Lance 2", 12030.0, -1, -1, &["lance-2"]),
    ("Champ 2", 12252.0, -1, -1, &["champion-2"]),
    ("End", 12325.3, -1, -1, &["hall-of-fame-2"]),
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
    match_splits(&plan.segs)
}

/// For each WR split the first candidate segment that lies after the previous matched split.
fn match_splits(segs: &[crate::route::Seg]) -> Vec<Option<usize>> {
    let mut prev: Option<usize> = None;
    WR_SPLITS
        .iter()
        .map(|(_, _, _, _, cands)| {
            let m = cands.iter().find_map(|c| {
                segs.iter()
                    .enumerate()
                    .position(|(i, s)| s.name == *c && prev.is_none_or(|p| i > p))
            });
            if m.is_some() {
                prev = m;
            }
            m
        })
        .collect()
}

/// Segment indices that end a WR split window (the same matching as `split_indices`).
pub fn window_ends(segs: &[crate::route::Seg]) -> Vec<usize> {
    match_splits(segs).into_iter().flatten().collect()
}

type Row = (
    Vec<u64>,
    Vec<u32>,
    Vec<u32>,
    bool,
    Vec<(String, u32, bool, f64)>,
);

/// Real fight lengths read off the WR video (`human-run/video-analysis/data/calib/v1..v4`):
/// (segment name, player actions ("turns"), duration in seconds from intro to the defeated box).
/// The League actions of Lorelei / Bruno / Agatha / Lance / Champion / Lance 2 were recounted from the bag counts and
/// HP-bar steps at 1-3 s (journal 2026-11-22-10-00; the 4 s estimates had missed 1-3 item turns per fight):
/// 11 -> 13, 7 -> 9, 6 -> 7, 8 -> 10, 10 -> 12, 7.5 -> 9. Durations are unchanged.
pub const WR_FIGHTS: &[(&str, f64, f64)] = &[
    ("brock", 2.0, 31.0),
    ("misty", 6.5, 54.0),
    ("rival-ss-anne", 6.0, 76.0),
    ("koga", 8.0, 72.0),
    ("blaine", 6.0, 80.0),
    ("sabrina", 9.0, 80.0),
    ("giovanni", 6.0, 78.0),
    ("rival-route22", 12.0, 125.0),
    ("lorelei", 13.0, 148.0),
    ("bruno", 9.0, 104.0),
    ("agatha", 7.0, 88.0),
    ("lance", 10.0, 120.0),
    ("champion", 12.0, 154.0),
    ("bruno-2", 5.0, 92.0),
    ("agatha-2", 6.5, 98.0),
    ("lance-2", 9.0, 110.0),
    ("champion-2", 11.5, 196.0),
];

fn is_league(n: &str) -> bool {
    matches!(
        n.trim_end_matches("-2"),
        "lorelei" | "bruno" | "agatha" | "lance" | "champion"
    )
}

pub fn print_table(plan: &Plan, ctx: &Ctx, seed: u64, r: u64, all: bool, within: Option<f64>) {
    let n = plan.segs.len();
    let rows: Vec<Row> = (0..r)
        .into_par_iter()
        .map(|i| {
            let mut sc = vec![u64::MAX; n];
            let mut pr = Probe::default();
            let o = attempt(ctx, seed, i, None, None, Some(&mut sc), Some(&mut pr));
            (sc, pr.caught_at, pr.level_at, o.finished, pr.fights)
        })
        .collect();
    let stat = |i: usize| -> Option<(f64, f64, f64, f64, f64)> {
        let mut v: Vec<f64> = rows
            .iter()
            .filter(|(sc, ..)| sc[i] != u64::MAX)
            .map(|(sc, ..)| sc[i] as f64 / FPS)
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
            .filter(|(sc, c, ..)| sc[i] != u64::MAX && c.len() > i)
            .map(|(_, c, ..)| c[i] as f64)
            .collect();
        v.iter().sum::<f64>() / v.len().max(1) as f64
    };
    let lvl_at = |i: usize| -> f64 {
        let v: Vec<f64> = rows
            .iter()
            .filter(|(sc, _, l, ..)| sc[i] != u64::MAX && l.len() > i)
            .map(|(_, _, l, ..)| l[i] as f64)
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
        "{:<18}{:>9}{:>9}{:>8}{:>8}{:>8}{:>8}{:>5}{:>5}{:>6}{:>5}  segment",
        "split", "WR", "model", "err", "err%", "p10", "p90", "dexM", "dexW", "lvM", "lvW"
    );
    let mut js: Vec<String> = vec![];
    let mut prev_wr = 0.0;
    let mut prev_m = 0.0;
    for (k, ix) in split_indices(plan).into_iter().enumerate() {
        let (name, wr, wdex, wlv, _) = WR_SPLITS[k];
        let Some(i) = ix else {
            continue;
        };
        let Some((m, _sd, p10, p90, _)) = stat(i) else {
            continue;
        };
        println!(
            "{:<18}{:>9}{:>9}{:>+8.0}{:>+7.1}%{:>8}{:>8}{:>5.0}{:>5}{:>6.1}{:>5}  {}   (seg model {} vs WR {})",
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
            lvl_at(i),
            if wlv >= 0 {
                wlv.to_string()
            } else {
                "-".into()
            },
            plan.segs[i].name,
            hms(m - prev_m),
            hms(wr - prev_wr)
        );
        prev_wr = wr;
        prev_m = m;
        js.push(format!(
            "{{\"n\":\"{name}\",\"wr\":{wr:.0},\"m\":{m:.1},\"lv\":{:.2},\"dex\":{:.2}}}",
            lvl_at(i),
            dex_at(i)
        ));
    }
    // Fidelity fit 2026-11-14: spread of each WR window (the no-reset time between two splits),
    // to set against the runner's window-to-window deviations (`tools/fit_attempts.py`).
    println!("\nwindow sd (no-reset, seconds): window  mean  sd  cv%");
    let mut prev_i: Option<usize> = None;
    for (k, ix) in split_indices(plan).into_iter().enumerate() {
        let Some(i) = ix else { continue };
        let v: Vec<f64> = rows
            .iter()
            .filter(|(sc, ..)| sc[i] != u64::MAX && prev_i.is_none_or(|p| sc[p] != u64::MAX))
            .map(|(sc, ..)| (sc[i] - prev_i.map_or(0, |p| sc[p])) as f64 / FPS)
            .collect();
        if v.len() > 1 {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt();
            println!(
                "  WINDOW {:<18}{:>8.0}{:>7.1}{:>6.1}",
                WR_SPLITS[k].0,
                m,
                sd,
                sd / m * 100.0
            );
        }
        prev_i = Some(i);
    }
    // Where the time goes between WR splits: fights (trainers), walking and wild encounters
    // (walks, hunts, grinds), everything else (text, menus, tutors, shops).
    println!("\nmodel time by kind per WR window (fight / walk+wild / text+other), seconds:");
    let class = |k: &crate::route::Kind| {
        use crate::route::Kind::*;
        match k {
            Trainer { .. } | Parties { .. } | Group { .. } => 0,
            Walk { .. } | Wander { .. } | Hunt { .. } | Grind { .. } => 1,
            _ => 2,
        }
    };
    let mut start = 0usize;
    let mut prev_cum = 0.0;
    for (k, ix) in split_indices(plan).into_iter().enumerate() {
        let Some(i) = ix else { continue };
        let mut acc = [0.0f64; 3];
        for j in start..=i {
            if let Some((m, ..)) = stat(j) {
                acc[class(&plan.segs[j].kind)] += m - prev_cum;
                prev_cum = m;
            }
        }
        start = i + 1;
        println!(
            "{:<18} fight {:>5.0}  walk/wild {:>5.0}  text/other {:>5.0}",
            WR_SPLITS[k].0, acc[0], acc[1], acc[2]
        );
    }
    match within {
        Some(t) => println!(
            "\nfight length: model player actions (win rate) vs the WR video's, attempts finishing within {}:",
            hms(t)
        ),
        None => println!("\nfight length: model player actions (win rate) vs the WR video's:"),
    }
    let mut fit = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (name, wt, ws) in WR_FIGHTS {
        // `within`: only attempts that finished in that many seconds (the WR video is one successful run, so
        // its fights are conditioned on success: luck in the fights is part of why it finished).
        let v: Vec<&(String, u32, bool, f64)> = rows
            .iter()
            .filter(|r| {
                within.is_none_or(|t| {
                    r.3 && r
                        .0
                        .iter()
                        .filter(|&&x| x != u64::MAX)
                        .max()
                        .is_some_and(|&x| x as f64 / FPS <= t)
                })
            })
            .flat_map(|r| r.4.iter())
            .filter(|f| f.0 == *name)
            .collect();
        if v.is_empty() {
            continue;
        }
        let n = v.len() as f64;
        let won = v.iter().filter(|f| f.2).count() as f64 / n;
        let turns = v.iter().map(|f| f.1 as f64).sum::<f64>() / n;
        // Segment duration of the fight (intro to outro, level-ups included), from the cumulative clocks.
        let secs = plan
            .segs
            .iter()
            .position(|sg| sg.name == *name)
            .and_then(|i| {
                let d: Vec<f64> = rows
                    .iter()
                    .filter(|r| r.0[i] != u64::MAX && i > 0 && r.0[i - 1] != u64::MAX)
                    .map(|r| (r.0[i] - r.0[i - 1]) as f64 / FPS)
                    .collect();
                (!d.is_empty()).then(|| d.iter().sum::<f64>() / d.len() as f64)
            })
            .unwrap_or(0.0);
        let core = {
            let c: Vec<f64> = v.iter().filter(|f| f.2).map(|f| f.3 / FPS).collect();
            c.iter().sum::<f64>() / c.len().max(1) as f64
        };
        if turns > 0.0 {
            fit.0 += (turns / wt).ln().abs();
            fit.1 += (core / ws).ln().abs();
            fit.2 += 1.0;
            if !is_league(name) {
                fit.3 += (turns / wt).ln().abs();
            }
        }
        println!(
            "  {:<16} model {:>5.1} actions core {:>4.0} s (+outro/level-ups {:>4.0} s) (won {:>4.0}%, n={:>5.0}) | WR {:>4.1} actions {:>4.0} s",
            name,
            turns,
            core,
            secs,
            won * 100.0,
            n,
            wt,
            ws
        );
    }
    if fit.2 > 0.0 {
        println!(
            "fit: mean |log(model/WR)| over {} fights: actions {:.3}, core seconds {:.3}; actions without the League fights {:.3}",
            fit.2, fit.0 / fit.2, fit.1 / fit.2, fit.3 / 7.0
        );
    }
    // Survival: of the attempts that passed the starter gate, how many reach each early split.
    let pass_i = plan
        .segs
        .iter()
        .position(|sg| sg.name == "rival-lab")
        .unwrap_or(0);
    let passed = rows
        .iter()
        .filter(|(sc, ..)| sc[pass_i] != u64::MAX)
        .count() as f64;
    if passed > 0.0 {
        let mut prev = 1.0;
        print!("reach (of starter passers):");
        for (k, ix) in split_indices(plan).into_iter().enumerate().take(14) {
            let Some(i) = ix else { continue };
            let r = rows.iter().filter(|(sc, ..)| sc[i] != u64::MAX).count() as f64 / passed;
            print!(
                " {} {:.1}% (step {:.1}%)",
                WR_SPLITS[k].0,
                r * 100.0,
                r / prev * 100.0
            );
            prev = r.max(1e-9);
        }
        println!();
    }
    let mut runs: Vec<f64> = rows
        .iter()
        .zip(0..)
        .filter(|((_, _, _, fin, _), _)| *fin)
        .map(|((sc, ..), _)| *sc.iter().filter(|&&x| x != u64::MAX).max().unwrap() as f64 / FPS)
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let fin = runs.len();
    println!(
        "JSON {{\"finish\":{fin},\"attempts\":{r},\"splits\":[{}]}}",
        js.join(",")
    );
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
