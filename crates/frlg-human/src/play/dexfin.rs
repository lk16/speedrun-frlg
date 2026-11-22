//! The end of the dex fill: when hunting is down to sites with one uncaught species at a small
//! share of the encounters, the last species for Oak's check (caught >= 60,
//! `decompiled/data/maps/PalletTown/scripts.inc:41-57`) come cheaper from what the human already
//! has: stone evolutions from the bag, Rare Candy evolutions, in-game trades of a boxed mon, the
//! Magikarp salesman and a fossil revival. Not in the model's run (its 60 are caught before the
//! League); logged `model-vs-actual`.
//!
//! [`plan`] reads the party, the PC boxes, the bag, the money and the Pokédex from RAM, lists every
//! action that would add a species, and picks the cheapest set of **places** whose actions reach 60
//! (all subsets of places are tried: there are at most a handful). Each `dex-finish-N` stage
//! re-plans from RAM and does the first pending place, so a death costs one place. The last stage
//! puts the party back the way the fill left it (the post-game legs need Lapras for Strength and
//! Waterfall, a Cut user and the Fly user).
use serde_json::json;

use super::dex::{self, caught_count, owned_set};
use super::legs2::have;
use super::names::{id, names};
use super::ram::{read_mon, Ram};
use super::session::{PResult, PlayError, Session};
use super::shop::{self, PcOp};
use super::walk::{go_map, here};
use crate::params::Special;

/// Where an action happens. `Pc` is a party/PC visit for stone and candy evolutions, done at the
/// Pokémon Center of the first real place of the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    Pc,
    Route4,
    Vermilion,
    Route11,
    Celadon,
    Route18,
    Cinnabar,
}

impl Place {
    /// Visit order (a loop Mt. Moon -> Vermilion -> Celadon -> Fuchsia -> Cinnabar -> Pallet).
    const ORDER: [Place; 7] = [
        Place::Pc,
        Place::Route4,
        Place::Vermilion,
        Place::Route11,
        Place::Celadon,
        Place::Route18,
        Place::Cinnabar,
    ];

    /// Frames to get there and back into the loop: GUESSES (fly ~600 frames, walking ~16 frames a
    /// tile on the bike), used only to rank plans, never as a model constant.
    fn cost(self) -> u32 {
        match self {
            Place::Pc => 2500,
            Place::Route4 => 2500,
            Place::Vermilion => 1500,
            Place::Route11 => 1800,
            Place::Celadon => 2500,
            Place::Route18 => 2500,
            Place::Cinnabar => 3500,
        }
    }

    /// The Pokémon Center a party change for this place uses, and its `MAPSEC_*` for Fly.
    fn center(self) -> (&'static str, &'static str) {
        match self {
            Place::Pc | Place::Route4 => {
                ("MAP_ROUTE4_POKEMON_CENTER_1F", "MAPSEC_ROUTE_4_POKECENTER")
            }
            Place::Vermilion | Place::Route11 => (
                "MAP_VERMILION_CITY_POKEMON_CENTER_1F",
                "MAPSEC_VERMILION_CITY",
            ),
            Place::Celadon => ("MAP_CELADON_CITY_POKEMON_CENTER_1F", "MAPSEC_CELADON_CITY"),
            Place::Route18 => ("MAP_FUCHSIA_CITY_POKEMON_CENTER_1F", "MAPSEC_FUCHSIA_CITY"),
            Place::Cinnabar => (
                "MAP_CINNABAR_ISLAND_POKEMON_CENTER_1F",
                "MAPSEC_CINNABAR_ISLAND",
            ),
        }
    }
}

/// What an action does.
#[derive(Clone, Debug)]
pub enum Kind {
    /// Use `item` on `pre` (game species id).
    Stone { pre: u16, item: String },
    /// Rare Candies on `pre`, `n` of them.
    Candy { pre: u16, n: u32 },
    /// In-game trade: give `give`, get the target (`dex::special`, `SPECIALS`).
    Trade { give: u16 },
    /// Talk to an NPC who gives (or sells) the target (`dex::special`).
    Gift { price: u32 },
    /// Fossil revival at the Cinnabar lab.
    Fossil { item: String },
    /// Buy `item` at the Celadon store 4F, then use it on `pre`.
    BuyStone { pre: u16, item: String, price: u32 },
}

#[derive(Clone, Debug)]
pub struct Action {
    /// Target species name (`SPECIES_*`) and national number.
    pub target: String,
    pub dex: u32,
    pub place: Place,
    pub kind: Kind,
    /// Frames of the action itself (GUESS for ranking; the stone scene was measured at 1853,
    /// `journal/2026-11-22-13-15-dex-shop.md`).
    pub cost: u32,
}

/// In-game trades of this version: (got, requested, place). `decompiled/src/data/ingame_trades.h`
/// (`.species` / `.requestedSpecies`, FireRed vs LeafGreen under `#if defined(FIRERED)`), the
/// NPCs `setvar VAR_0x8008, INGAME_TRADE_*` in `data/maps/<map>/scripts.inc`.
fn trades(lg: bool) -> Vec<(&'static str, &'static str, Place)> {
    let mut v = vec![
        // `INGAME_TRADE_FARFETCHD`, `VermilionCity_House2/scripts.inc:7`.
        ("SPECIES_FARFETCHD", "SPECIES_SPEAROW", Place::Vermilion),
        // `INGAME_TRADE_ELECTRODE` / `_TANGELA`, `CinnabarIsland_PokemonLab_Lounge/scripts.inc:11,45`.
        ("SPECIES_ELECTRODE", "SPECIES_RAICHU", Place::Cinnabar),
        ("SPECIES_TANGELA", "SPECIES_VENONAT", Place::Cinnabar),
    ];
    if lg {
        // `INGAME_TRADE_NIDORINOA` (LG: Nidorino for Nidorina), `Route11_EastEntrance_2F/scripts.inc:25`;
        // `INGAME_TRADE_LICKITUNG` (LG: for Slowbro), `Route18_EastEntrance_2F/scripts.inc:15`.
        v.push(("SPECIES_NIDORINO", "SPECIES_NIDORINA", Place::Route11));
        v.push(("SPECIES_LICKITUNG", "SPECIES_SLOWBRO", Place::Route18));
    } else {
        v.push(("SPECIES_NIDORINA", "SPECIES_NIDORINO", Place::Route11));
        v.push(("SPECIES_LICKITUNG", "SPECIES_GOLDUCK", Place::Route18));
    }
    v
}

/// Stones the Celadon store's 4F sells (`decompiled/data/maps/CeladonCity_DepartmentStore_4F/scripts.inc`,
/// the clerk's `pokemart` list).
const CELADON_STONES: [&str; 4] = [
    "ITEM_FIRE_STONE",
    "ITEM_THUNDER_STONE",
    "ITEM_WATER_STONE",
    "ITEM_LEAF_STONE",
];

/// Level from experience (`decompiled/src/data/pokemon/experience_tables.h`, the `EXP_*` macros).
pub fn level_from_exp(growth: &str, exp: u32) -> u32 {
    let f = |n: i64| -> i64 {
        let c = n * n * n;
        match growth {
            "GROWTH_SLOW" => 5 * c / 4,
            "GROWTH_FAST" => 4 * c / 5,
            "GROWTH_MEDIUM_SLOW" => 6 * c / 5 - 15 * n * n + 100 * n - 140,
            "GROWTH_ERRATIC" => {
                if n <= 50 {
                    (100 - n) * c / 50
                } else if n <= 68 {
                    (150 - n) * c / 100
                } else if n <= 98 {
                    ((1911 - 10 * n) / 3) * c / 500
                } else {
                    (160 - n) * c / 100
                }
            }
            "GROWTH_FLUCTUATING" => {
                if n <= 15 {
                    ((n + 1) / 3 + 24) * c / 50
                } else if n <= 36 {
                    (n + 14) * c / 50
                } else {
                    ((n / 2) + 32) * c / 50
                }
            }
            _ => c,
        }
    };
    let mut lv = 1;
    while lv < 100 && exp as i64 >= f(lv as i64 + 1) {
        lv += 1;
    }
    lv
}

/// A mon the human has: game species, level, in the party (slot) or a box.
#[derive(Clone, Debug)]
pub struct Body {
    pub species: u16,
    pub level: u32,
    pub party: Option<usize>,
}

/// Party and boxes (`gPokemonStoragePtr`, `decompiled/include/pokemon_storage_system.h:44-50`;
/// a box mon's level is derived from its experience, `BoxPokemon` has no level).
pub fn bodies(s: &mut Session) -> Vec<Body> {
    let mut v: Vec<Body> = Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .enumerate()
        .map(|(i, m)| Body {
            species: m.species,
            level: m.level as u32,
            party: Some(i),
        })
        .collect();
    let p = s.sym("gPokemonStoragePtr");
    let st = s.hf.rec.emu().read32(p);
    let n = names();
    for b in 0..14u32 {
        for i in 0..30u32 {
            let a = st + 4 + (b * 30 + i) * 80;
            let emu = s.hf.rec.emu();
            if emu.read32(a) == 0 && emu.read32(a + 4) == 0 {
                continue;
            }
            let m = read_mon(emu, a);
            let name = n.species(m.species).to_string();
            let growth = s
                .plan
                .dex
                .species
                .iter()
                .find(|x| x.name == name)
                .map_or("GROWTH_MEDIUM_FAST".to_string(), |x| x.growth.clone());
            v.push(Body {
                species: m.species,
                level: level_from_exp(&growth, m.exp),
                party: None,
            });
        }
    }
    v
}

/// Every action that would add a species, from the RAM state.
pub fn candidates(s: &mut Session) -> Vec<Action> {
    let n = names();
    let own = owned_set(s);
    let lg = s.plan.params.version == "LG";
    let bodies = bodies(s);
    let money = Ram::new(&s.obs).money(s.hf.rec.emu());
    let dex = s.plan.dex;
    let dex_of = |name: &str| {
        dex.species
            .iter()
            .find(|x| x.name == name)
            .map_or(0, |x| x.dex)
    };
    let mut out = Vec::new();
    // Species reachable as bodies: owned bodies plus what a trade would bring (stone chains such as
    // the traded Nidorino -> Nidoking).
    let mut sources: Vec<(u16, u32, Option<usize>, Option<Place>)> = bodies
        .iter()
        .map(|b| (b.species, b.level, b.party, None))
        .collect();
    for (got, req, place) in trades(lg) {
        let d = dex_of(got);
        if d == 0 || own[d as usize] {
            continue;
        }
        let rid = id(req);
        if let Some(b) = bodies.iter().find(|b| b.species == rid) {
            out.push(Action {
                target: got.into(),
                dex: d,
                place,
                kind: Kind::Trade { give: rid },
                cost: 3200 + if b.party.is_some() { 0 } else { 700 },
            });
            sources.push((id(got), b.level, None, Some(place)));
        }
    }
    for (sp, lv, party, via) in &sources {
        let name = n.species(*sp).to_string();
        let Some(spc) = dex.species.iter().find(|x| x.name == name) else {
            continue;
        };
        for (k, param, t) in &spc.evo {
            let tgt = &dex.species[*t];
            if tgt.dex == 0 || tgt.dex > 151 || own[tgt.dex as usize] {
                continue;
            }
            let boxed = party.is_none() && via.is_none();
            match k.as_str() {
                "EVO_ITEM" => {
                    if have(s, param) > 0 {
                        out.push(Action {
                            target: tgt.name.clone(),
                            dex: tgt.dex,
                            place: via.unwrap_or(Place::Pc),
                            kind: Kind::Stone {
                                pre: *sp,
                                item: param.clone(),
                            },
                            cost: 1900 + if boxed { 700 } else { 0 },
                        });
                    } else if CELADON_STONES.contains(&param.as_str()) && via.is_none() {
                        let price = shop::price(s, id(param));
                        if price > 0 && price <= money {
                            out.push(Action {
                                target: tgt.name.clone(),
                                dex: tgt.dex,
                                place: Place::Celadon,
                                kind: Kind::BuyStone {
                                    pre: *sp,
                                    item: param.clone(),
                                    price,
                                },
                                cost: 2600 + if boxed { 2500 } else { 0 },
                            });
                        }
                    }
                }
                // A boxed mon whose species is also in the party is not a candy target: the bag's
                // candy goes on a party slot by species (the party Paras is the Cut user).
                "EVO_LEVEL"
                    if via.is_none()
                        && (party.is_some()
                            || !bodies.iter().any(|b| b.species == *sp && b.party.is_some())) =>
                {
                    let at: u32 = param.parse().unwrap_or(999);
                    let need = at.saturating_sub(*lv).max(1);
                    if need as u16 <= have(s, "ITEM_RARE_CANDY") {
                        out.push(Action {
                            target: tgt.name.clone(),
                            dex: tgt.dex,
                            place: Place::Pc,
                            kind: Kind::Candy { pre: *sp, n: need },
                            cost: 1900 + 400 * need + if boxed { 700 } else { 0 },
                        });
                    }
                }
                _ => {}
            }
        }
    }
    // The Magikarp salesman (`Route4_PokemonCenter_1F/scripts.inc:23-53`, `MAGIKARP_PRICE` 500).
    let d = dex_of("SPECIES_MAGIKARP");
    if d > 0 && !own[d as usize] && money >= 500 && !super::legs2::flag(s, "FLAG_BOUGHT_MAGIKARP") {
        out.push(Action {
            target: "SPECIES_MAGIKARP".into(),
            dex: d,
            place: Place::Route4,
            kind: Kind::Gift { price: 500 },
            cost: 1500,
        });
    }
    // Fossils in the bag (`CinnabarIsland_PokemonLab_ExperimentRoom/scripts.inc`: Helix -> Omanyte,
    // Dome -> Kabuto, Old Amber -> Aerodactyl, all `givemon ..., 5`).
    for (item, sp) in [
        ("ITEM_HELIX_FOSSIL", "SPECIES_OMANYTE"),
        ("ITEM_DOME_FOSSIL", "SPECIES_KABUTO"),
        ("ITEM_OLD_AMBER", "SPECIES_AERODACTYL"),
    ] {
        let d = dex_of(sp);
        if d > 0 && !own[d as usize] && have(s, item) > 0 {
            out.push(Action {
                target: sp.into(),
                dex: d,
                place: Place::Cinnabar,
                kind: Kind::Fossil { item: item.into() },
                cost: 2500,
            });
        }
    }
    out
}

/// The cheapest set of places whose actions add `need` species: every subset of the places that
/// have actions; within a subset each target once (its cheapest action), each bag item at most as
/// often as the bag holds it, money for purchases; plus one PC visit per three boxed mons the
/// actions need in the party. Returns the chosen actions in visit order.
pub fn plan(s: &mut Session, need: u32) -> Option<Vec<Action>> {
    let cands = candidates(s);
    plan_from(s, cands, need)
}

/// [`plan`] over a given candidate list.
fn plan_from(s: &mut Session, cands: Vec<Action>, need: u32) -> Option<Vec<Action>> {
    let mut places: Vec<Place> = cands.iter().map(|a| a.place).collect();
    places.sort();
    places.dedup();
    let money = Ram::new(&s.obs).money(s.hf.rec.emu());
    let mut bag: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
    for a in &cands {
        match &a.kind {
            Kind::Stone { item, .. } => {
                let c = have(s, item);
                bag.insert(item.clone(), c);
            }
            Kind::Candy { .. } => {
                let c = have(s, "ITEM_RARE_CANDY");
                bag.insert("ITEM_RARE_CANDY".into(), c);
            }
            _ => {}
        }
    }
    let in_party: std::collections::HashSet<u16> = bodies(s)
        .iter()
        .filter(|b| b.party.is_some())
        .map(|b| b.species)
        .collect();
    let mut best: Option<(u32, Vec<Action>)> = None;
    for mask in 1u32..(1 << places.len()) {
        let inc: Vec<Place> = (0..places.len())
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| places[i])
            .collect();
        let mut acts: Vec<&Action> = cands.iter().filter(|a| inc.contains(&a.place)).collect();
        acts.sort_by_key(|a| a.cost);
        let mut left = bag.clone();
        let mut cash = money;
        let mut got = std::collections::HashSet::new();
        let mut chosen = Vec::new();
        for a in acts {
            if got.contains(&a.dex) {
                continue;
            }
            match &a.kind {
                Kind::Stone { item, .. } => {
                    let c = left.entry(item.clone()).or_default();
                    if *c == 0 {
                        continue;
                    }
                    *c -= 1;
                }
                Kind::Candy { n, .. } => {
                    let c = left.entry("ITEM_RARE_CANDY".into()).or_default();
                    if (*c as u32) < *n {
                        continue;
                    }
                    *c -= *n as u16;
                }
                Kind::Gift { price } | Kind::BuyStone { price, .. } => {
                    if cash < *price {
                        continue;
                    }
                    cash -= price;
                }
                _ => {}
            }
            got.insert(a.dex);
            chosen.push(a.clone());
            if got.len() as u32 >= need {
                break;
            }
        }
        if (got.len() as u32) < need {
            continue;
        }
        let used: std::collections::HashSet<Place> = chosen.iter().map(|a| a.place).collect();
        let withdraws = chosen
            .iter()
            .filter(|a| match &a.kind {
                Kind::Stone { pre, .. } | Kind::Candy { pre, .. } => !in_party.contains(pre),
                Kind::Trade { give } => !in_party.contains(give),
                _ => false,
            })
            .count() as u32;
        let cost: u32 = used.iter().map(|p| p.cost()).sum::<u32>()
            + chosen.iter().map(|a| a.cost).sum::<u32>()
            + withdraws.div_ceil(3) * 2000;
        if best.as_ref().is_none_or(|b| cost < b.0) {
            best = Some((cost, chosen));
        }
    }
    let (_, mut v) = best?;
    v.sort_by_key(|a| Place::ORDER.iter().position(|p| *p == a.place));
    Some(v)
}

/// Party slots the errands never deposit: the lead (the fighter) and the Fly and Cut users (the
/// walk to every place may need them).
pub fn keep_slots(s: &mut Session) -> Vec<usize> {
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let fly = id("MOVE_FLY");
    let cut = id("MOVE_CUT");
    let mut v = vec![0];
    for mv in [fly, cut, id("MOVE_SURF"), id("MOVE_STRENGTH")] {
        if let Some(i) = party.iter().position(|m| m.knows(mv).is_some()) {
            if !v.contains(&i) {
                v.push(i);
            }
        }
    }
    // The pre-evolution of a stone evolution the plan still does from the party later
    // (`dex::evolve` uses party mons only: Eevee for the Jolteon at erika-badge).
    let own = owned_set(s);
    let pending: Vec<crate::params::Evolve> = s
        .plan
        .params
        .evolutions
        .iter()
        .filter(|e| e.via == "stone")
        .cloned()
        .collect();
    for e in &pending {
        let t = s.plan.dex.id(&e.species);
        if own[s.plan.dex.species[t].dex as usize] {
            continue;
        }
        for (i, m) in party.iter().enumerate() {
            let name = names().species(m.species).to_string();
            let Some(sp) = s.plan.dex.species.iter().find(|x| x.name == name) else {
                continue;
            };
            let d = sp.dex;
            if crate::sim::evo_source(&s.ctx, e, &|x| x == d).is_some() && !v.contains(&i) {
                v.push(i);
            }
        }
    }
    v.truncate(5);
    v
}

/// The party species the fill left (logged by the first `dex-finish` stage, read back so a
/// resumed run restores the same party).
fn party0(s: &mut Session) -> Vec<u16> {
    if let Some(v) = s
        .log
        .events
        .iter()
        .find(|e| e.kind == "decision" && e.text.starts_with("dex finish: party before"))
        .and_then(|e| e.nums.get("party").and_then(|p| p.as_array()).cloned())
    {
        return v
            .iter()
            .filter_map(|x| x.as_u64())
            .map(|x| x as u16)
            .collect();
    }
    let p: Vec<u16> = Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .map(|m| m.species)
        .collect();
    let names: Vec<String> = p
        .iter()
        .map(|&x| {
            names()
                .species(x)
                .trim_start_matches("SPECIES_")
                .to_string()
        })
        .collect();
    s.event(
        "decision",
        format!("dex finish: party before the errands {}", names.join(", ")),
        json!({"party": p}),
    );
    p
}

/// Get somewhere Fly works and fly to `mapsec` unless already on `map`'s side of it.
fn fly_to_center(s: &mut Session, center: &str, mapsec: &str) -> PResult<()> {
    let g = s.graph();
    let cid = g.id(center);
    if here(s).is_some_and(|n| n.0 == cid) {
        return Ok(());
    }
    // Close by on foot (the Route 4 center from Mt. Moon): the next walk goes there.
    let walk = super::walk::plan_len(
        s,
        None,
        &move |n: &super::walk::Node| n.0 == cid,
        &super::legs2::hm_opts(),
    );
    if walk.is_some_and(|l| l < 150) {
        return Ok(());
    }
    super::legs3::leave_to_fly(s, center)?;
    super::items::fly(s, mapsec)
}

/// Make the party hold `want` (game species, from the boxes), depositing mons not in the keep
/// slots and not in `want`, at `center`. Deposits highest slot first (slots shift down).
fn arrange(s: &mut Session, center: &str, want: &[u16]) -> PResult<()> {
    let party: Vec<u16> = Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .map(|m| m.species)
        .collect();
    let mut missing: Vec<u16> = Vec::new();
    {
        let mut have_p = party.clone();
        for &w in want {
            if let Some(i) = have_p.iter().position(|&x| x == w) {
                have_p[i] = 0;
            } else {
                missing.push(w);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let keep = keep_slots(s);
    let mut free = 6usize.saturating_sub(party.len());
    let mut deps = Vec::new();
    let mut wanted = want.to_vec();
    for (i, &sp) in party.iter().enumerate().rev() {
        if free >= missing.len() {
            break;
        }
        if keep.contains(&i) {
            continue;
        }
        if let Some(k) = wanted.iter().position(|&w| w == sp) {
            wanted.remove(k);
            continue;
        }
        deps.push(PcOp::Deposit(i));
        free += 1;
    }
    let mut ops = deps;
    ops.extend(missing.iter().take(free).map(|&m| PcOp::Withdraw(m)));
    s.event(
        "decision",
        format!(
            "PC at {}: {}",
            center.trim_start_matches("MAP_"),
            ops.iter()
                .map(|o| match o {
                    PcOp::Deposit(i) => format!(
                        "deposit {}",
                        names().species(party[*i]).trim_start_matches("SPECIES_")
                    ),
                    PcOp::Withdraw(sp) => format!(
                        "withdraw {}",
                        names().species(*sp).trim_start_matches("SPECIES_")
                    ),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        json!({"center": center}),
    );
    shop::pc(s, center, &ops)
}

fn party_has(s: &mut Session, sp: u16) -> Option<usize> {
    dex::party_slot(s, sp)
}

/// Stone and candy evolutions (the `Pc` place): in batches that fit the party.
fn do_evolutions(s: &mut Session, acts: &[Action], center: &str) -> PResult<()> {
    let mut todo: Vec<Action> = acts.to_vec();
    while !todo.is_empty() {
        let keep = keep_slots(s).len();
        let room = 6 - keep;
        let batch: Vec<Action> = todo.drain(..todo.len().min(room)).collect();
        let want: Vec<u16> = batch
            .iter()
            .filter_map(|a| match &a.kind {
                Kind::Stone { pre, .. } | Kind::Candy { pre, .. } => Some(*pre),
                _ => None,
            })
            .collect();
        arrange(s, center, &want)?;
        for a in &batch {
            evolve_one(s, a)?;
        }
    }
    Ok(())
}

fn evolve_one(s: &mut Session, a: &Action) -> PResult<()> {
    let (pre, item, k) = match &a.kind {
        Kind::Stone { pre, item } | Kind::BuyStone { pre, item, .. } => (*pre, item.clone(), 1),
        Kind::Candy { pre, n } => (*pre, "ITEM_RARE_CANDY".to_string(), *n),
        _ => return Ok(()),
    };
    let Some(slot) = party_has(s, pre) else {
        return Err(PlayError::Fatal(format!(
            "{} not in the party for {}",
            names().species(pre),
            a.target
        )));
    };
    let tid = id(&a.target);
    for _ in 0..k {
        let now = shop::use_item_on(s, &item, slot)?;
        if now == tid {
            break;
        }
    }
    let ok = dex::owned(s, a.dex);
    let n = caught_count(s);
    s.event(
        if ok { "catch" } else { "model-vs-actual" },
        format!(
            "{} -> {}: {} ({n} caught)",
            names().species(pre).trim_start_matches("SPECIES_"),
            a.target.trim_start_matches("SPECIES_"),
            if ok { "evolved" } else { "did NOT evolve" }
        ),
        json!({"species": a.target, "evolved": ok, "caught": n}),
    );
    if ok {
        Ok(())
    } else {
        Err(PlayError::Fatal(format!("{} did not evolve", a.target)))
    }
}

fn npc(s: &mut Session, a: &Action) -> PResult<()> {
    let (kind, requires) = match &a.kind {
        Kind::Trade { give } => ("trade", vec![names().species(*give).to_string()]),
        _ => ("gift", vec![]),
    };
    let sp = Special {
        after: "dex-finish".into(),
        kind: kind.into(),
        species: vec![a.target.clone()],
        requires,
        ..Default::default()
    };
    if !dex::special(s, &sp)? {
        return Err(PlayError::Fatal(format!("{kind} for {} failed", a.target)));
    }
    Ok(())
}

/// The fossil scientist (`CinnabarIsland_PokemonLab_ExperimentRoom_EventScript_FossilScientist`):
/// show the fossil (the multichoice's first row is the only fossil in the bag, YES), "go for a
/// walk": stepping into the Entrance sets `..._REVIVE_STATE` 2 (`CinnabarIsland_PokemonLab_Entrance/
/// scripts.inc:6-10`, `OnTransition`), back, talk again: `givemon` (to the PC with a full party,
/// `ScriptGiveMon` sets the caught flag either way, `decompiled/src/script_pokemon_util.c:48-73`),
/// nickname NO.
fn fossil(s: &mut Session, a: &Action) -> PResult<()> {
    let map = "MAP_CINNABAR_ISLAND_POKEMON_LAB_EXPERIMENT_ROOM";
    let local = super::legs3::obj_id(s, map, "FossilScientist")?;
    let state = |s: &mut Session| {
        super::legs2::var(
            s,
            "VAR_MAP_SCENE_CINNABAR_ISLAND_POKEMON_LAB_EXPERIMENT_ROOM_REVIVE_STATE",
        )
    };
    if state(s) == 0 {
        super::legs::talk_to(s, map, local)?;
        dex::run_npc(s, a.dex, None)?;
        if state(s) != 1 {
            return Err(PlayError::Death(
                "the scientist did not take the fossil".into(),
            ));
        }
    }
    if state(s) == 1 {
        go_map(
            s,
            "MAP_CINNABAR_ISLAND_POKEMON_LAB_ENTRANCE",
            &super::legs2::hm_opts(),
        )?;
    }
    super::legs::talk_to(s, map, local)?;
    dex::run_npc(s, a.dex, None)?;
    let ok = dex::owned(s, a.dex);
    let n = caught_count(s);
    s.event(
        if ok { "catch" } else { "model-vs-actual" },
        format!(
            "fossil {}: {} ({n} caught)",
            a.target.trim_start_matches("SPECIES_"),
            if ok { "revived" } else { "NOT revived" }
        ),
        json!({"species": a.target, "obtained": ok, "caught": n}),
    );
    if ok {
        Ok(())
    } else {
        Err(PlayError::Death("fossil not revived".into()))
    }
}

/// One `dex-finish-N` stage: re-plan from RAM, do the first pending place.
pub fn finish_round(s: &mut Session) -> PResult<()> {
    s.seg = "oak-national-dex".into();
    let n = caught_count(s);
    if n >= 60 || super::legs2::var(s, "VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB") >= 8 {
        return Ok(());
    }
    let p0 = party0(s);
    let _ = p0;
    let Some(acts) = plan(s, 60 - n) else {
        return Err(PlayError::Fatal(format!(
            "dex finish: {n} caught and no set of evolutions / trades / gifts reaches 60"
        )));
    };
    let money = Ram::new(&s.obs).money(s.hf.rec.emu());
    s.event(
        "decision",
        format!(
            "dex finish plan ({n} caught, {} to go): {}",
            60 - n,
            acts.iter()
                .map(|a| format!("{} @{:?}", a.target.trim_start_matches("SPECIES_"), a.place))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        json!({"caught": n, "money": money, "plan": acts.iter().map(|a| json!({"target": a.target, "place": format!("{:?}", a.place), "kind": format!("{:?}", a.kind)})).collect::<Vec<_>>()}),
    );
    if !s
        .log
        .events
        .iter()
        .any(|e| e.kind == "model-vs-actual" && e.text.starts_with("dex finish:"))
    {
        s.event(
            "model-vs-actual",
            "dex finish: the model's 60 are caught before the League; the played fill ends with evolutions, trades, a gift and a fossil after the hunts (not modelled here)",
            json!({"caught": n}),
        );
    }
    let first = acts[0].place;
    // The Pc place happens at the first real place's center (or Route 4's when it is the only one).
    let real = acts
        .iter()
        .map(|a| a.place)
        .find(|p| *p != Place::Pc)
        .unwrap_or(Place::Route4);
    let (center, sec) = real.center();
    let here_acts: Vec<Action> = acts
        .iter()
        .filter(|a| a.place == first || (first == Place::Pc && a.place == real))
        .cloned()
        .collect();
    fly_to_center(s, center, sec)?;
    // Evolutions first (they free the party for the trade mons).
    let evos: Vec<Action> = here_acts
        .iter()
        .filter(|a| {
            matches!(a.kind, Kind::Stone { .. } | Kind::Candy { .. }) && a.place == Place::Pc
        })
        .cloned()
        .collect();
    if !evos.is_empty() {
        do_evolutions(s, &evos, center)?;
    }
    // Trade mons for this place and every later one, as far as the party allows.
    let gives: Vec<u16> = acts
        .iter()
        .filter_map(|a| match &a.kind {
            Kind::Trade { give } => Some(*give),
            _ => None,
        })
        .collect();
    let mine: Vec<u16> = here_acts
        .iter()
        .filter_map(|a| match &a.kind {
            Kind::Trade { give } => Some(*give),
            _ => None,
        })
        .collect();
    let missing_mine = mine.iter().any(|g| party_has(s, *g).is_none());
    if missing_mine || (first == Place::Pc && !gives.is_empty()) {
        let room = 6 - keep_slots(s).len();
        let mut want = mine.clone();
        for g in &gives {
            if want.len() >= room {
                break;
            }
            if !want.contains(g) {
                want.push(*g);
            }
        }
        // A trade the party cannot hold yet waits for the next place's center.
        let (c, sc) = first.center();
        if first != Place::Pc {
            fly_to_center(s, c, sc)?;
        }
        arrange(s, if first == Place::Pc { center } else { c }, &want)?;
    }
    let mut order = here_acts.clone();
    order.sort_by_key(|a| match a.kind {
        Kind::Gift { .. } => 0,
        Kind::Trade { .. } => 1,
        Kind::BuyStone { .. } => 2,
        Kind::Stone { .. } | Kind::Candy { .. } => 3,
        Kind::Fossil { .. } => 4,
    });
    for a in &order {
        if dex::owned(s, a.dex) {
            continue;
        }
        match &a.kind {
            Kind::Gift { .. } | Kind::Trade { .. } => npc(s, a)?,
            Kind::Fossil { .. } => fossil(s, a)?,
            Kind::BuyStone { item, pre, .. } => {
                shop::buy(
                    s,
                    "MAP_CELADON_CITY_DEPARTMENT_STORE_4F",
                    None,
                    &[(item.as_str(), 1)],
                )?;
                if party_has(s, *pre).is_none() {
                    arrange(s, "MAP_CELADON_CITY_POKEMON_CENTER_1F", &[*pre])?;
                }
                evolve_one(s, a)?;
            }
            Kind::Stone { .. } | Kind::Candy { .. } if a.place != Place::Pc => {
                // A stone on a mon this place's trade just brought (Nidorino -> Nidoking).
                evolve_one(s, a)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// After the last errand: the party the fill left (Lapras for Strength / Waterfall, the Cut and
/// Fly users), at the nearest center.
pub fn finish_party(s: &mut Session) -> PResult<()> {
    s.seg = "oak-national-dex".into();
    if super::legs2::var(s, "VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB") >= 8 {
        return Ok(());
    }
    let p0 = party0(s);
    let party: Vec<u16> = Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .map(|m| m.species)
        .collect();
    let mut want = Vec::new();
    for &sp in &p0 {
        want.push(sp);
    }
    let missing = want.iter().filter(|w| !party.contains(w)).count();
    if missing == 0 {
        return Ok(());
    }
    // The nearest center: where the last errand was (Cinnabar, Fuchsia, Vermilion, Route 4).
    let g = s.graph();
    let here_map = here(s)
        .map(|n| g.maps[&n.0].name.clone())
        .unwrap_or_default();
    let center = [
        ("Cinnabar", Place::Cinnabar),
        ("Fuchsia", Place::Route18),
        ("Route18", Place::Route18),
        ("Celadon", Place::Celadon),
        ("Vermilion", Place::Vermilion),
        ("Route11", Place::Vermilion),
    ]
    .iter()
    .find(|(k, _)| here_map.contains(k))
    .map_or(Place::Vermilion, |x| x.1);
    let (c, sc) = center.center();
    fly_to_center(s, c, sc)?;
    // Deposit everything not in the old party (keep slots stay), withdraw the old party.
    let keep = keep_slots(s);
    let mut ops = Vec::new();
    let mut need: Vec<u16> = want.clone();
    for (i, sp) in party.iter().enumerate() {
        if let Some(k) = need.iter().position(|w| w == sp) {
            need.remove(k);
        } else if !keep.contains(&i) {
            ops.push(i);
        }
    }
    let mut pc_ops: Vec<PcOp> = ops.iter().rev().map(|&i| PcOp::Deposit(i)).collect();
    let free = 6 - party.len() + ops.len();
    pc_ops.extend(need.iter().take(free).map(|&m| PcOp::Withdraw(m)));
    s.event(
        "decision",
        format!(
            "PC: the party back for the post-game ({} out, {} in)",
            ops.len(),
            need.len().min(free)
        ),
        json!({"center": c}),
    );
    shop::pc(s, c, &pc_ops)?;
    let _ = have(s, "ITEM_POKE_BALL");
    Ok(())
}

/// v3: the dex work a human does while passing a Pokémon Center on the main route (not the
/// post-game): the same RAM planner as the dex finish ([`candidates`]), restricted to the
/// party/PC (`Place::Pc`: stones and Rare Candies already in the bag, on party or boxed mons) and
/// to `place` (the town's trades, fossils, stones bought there). Every species any such action
/// adds is taken, not just enough for 60: the plan's dex is due at the first Hall of Fame. Stone
/// purchases keep `reserve` yen in hand (the League's heals). Eevee is left alone (the plan's
/// own Thunder Stone evolution at erika-badge), and Rare Candy evolutions are taken only when
/// they need at most two candies (the rest go to the carry, `candy_early`). Up to three passes
/// (an evolution makes a trade possible: Pikachu -> Raichu -> Electrode). Returns the species
/// added.
pub fn errands(s: &mut Session, center: &str, place: Option<Place>, reserve: u32) -> PResult<u32> {
    let n0 = caught_count(s);
    let eevee = id("SPECIES_EEVEE");
    for pass in 0..3 {
        let money = Ram::new(&s.obs).money(s.hf.rec.emu());
        let cands: Vec<Action> = candidates(s)
            .into_iter()
            .filter(|a| a.place == Place::Pc || Some(a.place) == place)
            .filter(|a| match &a.kind {
                Kind::Stone { pre, .. } => *pre != eevee,
                Kind::BuyStone { pre, price, .. } => *pre != eevee && money >= price + reserve,
                Kind::Candy { n, .. } => *n <= 2,
                Kind::Gift { .. } => place == Some(Place::Route4),
                _ => true,
            })
            .collect();
        let mut targets: Vec<u32> = cands.iter().map(|a| a.dex).collect();
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            break;
        }
        let mut acts = None;
        for need in (1..=targets.len() as u32).rev() {
            if let Some(v) = plan_from(s, cands.clone(), need) {
                acts = Some(v);
                break;
            }
        }
        let Some(acts) = acts else { break };
        let desc: Vec<String> = acts
            .iter()
            .map(|a| {
                let how = match &a.kind {
                    Kind::Stone { item, .. } | Kind::BuyStone { item, .. } => {
                        item.trim_start_matches("ITEM_").to_string()
                    }
                    Kind::Candy { n, .. } => format!("{n} Rare Candy"),
                    Kind::Trade { give } => format!(
                        "trade {}",
                        names().species(*give).trim_start_matches("SPECIES_")
                    ),
                    Kind::Gift { .. } => "gift".into(),
                    Kind::Fossil { .. } => "fossil".into(),
                };
                format!("{} ({how})", a.target.trim_start_matches("SPECIES_"))
            })
            .collect();
        let caught = caught_count(s);
        s.event(
            "decision",
            format!(
                "dex errands at {} (pass {}): {}",
                center.trim_start_matches("MAP_"),
                pass + 1,
                desc.join(", ")
            ),
            json!({"center": center, "caught": caught, "money": money,
                   "plan": acts.iter().map(|a| json!({"target": a.target, "place": format!("{:?}", a.place), "kind": format!("{:?}", a.kind)})).collect::<Vec<_>>()}),
        );
        execute(s, &acts, center)?;
        if caught_count(s) == caught {
            break;
        }
    }
    let n = caught_count(s);
    if n > n0 {
        s.event(
            "model-vs-actual",
            format!(
                "dex errands at {}: +{} species ({n} caught); the model banks these at its own anchors",
                center.trim_start_matches("MAP_"),
                n - n0
            ),
            json!({"center": center, "gained": n - n0, "caught": n}),
        );
    }
    Ok(n - n0)
}

/// Do `acts` (all in `center`'s town): Pc evolutions in party-sized batches, the trades (the
/// mons to give withdrawn first), gifts, stones bought and used, fossils.
fn execute(s: &mut Session, acts: &[Action], center: &str) -> PResult<()> {
    let is_pc_evo =
        |a: &Action| matches!(a.kind, Kind::Stone { .. } | Kind::Candy { .. }) && a.place == Place::Pc;
    let evos: Vec<Action> = acts.iter().filter(|a| is_pc_evo(a)).cloned().collect();
    if !evos.is_empty() {
        do_evolutions(s, &evos, center)?;
    }
    let gives: Vec<u16> = acts
        .iter()
        .filter_map(|a| match &a.kind {
            Kind::Trade { give } => Some(*give),
            _ => None,
        })
        .collect();
    if gives.iter().any(|g| party_has(s, *g).is_none()) {
        let room = 6 - keep_slots(s).len();
        let want: Vec<u16> = gives.iter().copied().take(room).collect();
        arrange(s, center, &want)?;
    }
    let mut order: Vec<Action> = acts.iter().filter(|a| !is_pc_evo(a)).cloned().collect();
    order.sort_by_key(|a| match a.kind {
        Kind::Gift { .. } => 0,
        Kind::Trade { .. } => 1,
        Kind::BuyStone { .. } => 2,
        Kind::Stone { .. } | Kind::Candy { .. } => 3,
        Kind::Fossil { .. } => 4,
    });
    for a in &order {
        if dex::owned(s, a.dex) {
            continue;
        }
        match &a.kind {
            Kind::Gift { .. } => npc(s, a)?,
            Kind::Trade { give } => {
                if party_has(s, *give).is_none() {
                    arrange(s, center, &[*give])?;
                }
                npc(s, a)?
            }
            Kind::Fossil { .. } => fossil(s, a)?,
            Kind::BuyStone { item, pre, .. } => {
                if have(s, item) == 0 {
                    shop::buy(
                        s,
                        "MAP_CELADON_CITY_DEPARTMENT_STORE_4F",
                        Some((3, 13)),
                        &[(item.as_str(), 1)],
                    )?;
                }
                if party_has(s, *pre).is_none() {
                    arrange(s, center, &[*pre])?;
                }
                evolve_one(s, a)?;
            }
            Kind::Stone { .. } | Kind::Candy { .. } => evolve_one(s, a)?,
        }
    }
    Ok(())
}
