//! `Params::fight_lists` (exp-economy audit, journal `2026-11-22-*-exp-and-koga.md`): a named trainer segment gets
//! exactly the listed fights, any other named segment a new `<seg>+fights` segment after it.
use frlg_human::data::Dex;
use frlg_human::eval::Plan;
use frlg_human::route::Kind;

fn plan(extra: &str) -> Plan {
    let dex: &'static Dex = Box::leak(Box::new(Dex::load()));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../human-run/plans/1211-cerulean-potions.md"
    ))
    .unwrap();
    let text = text.replacen(
        "\"persim_held\": true",
        &format!("\"persim_held\": true, {extra}"),
        1,
    );
    Plan::load(dex, &text).unwrap()
}

fn fights(p: &Plan, seg: &str) -> Option<usize> {
    p.segs
        .iter()
        .find(|s| s.name == seg)
        .map(|s| match &s.kind {
            Kind::Parties { parties, .. } => parties.len(),
            _ => usize::MAX,
        })
}

#[test]
fn fight_lists_replace_and_insert() {
    let base = plan("\"fight_lists\": {}");
    assert_eq!(fights(&base, "route8-trainers"), Some(1));
    assert_eq!(fights(&base, "cycling-road+fights"), None);
    let p = plan(
        "\"fight_lists\": {\"route8-trainers\": [\"TRAINER_LASS_JULIA\", \"TRAINER_SUPER_NERD_AIDAN\"], \
         \"cycling-road\": [\"TRAINER_BIKER_LAO\"]}",
    );
    assert_eq!(fights(&p, "route8-trainers"), Some(2));
    assert_eq!(fights(&p, "cycling-road+fights"), Some(1));
    // After its anchor (pickups anchored on the same segment are inserted between them).
    let at = |n: &str| p.segs.iter().position(|s| s.name == n).unwrap();
    assert!(at("cycling-road+fights") > at("cycling-road"));
    assert!(at("cycling-road+fights") < at("snorlax"));
}

#[test]
fn empty_list_drops_a_single_trainer() {
    let base = plan("\"fight_lists\": {}");
    assert_eq!(fights(&base, "bond-bridge-nikki"), Some(usize::MAX));
    let p = plan("\"fight_lists\": {\"bond-bridge-nikki\": []}");
    assert_eq!(fights(&p, "bond-bridge-nikki"), Some(0));
    assert_eq!(fights(&p, "bond-bridge-nikki+fights"), None);
}
