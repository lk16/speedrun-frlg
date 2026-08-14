//! Which of the intro's waits are input-gated, and which are timer-gated?
//!
//! Method: replay a committed ledger to the start of a segment, then for
//! probe points every STRIDE frames inside the segment, inject one idle
//! frame at the probe point and replay the segment's remaining committed
//! input. If the segment's `reached` condition still lands at the same
//! absolute frame, the injected frame was absorbed by a timer wait --
//! local slack, a scripted beat the drive is already early for. If the
//! end shifts by one frame, the probe point is on the input-critical
//! path. The absorbed count bounds how much a smarter drive could ever
//! save: only input-critical frames respond to input at all.
//!
//! Usage: intro-slack <ledger.json> <segment> [stride]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger_path = std::env::args().nth(1).expect("ledger.json path");
    let seg_name = std::env::args().nth(2).expect("segment name");
    let stride: usize = std::env::args()
        .nth(3)
        .map(|s| s.parse().expect("stride"))
        .unwrap_or(16);
    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = frlg_route::Observer::new(syms).map_err(std::io::Error::other)?;

    let version = frlg_route::Version::of_rom(&rom)?.ok_or("not FR/LG")?;
    let starter = match ledger.starter.as_str() {
        "bulbasaur" => frlg_route::Starter::Bulbasaur,
        "squirtle" => frlg_route::Starter::Squirtle,
        _ => frlg_route::Starter::Charmander,
    };
    let segs = frlg_route::segments::all(version, starter, ledger.tuning);
    let reached = &segs
        .iter()
        .find(|s| s.name == seg_name)
        .ok_or("segment not in target list")?
        .reached;

    // Replay the prefix, grab the segment's own committed input.
    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    let mut seg_input: Option<Vec<u16>> = None;
    for seg in &ledger.segments {
        let log = frlg_emu::InputLog::decode(&std::fs::read(&seg.log)?)?;
        if seg.name == seg_name {
            seg_input = Some(log.frames);
            break;
        }
        for &keys in &log.frames {
            emu.step(keys);
        }
    }
    let seg_input = seg_input.ok_or("segment not in ledger")?;
    let start_state = emu.save_state()?;

    // Baseline: earliest frame (within the committed input) where `reached`
    // first holds.
    let end_of = |emu: &mut frlg_emu::Emu, input: &[u16]| -> Option<usize> {
        for (i, &keys) in input.iter().enumerate() {
            emu.step(keys);
            if (reached)(&obs, emu) {
                return Some(i);
            }
        }
        None
    };
    emu.load_state(&start_state)?;
    let baseline = end_of(&mut emu, &seg_input).ok_or("committed input never reaches the goal")?;
    println!(
        "{seg_name}: committed input reaches the goal at frame {baseline} of {}",
        seg_input.len()
    );

    // Probe: idle 1 frame at p, then the committed input from p onward.
    let mut absorbed = 0usize;
    let mut critical = 0usize;
    let mut runs: Vec<(usize, bool)> = Vec::new(); // (probe, absorbed?)
    let mut p = 0usize;
    while p < baseline {
        emu.load_state(&start_state)?;
        for &keys in &seg_input[..p] {
            emu.step(keys);
        }
        emu.step(0);
        let mut input = seg_input[p..].to_vec();
        input.push(0);
        let end = end_of(&mut emu, &input).map(|i| p + 1 + i);
        let is_absorbed = end == Some(baseline);
        if is_absorbed {
            absorbed += 1;
        } else {
            critical += 1;
        }
        runs.push((p, is_absorbed));
        p += stride;
    }
    println!(
        "probes every {stride} frames: {absorbed} absorbed (timer-gated slack), {critical} shift the end (input-critical)"
    );
    // Contiguous regions, for the doc.
    let mut region_start: Option<usize> = None;
    for i in 0..runs.len() {
        let (p, a) = runs[i];
        let next_a = runs.get(i + 1).map(|&(_, a)| a);
        if a && region_start.is_none() {
            region_start = Some(p);
        }
        if let Some(s) = region_start {
            if next_a != Some(true) {
                println!("  timer-gated region ~f{s}..f{p}");
                region_start = None;
            }
        }
    }
    println!(
        "=> at most ~{} of {} frames respond to input at all; the rest is scripted floor",
        critical * stride,
        baseline
    );
    Ok(())
}
