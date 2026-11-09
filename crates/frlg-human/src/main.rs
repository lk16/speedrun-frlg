use frlg_human::data::Dex;
use frlg_human::eval::*;
use frlg_human::route::Src;
use frlg_human::sim::{attempt, FPS};
use std::sync::OnceLock;

static DEX: OnceLock<Dex> = OnceLock::new();

fn dex() -> &'static Dex {
    DEX.get_or_init(Dex::load)
}

fn arg<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> T {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn rung_arg(args: &[String]) -> usize {
    let v: String = arg(args, "--rung", "0".to_string());
    (0..LADDER_MINUTES.len())
        .find(|&i| rung_name(i) == v || i.to_string() == v)
        .unwrap_or_else(|| panic!("unknown rung {v}"))
}

fn load(path: &str) -> Plan {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    Plan::load(dex(), &text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn hms(frames: f64) -> String {
    let s = (frames / FPS) as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

fn usage() -> ! {
    eprintln!(
        "usage: frlg-human <command> ...\n\
         \x20 ladder  <plan> [--R n] [--cap n] [--seed s]      tightest rung with >=50 finishes and its EHT\n\
         \x20 eval    <plan> --rung <i|WR|WR+10m> [--R n]       EHT at one rung\n\
         \x20 compare <a> <b> --rung <r> [--R n]                paired difference EHT(a)-EHT(b) with 95% CI\n\
         \x20 report  <plan> --rung <r> [--R n]                 per-segment mean, spread, resets, time by source\n\
         \x20 dist    <plan> [--R n]                            run-time distribution with no resets\n\
         \x20 starter [--min n]                                 level-5 stat-total distribution of a Squirtle"
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = arg(&args, "--seed", 1);
    match args.first().map(String::as_str) {
        Some("ladder") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let r: u64 = arg(&args, "--R", 10_000);
            let cap: u64 = arg(&args, "--cap", 400_000);
            match find_rung(&ctx, seed, r, cap, 50, 0) {
                Some(res) => {
                    let eht = res.totals.eht_hours().unwrap();
                    println!(
                        "{{\"plan\":\"{}\",\"rung\":\"{}\",\"eht_hours\":{:.3},\"R\":{},\"finishes\":{},\"seed\":{}}}",
                        plan.params.name,
                        rung_name(res.rung),
                        eht,
                        res.totals.attempts,
                        res.totals.finishes,
                        seed
                    );
                }
                None => println!("{{\"plan\":\"{}\",\"rung\":null,\"note\":\"fewer than 50 finishes even at WR+160m\"}}", plan.params.name),
            }
        }
        Some("eval") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let rung = rung_arg(&args);
            let r: u64 = arg(&args, "--R", 10_000);
            let t = totals(&evaluate_blocks(&ctx, seed, r, rung_frames(rung)), r);
            println!(
                "{} @ {}: finishes {}/{} EHT {}",
                plan.params.name,
                rung_name(rung),
                t.finishes,
                t.attempts,
                t.eht_hours().map_or("inf".into(), |h| format!("{h:.3} h"))
            );
        }
        Some("compare") => {
            let (a, b) = (load(&args[1]), load(&args[2]));
            let (ca, cb) = (a.ctx(), b.ctx());
            let rung = rung_arg(&args);
            let r: u64 = arg(&args, "--R", 10_000);
            let d = compare(&ca, &cb, seed, r, rung);
            let f = |t: &Totals| {
                t.eht_hours()
                    .map_or("inf".into(), |h| format!("{h:.3} h ({} fin)", t.finishes))
            };
            println!("rung {}: a {} | b {}", rung_name(rung), f(&d.a), f(&d.b));
            match (d.diff_hours, d.ci) {
                (Some(x), Some((lo, hi))) => println!(
                    "diff a-b {x:+.3} h, 95% CI [{lo:+.3}, {hi:+.3}], significant: {}",
                    lo > 0.0 || hi < 0.0
                ),
                (Some(x), None) => {
                    println!("diff a-b {x:+.3} h (CI unavailable: too few finishes)")
                }
                _ => println!("diff undefined (a plan has zero finishes)"),
            }
        }
        Some("report") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let rung = rung_arg(&args);
            let r: u64 = arg(&args, "--R", 10_000);
            let rep = report(&ctx, seed, r, rung_frames(rung));
            let total_runs: f64 = rep.segs.iter().map(|s| s.sum).sum::<f64>() / rep.attempts as f64;
            println!(
                "{:<28}{:>9}{:>10}{:>9}{:>8}{:>8}{:>8}{:>6}{:>6}{:>8}",
                "segment",
                "reached",
                "mean",
                "sd(s)",
                "resets",
                "whiteo",
                "aband",
                "lvl",
                "dex",
                "%time"
            );
            let mut by_src = [0.0f64; 3];
            for (seg, s) in plan.segs.iter().zip(&rep.segs) {
                if s.n == 0 {
                    continue;
                }
                let mean = s.sum / s.n as f64;
                let sd = (s.sumsq / s.n as f64 - mean * mean).max(0.0).sqrt();
                let share = s.sum / rep.attempts as f64 / total_runs * 100.0;
                by_src[match seg.src {
                    Src::Ledger => 0,
                    Src::Tiles => 1,
                    Src::Guess => 2,
                }] += s.sum;
                println!(
                    "{:<28}{:>8.1}%{:>10}{:>9.1}{:>8}{:>8}{:>8}{:>6.1}{:>6.1}{:>7.1}%",
                    seg.name,
                    s.n as f64 / rep.attempts as f64 * 100.0,
                    hms(mean),
                    sd / FPS,
                    s.resets,
                    s.whiteouts,
                    s.abandons,
                    s.level_sum / s.n as f64,
                    s.caught_sum / s.n as f64,
                    share
                );
            }
            println!(
                "mean run clock per attempt (resets truncate): {}",
                hms(total_runs)
            );
            let tot: f64 = by_src.iter().sum();
            println!(
                "time by source: ledger {:.0}%  tiles {:.0}%  guess {:.0}%",
                by_src[0] / tot * 100.0,
                by_src[1] / tot * 100.0,
                by_src[2] / tot * 100.0
            );
        }
        Some("dist") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let r: u64 = arg(&args, "--R", 20_000);
            let mut runs: Vec<f64> = vec![];
            let mut unfinished = 0;
            let (mut oak_sum, mut oak_n, mut wo) = (0u64, 0u64, 0u64);
            for i in 0..r {
                let mut oak = 0u32;
                let o = attempt(&ctx, seed, i, None, None, None, Some(&mut oak));
                wo += o.whiteouts as u64;
                if oak > 0 {
                    oak_sum += oak as u64;
                    oak_n += 1;
                }
                if o.finished {
                    runs.push(o.run as f64);
                } else {
                    unfinished += 1;
                }
            }
            println!(
                "Kanto caught at Oak's check (before fill hunts): mean {:.1} over {} attempts that got there; whiteouts per attempt {:.3}",
                oak_sum as f64 / oak_n.max(1) as f64,
                oak_n,
                wo as f64 / r as f64
            );
            runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if runs.is_empty() {
                println!("no finishes ({unfinished} unfinished)");
                return;
            }
            let q = |p: f64| hms(runs[((runs.len() - 1) as f64 * p) as usize]);
            let mean = runs.iter().sum::<f64>() / runs.len() as f64;
            println!(
                "no-reset finishes {} unfinished {} | min {} p10 {} p50 {} p90 {} mean {} (WR is {})",
                runs.len(),
                unfinished,
                q(0.0),
                q(0.1),
                q(0.5),
                q(0.9),
                hms(mean),
                hms(WR_SECONDS * FPS)
            );
        }
        Some("starter") => {
            let min: u32 = arg(&args, "--min", 0);
            use frlg_mon::stats::calc_stats;
            let sq = frlg_mon::species::by_id(frlg_mon::species::SQUIRTLE).unwrap();
            let mut totals = vec![];
            for i in 0..100_000u64 {
                let mut rng = frlg_human::rng::Rng::new(1, i, 7);
                let mut g = frlg_rng::Rng(rng.next_u64() as u32);
                let genome = frlg_mon::create::gift_mon(&mut g);
                let s = calc_stats(sq, genome.ivs, 5, genome.nature());
                totals.push(
                    s.hp as u32
                        + s.atk as u32
                        + s.def as u32
                        + s.spe as u32
                        + s.spa as u32
                        + s.spd as u32,
                );
            }
            totals.sort();
            for p in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99, 1.0] {
                println!(
                    "p{:<4} total {}",
                    (p * 100.0) as u32,
                    totals[((totals.len() - 1) as f64 * p) as usize]
                );
            }
            if min > 0 {
                let pass =
                    totals.iter().filter(|&&t| t >= min).count() as f64 / totals.len() as f64;
                println!("P(total >= {min}) = {pass:.4}");
            }
        }
        _ => usage(),
    }
}
