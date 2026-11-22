//! S4 battle bridge against real game states: the RAM conversion agrees with the simulator's
//! own stat model, and the simulator's policy, fed the converted state, makes the choice the
//! controller logged. Needs the LeafGreen ROM and the savestates in
//! `$FRLG_ARTIFACTS/states/human-play/battle-bridge/` (cut from the verified s11 `run.ilog`);
//! without them the tests print why and pass.
use std::path::PathBuf;

use frlg_emu::{keys, SymbolTable};
use frlg_human::battle::{decide, enemy_mon, make_mon, Action, Fs};
use frlg_human::data::Dex;
use frlg_human::eval::Plan;
use frlg_human::play::battle::{read_scene, sim_party_mon, tactics_for};
use frlg_human::play::battle_ram::BattleRam;
use frlg_human::rng::GameRng;
use frlg_route::{Feed, Observer, Recorder};

struct Env {
    rom: PathBuf,
    obs: Observer,
    ram: BattleRam,
    states: PathBuf,
}

fn env() -> Option<Env> {
    let art = PathBuf::from(std::env::var("FRLG_ARTIFACTS").ok()?);
    let rom = art.join("rom/pokeleafgreen.gba");
    let sym = art.join("rom/pokeleafgreen.sym");
    let states = art.join("states/human-play/battle-bridge");
    if !rom.exists() || !sym.exists() || !states.exists() {
        eprintln!("skip: LeafGreen ROM, symbols or battle-bridge states missing");
        return None;
    }
    let syms = SymbolTable::load(&sym).ok()?;
    Some(Env {
        rom,
        ram: BattleRam::new(&syms).unwrap(),
        obs: Observer::new(syms).unwrap(),
        states,
    })
}

fn dex() -> &'static Dex {
    Box::leak(Box::new(Dex::load()))
}

/// Load `state` and mash A to the first action menu (plain inputs: the policy's choice below
/// depends on the state, not on the human's timing).
fn to_first_menu(e: &Env, state: &str) -> Recorder {
    let mut rec = Recorder::from_reset(&e.rom).unwrap();
    rec.emu().load_state_file(&e.states.join(state)).unwrap();
    let obs = e.obs.clone();
    rec.mash_until("the first action menu", keys::A, 8000, |emu| {
        obs.battle_controller_is(emu, 0, "HandleInputChooseAction")
    })
    .unwrap();
    rec
}

#[test]
fn misty_first_turn_matches_the_policy_and_the_stat_model() {
    let Some(e) = env() else { return };
    let dex = dex();
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../human-run/plans/1122-lgdescent.md"
    ))
    .unwrap();
    let plan = Plan::load(dex, &text).unwrap();
    let ctx = plan.ctx();
    let mut rec = to_first_menu(&e, "0130700-pre-misty.state");
    let emu = rec.emu();
    let sc = read_scene(dex, &e.obs, &e.ram, emu, true).unwrap();
    assert_eq!(dex.species[sc.foe.species].name, "SPECIES_STARYU");
    assert_eq!((sc.foe.level, sc.foe.hp), (18, 38));
    assert_eq!(sc.bench.len(), 1, "Starmie waits behind Staryu");

    // The model's trainer mons (`enemy_mon`: trainers.h IVs, fixed nature) are the game's.
    let fight = dex.fight("TRAINER_LEADER_MISTY");
    let party = e.ram.enemy_party(emu);
    for (pm, raw) in fight.full.iter().zip(&party) {
        let m = enemy_mon(dex, pm);
        assert_eq!(
            m.stat,
            raw.stats.map(|s| s as i32),
            "{}",
            dex.species[m.species].name
        );
        assert_eq!(m.max_hp, raw.max_hp as i32);
    }
    // The model's stat formula on the carry's real IVs, EVs and nature gives the game's stats.
    let lead = &sc.party[sc.active];
    let conv = sim_party_mon(dex, lead, sc.badges, true, sc.active)
        .unwrap_or_else(|| panic!("lead {lead:?} battler {:?}", e.ram.battle_mon(emu, 0)));
    let m = make_mon(
        dex,
        conv.species,
        lead.level as u32,
        lead.ivs,
        lead.evs,
        (lead.personality % 25) as u8,
        sc.badges,
    );
    assert_eq!(m.stat, lead.stats.map(|s| s as i32));
    assert_eq!(m.max_hp, lead.max_hp as i32);

    // The policy on this state: Mega Kick, as the controller logged at frame 131559 (seed 11).
    let tac = tactics_for(&ctx, "misty", false, &sc);
    let fs = Fs::new(&tac, fight.heals);
    let d = decide(
        dex,
        &sc.me,
        &sc.foe,
        &sc.bench,
        &tac,
        &fs,
        fight.ai,
        fight.heals,
        false,
        &mut GameRng::from_u32(1),
    );
    assert_eq!(d.action, Action::Move(dex.move_id("MOVE_MEGA_KICK")));
    assert_eq!(d.read.danger, 8);
    assert!(!d.read.sure_kill);
}

#[test]
fn wild_paras_converts_exactly_and_has_a_catch_rate() {
    let Some(e) = env() else { return };
    let dex = dex();
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../human-run/plans/1122-lgdescent.md"
    ))
    .unwrap();
    let plan = Plan::load(dex, &text).unwrap();
    let ctx = plan.ctx();
    let mut rec = to_first_menu(&e, "0103300-wild-paras.state");
    let emu = rec.emu();
    let sc = read_scene(dex, &e.obs, &e.ram, emu, false).unwrap();
    assert_eq!(dex.species[sc.foe.species].name, "SPECIES_PARAS");
    // The wild mon's generation (PID nature, IVs) through the model's stat formula = the game's.
    let raw = &e.ram.enemy_party(emu)[0];
    let m = sim_party_mon(dex, raw, 0, false, 0).unwrap();
    let rebuilt = make_mon(
        dex,
        m.species,
        raw.level as u32,
        raw.ivs,
        raw.evs,
        (raw.personality % 25) as u8,
        0,
    );
    assert_eq!(rebuilt.stat, raw.stats.map(|s| s as i32));
    assert_eq!(rebuilt.max_hp, raw.max_hp as i32);
    // Catch odds at full HP with the plan's ball (`ball: great`), real catch rate 190.
    let sp = &dex.species[sc.foe.species];
    let tier = frlg_human::sim::ball_tier(&ctx, sp.catch_rate);
    assert_eq!(tier, 1);
    let odds = frlg_human::sim::catch_odds(
        sp.catch_rate,
        frlg_human::sim::BALL_MULT[tier],
        sc.foe.hp as u32,
        sc.foe.max_hp as u32,
        sc.foe.status,
    );
    let p = frlg_human::sim::catch_prob(odds);
    assert!(p > 0.2 && p < 0.6, "p {p}");
    // The weakening move never KOs from full HP, a crit included.
    let mv = frlg_human::battle::weaken_move(dex, &sc.me, &sc.foe).unwrap();
    let (_, hi) = frlg_human::battle::damage_range(dex, &sc.me, &sc.foe, mv);
    assert!(2 * hi < sc.foe.hp);
}
