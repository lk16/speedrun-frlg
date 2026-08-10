//! Exploration aid: replay the route up to the lab, then narrate the starter
//! pickup frame group by frame group.
//!
//! `cargo run --release --example starter`

use frlg_emu::{keys, SymbolTable};
use frlg_route::nav::{self, Goal};
use frlg_route::observe::{Observer, VAR_OAKS_LAB_SCENE, VAR_STARTER_MON};
use frlg_route::record::{Feed, Recorder};
use frlg_route::segments::{self, Starter, Tuning};

const OAKS_LAB: (u8, u8) = (4, 3);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rom = frlg_emu::default_rom_path().ok_or("no ROM")?;
    let syms = SymbolTable::load(&frlg_emu::default_sym_path().ok_or("no syms")?)?;
    let obs = Observer::new(syms)?;
    let mut rec = Recorder::from_reset(&rom)?;

    // Everything up to standing in the lab is already routed.
    for segment in segments::all(Starter::Squirtle, Tuning::default())
        .into_iter()
        .take(5)
    {
        (segment.run)(&mut rec, &obs)?;
        println!("{:<16} {}", segment.name, obs.snapshot(rec.emu()));
    }

    let say = |label: &str, rec: &mut Recorder| {
        let scene = obs.var(rec.emu(), VAR_OAKS_LAB_SCENE);
        let starter = obs.var(rec.emu(), VAR_STARTER_MON);
        println!(
            "{label:<22} {} scene={scene:?} starter={starter:?} can_step={}",
            obs.snapshot(rec.emu()),
            obs.player_can_step(rec.emu())
        );
    };

    rec.mash_until("Oak's offer", keys::A, 4000, |emu| {
        obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(2) && obs.player_can_step(emu)
    })?;
    say("after Oak's offer", &mut rec);

    nav::walk_to(&mut rec, &obs, Goal::tile(OAKS_LAB, 9, 5), 4000)?;
    say("below the ball", &mut rec);

    rec.wait_until("settle", 240, |emu| obs.player_can_step(emu))?;
    rec.hold(keys::UP, 8)?;
    rec.idle(1)?;
    say("facing the ball", &mut rec);

    for i in 0..10 {
        for _ in 0..30 {
            rec.tap(keys::A)?;
        }
        say(&format!("A x30 #{i}"), &mut rec);
        rec.emu()
            .write_png(std::path::Path::new(&format!("/tmp/starter-{i}.png")))?;
        if obs.party_count(rec.emu()) == 1 {
            break;
        }
    }
    Ok(())
}
