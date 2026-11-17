use frlg_human::data::Dex;
use frlg_human::eval::*;
use frlg_human::route::Src;
use frlg_human::sim::{attempt, Probe, FPS};
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
    // An invalid plan (a candidate whose anchors the route does not have) is a rejected input, not a
    // bug: say so on stderr and exit 2 so callers can tell it from a panic (101).
    Plan::load(dex(), &text).unwrap_or_else(|e| {
        eprintln!("invalid plan {path}: {e}");
        std::process::exit(2)
    })
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
         \x20 starter-phase <plan> [--R n]                     manip starter phase: attempts and wall time to a good starter\n\
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
        Some("eval") | Some("ev") => {
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
            let jh = |h: Option<f64>| h.map_or("null".to_string(), |v| format!("{v:.4}"));
            println!(
                "JSON {{\"rung\":\"{}\",\"R\":{},\"a_eht\":{},\"a_fin\":{},\"b_eht\":{},\"b_fin\":{},\"diff\":{},\"ci_lo\":{},\"ci_hi\":{}}}",
                rung_name(rung),
                r,
                jh(d.a.eht_hours()),
                d.a.finishes,
                jh(d.b.eht_hours()),
                d.b.finishes,
                jh(d.diff_hours),
                jh(d.ci.map(|c| c.0)),
                jh(d.ci.map(|c| c.1))
            );
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
                "{:<28}{:>9}{:>10}{:>9}{:>8}{:>8}{:>8}{:>6}{:>6}{:>8}{:>8}{:>6}",
                "segment",
                "reached",
                "mean",
                "sd(s)",
                "resets",
                "whiteo",
                "aband",
                "lvl",
                "dex",
                "%time",
                "money",
                "balls"
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
                    "{:<28}{:>8.3}%{:>10}{:>9.1}{:>8}{:>8}{:>8}{:>6.1}{:>6.1}{:>7.1}%{:>8.0}{:>6.0}",
                    seg.name,
                    s.n as f64 / rep.attempts as f64 * 100.0,
                    hms(mean),
                    sd / FPS,
                    s.resets,
                    s.whiteouts,
                    s.abandons,
                    s.level_sum / s.n as f64,
                    s.caught_sum / s.n as f64,
                    share,
                    s.money_sum / s.n as f64,
                    s.balls_sum / s.n as f64
                );
            }
            for (seg, s) in plan.segs.iter().zip(&rep.segs) {
                if s.skips > 0 {
                    println!(
                        "skipped {}: {} of {} attempts that reached it ({:.1}%)",
                        seg.name,
                        s.skips,
                        s.n,
                        s.skips as f64 / s.n.max(1) as f64 * 100.0
                    );
                }
            }
            println!(
                "mean run clock per attempt (resets truncate): {}",
                hms(total_runs)
            );
            let tot: f64 = by_src.iter().sum();
            if plan.params.starter_model == "manip" {
                println!("{}", phase_report(&ctx, seed, r.min(20_000)).text());
            }
            println!(
                "time by source: ledger {:.0}%  tiles {:.0}%  guess {:.0}%",
                by_src[0] / tot * 100.0,
                by_src[1] / tot * 100.0,
                by_src[2] / tot * 100.0
            );
        }
        Some("starter-phase") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let r: u64 = arg(&args, "--R", 20_000);
            println!("{}", phase_report(&ctx, seed, r).text());
        }
        Some("optstop") => {
            let plan = load(&args[1]);
            let mut ctx = plan.ctx();
            let rung = rung_arg(&args);
            let cfg = frlg_human::optstop::Cfg {
                target: rung_frames(rung),
                train: arg(&args, "--R", 200_000),
                eval: arg(&args, "--Reval", 400_000),
                iters: arg(&args, "--iters", 4),
                slack_s: arg(&args, "--slack", 240.0),
                seed,
            };
            let rn = rung_name(rung);
            let res = frlg_human::optstop::run(&mut ctx, &cfg, |it| {
                eprintln!("iter: EHT {:.2} h ({} finishes)", it.lambda_h, it.finishes);
            });
            if let Some(last) = res.last() {
                let body: Vec<String> = last
                    .table
                    .iter()
                    .map(|(n, c, _, _)| format!("\"{n}\":{c:.0}"))
                    .collect();
                println!(
                    "{{\"rung\":\"{rn}\",\"eht_h\":{:.3},\"table\":{{{}}}}}",
                    last.lambda_h,
                    body.join(",")
                );
                for (n, c, np, cens) in &last.table {
                    eprintln!(
                        "  {n:40} {c:8.0} s  n={np} {}",
                        if *cens { "CENSORED" } else { "" }
                    );
                }
            }
        }
        Some("dist") => {
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let r: u64 = arg(&args, "--R", 20_000);
            let mut runs: Vec<f64> = vec![];
            let mut unfinished = 0;
            let (mut oak_sum, mut oak_n, mut wo) = (0u64, 0u64, 0u64);
            let mut fill: Vec<(f64, f64, u64)> = vec![];
            let mut missing = [0u64; 152];
            let mut gave_up = 0u64;
            let mut oak_miss = [0u64; 152];
            let mut miss_n = 0u64;
            for i in 0..r {
                let mut pr = Probe::default();
                let o = attempt(&ctx, seed, i, None, None, None, Some(&mut pr));
                wo += o.whiteouts as u64;
                if !pr.oak_missing.is_empty() {
                    miss_n += 1;
                    for &d in &pr.oak_missing {
                        oak_miss[d as usize] += 1;
                    }
                }
                if !pr.missing.is_empty() {
                    gave_up += 1;
                    for &d in &pr.missing {
                        missing[d as usize] += 1;
                    }
                }
                if pr.oak_count > 0 {
                    oak_sum += pr.oak_count as u64;
                    oak_n += 1;
                }
                if fill.len() < pr.fill.len() {
                    fill.resize(pr.fill.len(), (0.0, 0.0, 0));
                }
                for (a, b) in fill.iter_mut().zip(&pr.fill) {
                    a.0 += b.0;
                    a.1 += b.1;
                    a.2 += b.2;
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
            for (h, f) in plan.params.fill.iter().zip(&fill) {
                if f.2 > 0 {
                    println!(
                        "  fill {:<28} {:<5} run {:>5.1}% mean {:>5.0}s gain {:>5.2} -> {:>6.0} s/species",
                        h.site.trim_start_matches("MAP_"),
                        h.area,
                        f.2 as f64 / r as f64 * 100.0,
                        f.0 / f.2 as f64 / FPS,
                        f.1 / f.2 as f64,
                        f.0 / FPS / f.1.max(1e-9)
                    );
                }
            }
            if miss_n > 0 {
                let mut m: Vec<(u64, usize)> = (1..=151).map(|d| (oak_miss[d], d)).collect();
                m.sort_by(|a, b| b.cmp(a));
                let line: Vec<String> = m
                    .iter()
                    .filter(|x| x.0 * 10 >= miss_n)
                    .map(|&(n, d)| format!("{d}:{:.0}%", n as f64 / miss_n as f64 * 100.0))
                    .collect();
                println!(
                    "missing when the fill starts ({miss_n} attempts), national no.: {}",
                    line.join(" ")
                );
            }
            if miss_n > 0 {
                // The same data the other way round: what is in the bag when the fill starts.
                let mut name: Vec<String> = vec![String::new(); 152];
                for sp in &ctx.dex.species {
                    if sp.dex != 0 && name[sp.dex as usize].is_empty() {
                        name[sp.dex as usize] =
                            sp.name.trim_start_matches("SPECIES_").to_lowercase();
                    }
                }
                let line: Vec<String> = (1..=151usize)
                    .filter(|&d| oak_miss[d] < miss_n)
                    .map(|d| {
                        format!(
                            "{}:{}:{:.0}%",
                            d,
                            name[d],
                            (miss_n - oak_miss[d]) as f64 / miss_n as f64 * 100.0
                        )
                    })
                    .collect();
                println!("in the bag when the fill starts: {}", line.join(" "));
            }
            if gave_up > 0 {
                let mut m: Vec<(u64, usize)> = (1..=151).map(|d| (missing[d], d)).collect();
                m.sort_by(|a, b| b.cmp(a));
                println!("fill gave up in {gave_up} attempts; most often missing (national no. x share):");
                let line: Vec<String> = m
                    .iter()
                    .take(30)
                    .filter(|x| x.0 * 5 >= gave_up)
                    .map(|&(n, d)| format!("{d}:{:.0}%", n as f64 / gave_up as f64 * 100.0))
                    .collect();
                println!("  {}", line.join(" "));
            }
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
        Some("dmg") => {
            // dmg <TRAINER_ID> <level> [--species S] [--moves A,B] [--ivs h,a,d,s,sa,sd] [--evs ...] [--nature N]
            //   [--badges mask] [--boost TYPE,PCT] [--stage n] [--hp cur,max]
            // Per foe of the trainer's party: HP, speed, and each move's damage range (85..100% roll) with the
            // hits needed (`ceil(hp / max)`, `ceil(hp / min)`), at a given special/attack stage and HP (Torrent).
            use frlg_human::battle::{damage_range, enemy_mon, make_mon};
            let dexr = dex();
            let level: u32 = args[2].parse().unwrap();
            let species = dexr.id(&arg(&args, "--species", "SPECIES_BLASTOISE".to_string()));
            let movearg: String = arg(&args, "--moves", "MOVE_SURF".to_string());
            let moves: Vec<usize> = movearg.split(',').map(|m| dexr.move_id(m)).collect();
            let list = |flag: &str, def: [u32; 6]| -> [u32; 6] {
                let s: String = arg(&args, flag, String::new());
                if s.is_empty() {
                    return def;
                }
                let v: Vec<u32> = s.split(',').map(|x| x.parse().unwrap()).collect();
                [v[0], v[1], v[2], v[3], v[4], v[5]]
            };
            let mut me = make_mon(
                dexr,
                species,
                level,
                list("--ivs", [15, 26, 9, 29, 30, 20]),
                list("--evs", [0; 6]),
                arg(&args, "--nature", 19u8),
                arg(&args, "--badges", 255u32) as u8,
            );
            let boost: String = arg(&args, "--boost", String::new());
            if let Some((t, p)) = boost.split_once(',') {
                me.boost = Some((t.parse().unwrap(), p.parse().unwrap()));
            }
            let stage: i8 = arg(&args, "--stage", 0i8);
            me.stages[0] = stage;
            me.stages[3] = stage;
            let hp: String = arg(&args, "--hp", String::new());
            if let Some((c, m)) = hp.split_once(',') {
                me.hp = c.parse().unwrap();
                me.max_hp = m.parse().unwrap();
            }
            println!(
                "{} L{level} stats(Atk,Def,Spe,SpA,SpD) {:?} hp {}/{} stage {stage}",
                dexr.species[species].name, me.stat, me.hp, me.max_hp
            );
            let f = dexr.fight(&args[1]);
            for p in &f.full {
                let e = enemy_mon(dexr, p);
                print!(
                    "  {:<22} L{:<3} hp {:>3} def/spd {:>3}/{:>3} spe {:>3}:",
                    dexr.species[e.species].name.trim_start_matches("SPECIES_"),
                    e.level,
                    e.max_hp,
                    e.stat[1],
                    e.stat[4],
                    e.stat[2]
                );
                for &mv in &moves {
                    let (lo, hi) = damage_range(dexr, &me, &e, mv);
                    let n = |d: i32| if d <= 0 { 99 } else { (e.max_hp + d - 1) / d };
                    print!(
                        "  {} {lo}-{hi} ({}..{} hits)",
                        dexr.moves[mv].name.trim_start_matches("MOVE_"),
                        n(hi),
                        n(lo)
                    );
                }
                println!();
            }
        }
        Some("battle") => {
            // battle <TRAINER_ID> <carry level> [--n 2000] [--iv 20] [--badges mask]
            //   [--species SPECIES_X] [--moves MOVE_A,MOVE_B] [--add MOVE_C,...]
            //   [--x defend,attack,...] [--heal-below 0.4] [--league]
            use frlg_human::battle::{enemy_mon, fight, heal_amount, make_mon, Tactics, XItem};
            let id = &args[1];
            let level: u32 = args[2].parse().unwrap();
            let n: u64 = arg(&args, "--n", 2000);
            let iv: u32 = arg(&args, "--iv", 20);
            let badges: u32 = arg(&args, "--badges", 0);
            let dexr = dex();
            let fightd = dexr.fight(id);
            let foes: Vec<_> = fightd.full.iter().map(|m| enemy_mon(dexr, m)).collect();
            let line =
                ["SPECIES_SQUIRTLE", "SPECIES_WARTORTLE", "SPECIES_BLASTOISE"].map(|s| dexr.id(s));
            let mut species = line[(level >= 16) as usize + (level >= 36) as usize];
            let alt: String = arg(&args, "--species", String::new());
            if !alt.is_empty() {
                species = dexr.id(&alt);
            }
            let movearg: String = arg(&args, "--moves", String::new());
            let mut moves: Vec<usize> = if movearg.is_empty() {
                let mut v = vec![];
                for &(l, m) in &dexr.species[species].learnset {
                    if l <= level && dexr.moves[m].power > 0 && !v.contains(&m) {
                        v.push(m);
                    }
                }
                let k = v.len();
                v.split_off(k.saturating_sub(4))
            } else {
                movearg.split(',').map(|m| dexr.move_id(m)).collect()
            };
            let addarg: String = arg(&args, "--add", String::new());
            for m in addarg.split(',').filter(|s| !s.is_empty()) {
                let mi = dexr.move_id(m);
                if moves.len() < 4 {
                    moves.push(mi);
                } else {
                    moves[0] = mi;
                }
            }
            if moves.is_empty() {
                moves.push(dexr.move_id("MOVE_TACKLE"));
            }
            let xarg: String = arg(&args, "--x", String::new());
            let xs: Vec<XItem> = xarg
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| match s {
                    "attack" => XItem::Attack,
                    "defend" => XItem::Defend,
                    "speed" => XItem::Speed,
                    "special" => XItem::Special,
                    "dire" => XItem::DireHit,
                    "accuracy" => XItem::Accuracy,
                    "guard" => XItem::GuardSpec,
                    o => panic!("unknown x item {o}"),
                })
                .collect();
            let league = args.iter().any(|a| a == "--league");
            let names: Vec<_> = moves.iter().map(|&m| dexr.moves[m].name.clone()).collect();
            println!(
                "carry {} L{level} moves {names:?} x {xs:?} (trainer ai {:x}, heals {})",
                dexr.species[species].name, fightd.ai, fightd.heals
            );
            let (mut won, mut turns, mut hp, mut ev) = (0.0, 0.0, 0.0, 0.0);
            let mut kos = vec![0u32; foes.len() + 1];
            for i in 0..n {
                let ivs = [
                    arg(&args, "--hp-iv", iv),
                    iv,
                    iv,
                    arg(&args, "--spa-iv", iv),
                    iv,
                    iv,
                ];
                let evall: u32 = arg(&args, "--ev", 0u32);
                // `--ivs h,atk,def,spe,spa,spd` / `--evs ...` (the sim's stat order) override the
                // uniform `--iv` / `--ev`; `--boost TYPE,PCT` is a held type booster.
                let list = |flag: &str, def: [u32; 6]| -> [u32; 6] {
                    let s: String = arg(&args, flag, String::new());
                    if s.is_empty() {
                        return def;
                    }
                    let v: Vec<u32> = s.split(',').map(|x| x.parse().unwrap()).collect();
                    [v[0], v[1], v[2], v[3], v[4], v[5]]
                };
                let ivs = list("--ivs", ivs);
                let mut me = make_mon(
                    dexr,
                    species,
                    level,
                    ivs,
                    list("--evs", [evall; 6]),
                    arg(&args, "--nature", 0u8),
                    badges as u8,
                );
                let boost: String = arg(&args, "--boost", String::new());
                if let Some((t, p)) = boost.split_once(',') {
                    me.boost = Some((t.parse().unwrap(), p.parse().unwrap()));
                }
                if args.iter().any(|a| a == "--print-stats") && i == 0 {
                    println!("stats {:?} hp {}", me.stat, me.max_hp);
                }
                let power: f64 = arg(&args, "--power", 1.0);
                if power != 1.0 {
                    me.stat[0] = (me.stat[0] as f64 * power) as i32;
                    me.stat[3] = (me.stat[3] as f64 * power) as i32;
                }
                me.moves = moves.clone();
                me.set_pp(dexr);
                let hpf: f64 = arg(&args, "--hp", 1.0);
                me.hp = ((me.max_hp as f64 * hpf).round() as i32).clamp(1, me.max_hp);
                let tac = Tactics {
                    x_items: xs.clone(),
                    heal_below: arg(&args, "--heal-below", 0.4),
                    heal: heal_amount(level, league),
                    full_restore: league,
                    sacs: arg(&args, "--sacs", 0u32),
                    sac_free: !args.iter().any(|a| a == "--sac-honest"),
                    helper_hp: arg(&args, "--helper-hp", 0i32),
                    torrent: args.iter().any(|a| a == "--torrent"),
                    torrent_stall: true,
                    torrent_early: args.iter().any(|a| a == "--early"),
                    others_alive: arg(&args, "--others", 0u32),
                    heal_stock: arg(&args, "--heals", u32::MAX),
                    rollouts: arg(&args, "--rollouts", 0u32),
                    setup_min: arg(&args, "--setup-min", 1u32),
                    menu_slip: arg(&args, "--menu-slip", 0.0f64),
                };
                let mut g = frlg_human::rng::GameRng::from_u32(
                    (seed as u32).wrapping_mul(2654435761) ^ (i as u32).wrapping_mul(0x9E3779B9),
                );
                let o = fight(
                    dexr,
                    &mut me,
                    &tac,
                    &foes,
                    fightd.ai,
                    fightd.heals,
                    false,
                    &mut g,
                );
                won += o.won as u32 as f64;
                turns += o.turns as f64;
                hp += o.hp_frac;
                ev += o.events as f64;
                kos[o.kos.len().min(foes.len())] += 1;
            }
            let foe_lv: Vec<_> = foes.iter().map(|f| f.level).collect();
            println!("kos histogram {kos:?}");
            println!(
                "{id} foes L{foe_lv:?}: win {:.3}  turns {:.1}  hp left {:.2}  events {:.1}",
                won / n as f64,
                turns / n as f64,
                hp / n as f64,
                ev / n as f64
            );
        }
        Some("pace") => {
            // pace <plan> --rung r: at every split, the latest run clock at which the plan's reset rule
            // still lets the attempt continue (reset when elapsed + floor + aggr * (mean - floor) > T).
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let rung = rung_arg(&args);
            let t = rung_frames(rung) as f64;
            println!("target T = {}", hms(t));
            println!(
                "{:<34}{:>6}{:>12}{:>12}{:>18}",
                "split (after segment)", "aggr", "floor left", "mean left", "reset if clock >"
            );
            for (i, sg) in plan.segs.iter().enumerate() {
                if !sg.split {
                    continue;
                }
                let floor = ctx.floor_after[i] as f64;
                let mean = ctx.mean_after[i];
                let rem = floor + ctx.aggr[i] * (mean - floor).max(0.0);
                println!(
                    "{:<34}{:>6.2}{:>12}{:>12}{:>18}",
                    sg.name,
                    ctx.aggr[i],
                    hms(floor),
                    hms(mean),
                    hms(t - rem)
                );
            }
        }
        Some("units") => {
            // units <plan>: the mistake model's exposure of one full run (diagnostic, model unchanged):
            // the hundreds of walked tiles and mashed or menu inputs that `mistake_rate` is charged on,
            // the events they give at the plan's rate and their mean frame cost.
            use frlg_human::route::Kind;
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let (mut tiles, mut inputs) = (0u64, 0u64);
            for s in ctx.segs.iter().filter(|s| !s.off_timer) {
                match &s.kind {
                    Kind::Text { .. } => inputs += s.inputs as u64,
                    Kind::Walk { tiles: t, .. } => tiles += *t as u64,
                    _ => {}
                }
            }
            let rate = plan.params.mistake_rate;
            let (lo, hi) = (plan.params.react_lo as f64, plan.params.react_hi as f64);
            let per_event = (60.0 + 299.0) / 2.0 + (lo + hi) / 2.0;
            let units = (tiles + inputs) as f64 / 100.0;
            println!(
                "tiles {tiles} inputs {inputs} units(100) {units:.1} | rate {rate} -> events/run {:.1}, {per_event:.0} f ({:.2} s) per event, {:.0} s per run",
                units * rate,
                per_event / FPS,
                units * rate * per_event / FPS
            );
        }
        Some("splits") => {
            // splits <plan> [--R n] [--all]: no-reset cumulative clock at each WR split (or every segment).
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let r: u64 = arg(&args, "--R", 4_000);
            let all = args.iter().any(|a| a == "--all");
            let within: f64 = arg(&args, "--within", 0.0);
            frlg_human::wrsplits::print_table(
                &plan,
                &ctx,
                seed,
                r,
                all,
                (within > 0.0).then_some(within),
            );
        }
        Some("bag") => {
            // What is in the bag after a segment (no resets): per species the share of attempts holding
            // it and its mean catch level, plus candies and money. Used to pick evolution events.
            let plan = load(&args[1]);
            let ctx = plan.ctx();
            let after: String = arg(&args, "--after", String::new());
            let idx = plan
                .segs
                .iter()
                .position(|s| s.name == after)
                .unwrap_or_else(|| panic!("--after {after:?} matches no segment"));
            let r: u64 = arg(&args, "--R", 20_000);
            let (mut n, mut cand, mut money, mut carry) = (0u64, 0.0, 0.0, 0.0);
            let mut cnt = [0u64; 152];
            let mut lv = [0f64; 152];
            let mut seq = [0f64; 152];
            let mut by_src = [0f64; 7];
            for i in 0..r {
                let mut pr = Probe {
                    snap_at: Some(idx),
                    ..Default::default()
                };
                attempt(&ctx, seed, i, None, None, None, Some(&mut pr));
                let Some(sn) = pr.snap else { continue };
                n += 1;
                cand += sn.candies as f64;
                money += sn.money as f64;
                carry += sn.carry as f64;
                for (d, l, q, sr) in sn.mons {
                    by_src[sr as usize % 7] += 1.0;
                    cnt[d as usize] += 1;
                    lv[d as usize] += l as f64;
                    seq[d as usize] += q as f64;
                }
            }
            let mut name: Vec<String> = vec![String::new(); 152];
            for sp in &ctx.dex.species {
                if sp.dex != 0 && name[sp.dex as usize].is_empty() {
                    name[sp.dex as usize] = sp.name.trim_start_matches("SPECIES_").to_string();
                }
            }
            println!(
                "after {after}: {n} of {r} attempts reached it; candies {:.2} money {:.0} carry Lv{:.1}",
                cand / n.max(1) as f64,
                money / n.max(1) as f64,
                carry / n.max(1) as f64
            );
            let mut tot = 0.0;
            for d in 1..=151usize {
                if cnt[d] > 0 {
                    let share = cnt[d] as f64 / n.max(1) as f64;
                    tot += share;
                    println!(
                        "  {d:3} {:<12} {:>5.1}%  lv {:>4.1}  #{:>4.1}",
                        name[d],
                        share * 100.0,
                        lv[d] / cnt[d] as f64,
                        seq[d] / cnt[d] as f64
                    );
                }
            }
            println!("expected species {tot:.2}");
            let parts: Vec<String> = frlg_human::sim::SRC_NAMES
                .iter()
                .zip(by_src)
                .map(|(nm, c)| format!("{nm} {:.1}", c / n.max(1) as f64))
                .collect();
            println!("entries by source: {}", parts.join(", "));
        }
        Some("names") => {
            let plan = load(&args[1]);
            for sg in &plan.segs {
                println!("{}", sg.name);
            }
        }
        Some("segs") => {
            // Segment names with their kind, for tools that enumerate knobs.
            let plan = load(&args[1]);
            for sg in &plan.segs {
                let k = match &sg.kind {
                    frlg_human::route::Kind::Walk { enc: Some(_), .. } => "walk-grass",
                    frlg_human::route::Kind::Wander { .. } => "wander",
                    _ => continue,
                };
                println!("{} {k}", sg.name);
            }
        }
        Some("starter") => {
            let min: u32 = arg(&args, "--min", 0);
            use frlg_mon::stats::calc_stats;
            let which: String = arg(&args, "--species", "squirtle".to_string());
            let sq = frlg_mon::species::by_id(match which.as_str() {
                "bulbasaur" => frlg_mon::species::BULBASAUR,
                "charmander" => frlg_mon::species::CHARMANDER,
                _ => frlg_mon::species::SQUIRTLE,
            })
            .unwrap();
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
            {
                let base = calc_stats(
                    sq,
                    frlg_mon::stats::Ivs {
                        hp: 15,
                        atk: 15,
                        def: 15,
                        spe: 15,
                        spa: 15,
                        spd: 15,
                    },
                    50,
                    0,
                );
                let score = |s: &frlg_mon::stats::Stats| s.spa as f64 + 0.6 * s.hp as f64;
                let mut st = vec![];
                let mut spa = [0u32; 20];
                for i in 0..100_000u64 {
                    let mut rng = frlg_human::rng::Rng::new(1, i, 7);
                    let mut g = frlg_rng::Rng(rng.next_u64() as u32);
                    let genome = frlg_mon::create::gift_mon(&mut g);
                    let s5 = calc_stats(sq, genome.ivs, 5, genome.nature());
                    spa[(s5.spa as usize).min(19)] += 1;
                    st.push(score(&calc_stats(sq, genome.ivs, 50, genome.nature())) / score(&base));
                }
                st.sort_by(|a, b| a.partial_cmp(b).unwrap());
                for p in [0.1, 0.5, 0.75, 0.9, 0.99] {
                    println!(
                        "p{:<3} strength {:.3}",
                        (p * 100.0) as u32,
                        st[((st.len() - 1) as f64 * p) as usize]
                    );
                }
                println!("L5 SpA histogram from 8: {:?}", &spa[8..17]);
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
