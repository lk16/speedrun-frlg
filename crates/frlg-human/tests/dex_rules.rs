//! S7 dex-fill rules shared by the simulator and the playing controller (`play/dex.rs`), checked
//! against what the controller read from RAM in the Safari Zone (journal
//! `2026-11-22-*-dex-shop.md`): the catch / escape factors the game set, the ball chance, the
//! visit cap, and the catch rule.
use frlg_human::data::Dex;
use frlg_human::eval::Plan;
use frlg_human::params::Hunt;
use frlg_human::sim::{
    catch_prob, hunt_skip, hunt_stop_new, safari_ball_odds, site_done, wants_catch, SafariAct,
    SafariMon,
};

fn plan() -> Plan {
    let dex: &'static Dex = Box::leak(Box::new(Dex::load()));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../human-run/plans/1122-lgdescent.md"
    ))
    .unwrap();
    Plan::load(dex, &text).unwrap()
}

#[test]
fn safari_factors_match_the_game() {
    let plan = plan();
    let ctx = plan.ctx();
    let dex = plan.dex;
    // (species, catch factor and escape factor the controller read from gBattleStruct at the
    // first menu, ball chance it logged).
    for (name, factor, esc, chance) in [
        ("SPECIES_NIDORAN_M", 18, 3, 0.50),
        ("SPECIES_DODUO", 14, 3, 0.41),
        ("SPECIES_PARASECT", 5, 5, 0.12),
    ] {
        let sp = &dex.species[dex.id(name)];
        let mut m = SafariMon::new(&ctx, sp);
        assert_eq!((m.base, m.esc), (factor, esc), "{name}");
        let p = catch_prob(safari_ball_odds(m.factor));
        assert!((p - chance).abs() < 0.006, "{name}: {p}");
        // The plan throws no rocks and no bait: balls only.
        assert_eq!(m.next_act(), SafariAct::Ball);
    }
}

#[test]
fn safari_visit_cap_and_catch_rule() {
    let plan = plan();
    let ctx = plan.ctx();
    let dex = plan.dex;
    let east = &plan.params.hunts[0];
    assert_eq!(east.site, "MAP_SAFARI_ZONE_EAST");
    // safari_stop 5: four caught in the East leaves one for the West.
    assert_eq!(hunt_stop_new(&ctx, east, 0), 5);
    assert_eq!(hunt_stop_new(&ctx, &plan.params.hunts[1], 4), 1);
    let off = Hunt {
        site: "MAP_ROUTE15".into(),
        stop_after: 2,
        ..Hunt::default()
    };
    assert_eq!(hunt_stop_new(&ctx, &off, 4), 2);
    let skip = hunt_skip(&ctx, east);
    let doduo = dex.id("SPECIES_DODUO");
    assert!(wants_catch(&ctx, doduo, false, &skip, true));
    assert!(!wants_catch(&ctx, doduo, true, &skip, true));
    assert!(!wants_catch(&ctx, doduo, false, &[doduo], true));
    // Site complete when every slot is owned.
    let (_, slots, _) = dex
        .area(&east.site, frlg_human::data::Area::Land, true, "none")
        .unwrap();
    assert!(site_done(&ctx, &slots, &|_| true, &skip));
    assert!(!site_done(&ctx, &slots, &|_| false, &skip));
}
