//! The route from the Route 4 Pokémon Center (`mt-moon-pc`, the last stage of `play/early.rs`) to
//! Giovanni 1, as stages, in the CURRENT plan's segment order (`route_wr.rs` `wr_order` with
//! `leg_pewter_cerulean` .. `leg_celadon` of `route.rs`): Mt. Moon (Robby, the Rocket, Miguel, the
//! Helix Fossil), Route 4's Mega Kick tutor, Cerulean (heal, Rare Candies, mart), Misty and TM03,
//! the Cerulean rival, Nugget Bridge, Route 25 and Bill, the walk back and the Cerulean Rocket
//! (`dig_after_bill`), Routes 5-6 and the Fan Club voucher, the S.S. Anne rival and HM01, the walk
//! back to the Cerulean bike shop, Cut, Route 9, Rock Tunnel, Lavender, Routes 8-7, Celadon (X
//! items, the Nugget sale, Eevee, the PP Up, the Tea), the Rocket Hideout and Giovanni.
//!
//! Every battle goes through the sim's policy (`fallback_battle::battle` -> `battle::fight`, keyed by
//! the plan segment in `Session::seg`); catches through `dex::wild_battle`'s pass rule; teaches through
//! `sim::replace_slot`. Trainers the plan fights (`fight_lists`, `extra_fights`, the audited forced
//! sets) are walked up to and talked to; every other trainer is left to the walker's sight-line walls.
//! Where the played game differs from what the model assumes, a `model-vs-actual` event says so.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::legs::{center_heal, split, talk_to};
use super::legs2::{flag, have, var};
use super::names::{id, names};
use super::prims::settle;
use super::ram::Ram;
use super::session::{PResult, PlayError, Session, Stage};
use super::walk::{go, go_face, go_map, go_to, here, plan_nodes, MapId, Node, Opts};

/// Cut usable; Saffron's gates are shut until the Tea (`Route5_SouthEntrance/scripts.inc:39`
/// etc.: the guard takes `ITEM_TEA`), so the planner may not route through Saffron before it.
fn cut_opts() -> Opts {
    Opts {
        cut: true,
        avoid: vec!["MAP_SAFFRON_CITY"],
        ..Opts::default()
    }
}

fn short(name: &str) -> &str {
    name.trim_start_matches("SPECIES_")
        .trim_start_matches("ITEM_")
        .trim_start_matches("MAP_")
        .trim_start_matches("TRAINER_")
        .trim_start_matches("MOVE_")
}

// ---------------------------------------------------------------------------------------------
// Trainers

/// The trainer's beaten flag: `TRAINER_FLAGS_START` + its id (`decompiled/include/constants/flags.h`,
/// `GetTrainerFlag`, `decompiled/src/battle_setup.c`).
fn trainer_flag(trainer: &str) -> Option<u16> {
    let n = names();
    Some(n.get("TRAINER_FLAGS_START")? + n.get(trainer)?)
}

pub fn beaten(s: &mut Session, trainer: &str) -> bool {
    trainer_flag(trainer)
        .and_then(|f| s.obs.flag(s.hf.rec.emu(), f))
        .unwrap_or(false)
}

/// The object event that fights `trainer`: (map id, local id, x, y) from the maps' `object_events`
/// (`walk::ObjInfo::beaten` is the flag of the first `trainerbattle` of its script).
fn trainer_obj(s: &Session, trainer: &str) -> Option<(String, u8, i16, i16)> {
    let f = trainer_flag(trainer)?;
    let g = s.graph();
    let mut found: Vec<(String, u8, i16, i16)> = g
        .maps
        .values()
        .flat_map(|m| {
            m.objects
                .iter()
                .filter(|o| o.beaten == f)
                .map(|o| (m.id.clone(), o.local_id, o.x, o.y))
                .collect::<Vec<_>>()
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Where object `local_id` of the current map stands now (`gObjectEvents`), if it is loaded.
fn live_pos(s: &mut Session, map: MapId, local_id: u8) -> Option<(i16, i16)> {
    if here(s).map(|n| n.0) != Some(map) {
        return None;
    }
    Ram::new(&s.obs)
        .objects(s.hf.rec.emu())
        .iter()
        .find(|l| l.local_id == local_id)
        .map(|l| (l.x, l.y))
}

/// Fight `trainer` (a plan fight: `fight_lists`, `extra_fights` or an audited forced trainer):
/// walk up to it (a sight line on the way starts the battle the same), face it, talk. The battle is
/// the sim's policy for plan segment `seg`. Already beaten: nothing.
pub fn fight_trainer(s: &mut Session, trainer: &str, seg: &str, o: &Opts) -> PResult<()> {
    if beaten(s, trainer) {
        return Ok(());
    }
    let Some((map, lid, x0, y0)) = trainer_obj(s, trainer) else {
        return Err(PlayError::Fatal(format!("no object fights {trainer}")));
    };
    let prev = std::mem::replace(&mut s.seg, seg.to_string());
    let mid = s.graph().id(&map);
    let t0 = s.frame();
    for _ in 0..6 {
        if beaten(s, trainer) {
            break;
        }
        let (x, y) = live_pos(s, mid, lid).unwrap_or((x0, y0));
        go_face(s, &map, x, y, o)?;
        if beaten(s, trainer) {
            break;
        }
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    s.seg = prev;
    if !beaten(s, trainer) {
        return Err(PlayError::Death(format!(
            "soft-lock: could not fight {trainer}"
        )));
    }
    s.event(
        "decision",
        format!(
            "fought {} (plan fight, segment {seg})",
            short(trainer).replace('_', " ")
        ),
        json!({"trainer": trainer, "seg": seg, "frames": s.frame() - t0}),
    );
    Ok(())
}

/// The trainers of plan segment `seg` the plan fights: its `fight_lists` entry if there is one,
/// else `extra` (the audited forced set as `route.rs` lists it).
fn plan_fights(s: &Session, seg: &str, extra: &[&str]) -> Vec<String> {
    match s.plan.params.fight_lists.get(seg) {
        Some(v) => v.clone(),
        None => extra.iter().map(|t| t.to_string()).collect(),
    }
}

// ---------------------------------------------------------------------------------------------
// Repels

/// Does the planned path from here to `goal` cross a tile where land encounters roll
/// (`frlg_route::world::Tile::land`, metatile attribute `ENCOUNTER_LAND`)?
fn path_has_encounters(s: &mut Session, nodes: &[Node]) -> usize {
    let g = s.graph();
    nodes
        .iter()
        .filter(|n| {
            g.maps
                .get(&n.0)
                .and_then(|m| m.tile(n.1, n.2))
                .is_some_and(|t| t.land)
        })
        .count()
}

/// A Repel before a stretch of plan segment `seg` when the plan's `repel_on` lists it, none is
/// active (`VAR_REPEL_STEP_COUNT`, `decompiled/src/item_use.c:552-567`) and the planned path on
/// this map crosses encounter tiles (review item 14: no Repel on a path without grass).
fn repel_for(s: &mut Session, seg: &str, nodes: &[Node]) -> PResult<()> {
    if !s.plan.params.repel_on.iter().any(|r| r == seg) {
        return Ok(());
    }
    if var(s, "VAR_REPEL_STEP_COUNT") > 0 {
        return Ok(());
    }
    let enc = path_has_encounters(s, nodes);
    if enc == 0 {
        return Ok(());
    }
    let item = ["ITEM_REPEL", "ITEM_SUPER_REPEL", "ITEM_MAX_REPEL"]
        .into_iter()
        .find(|it| have(s, it) > 0);
    let Some(item) = item else {
        let said = format!("plan repel_on {seg}: no Repel in the bag");
        if s.log.events.iter().any(|e| e.text.starts_with(&said)) {
            return Ok(());
        }
        s.event(
            "model-vs-actual",
            format!("plan repel_on {seg}: no Repel in the bag ({enc} encounter tiles ahead)"),
            json!({"seg": seg, "tiles": enc}),
        );
        return Ok(());
    };
    s.event(
        "decision",
        format!("Repel for {seg} (plan repel_on): {enc} encounter tiles on the path ahead"),
        json!({"seg": seg, "tiles": enc, "item": item}),
    );
    super::shop::use_item(s, item)
}

/// [`plan_nodes`] around every unbeaten trainer's sight line when there is a way, else the plain
/// plan: the order `go_to` walks in (`walk.rs`, "Around every unbeaten trainer's sight line"). A
/// waypoint taken from the plain plan can lie behind a skipped trainer's line that the walked route
/// avoids (r2i walked into Lass Miriam and Grunt 4 that way).
fn plan_nodes_walled(
    s: &mut Session,
    from: Node,
    goal: &dyn Fn(&Node) -> bool,
    o: &Opts,
) -> Option<Vec<Node>> {
    let walls = Opts {
        cone_walls: true,
        ..o.clone()
    };
    plan_nodes(s, from, goal, &walls).or_else(|| plan_nodes(s, from, goal, o))
}

/// The Repel runs out on the path `nodes` (`VAR_REPEL_STEP_COUNT` counts every step down,
/// `decompiled/src/wild_encounter.c:576-598`; Repel 100 steps, Super 200, Max 250,
/// `decompiled/src/data/items.json` `holdEffectParam`): walk to the tile where it ends so the next
/// loop turn re-uses one, when encounter tiles lie beyond it. `true` when it walked.
fn repel_waypoint(
    s: &mut Session,
    cur_id: &str,
    rs: &str,
    nodes: &[Node],
    o: &Opts,
) -> PResult<bool> {
    let left = var(s, "VAR_REPEL_STEP_COUNT") as usize;
    let on_plan = s.plan.params.repel_on.iter().any(|r| r == rs);
    if !on_plan || left == 0 || left + 1 >= nodes.len() {
        return Ok(false);
    }
    if path_has_encounters(s, &nodes[left..]) == 0 {
        return Ok(false);
    }
    let wp = nodes[left];
    go_to(
        s,
        &format!("{cur_id} ({}, {}): where the Repel ends", wp.1, wp.2),
        &move |n: &Node| *n == wp,
        o,
    )?;
    Ok(true)
}

/// Walk to `dest` one map at a time; on each map, first the plan's Repel for that map's segment
/// (`seg_of(map id)`), then the trainers of `fights` that stand on this map, nearest first.
fn travel(
    s: &mut Session,
    dest: &str,
    o: &Opts,
    seg_of: &dyn Fn(&str) -> Option<&'static str>,
    fights: &[(String, String)],
) -> PResult<()> {
    travel_to(s, dest, None, o, seg_of, fights)
}

/// [`travel`] to the tile `at` of `dest` (anywhere on it when `None`).
fn travel_to(
    s: &mut Session,
    dest: &str,
    at: Option<(i16, i16)>,
    o: &Opts,
    seg_of: &dyn Fn(&str) -> Option<&'static str>,
    fights: &[(String, String)],
) -> PResult<()> {
    let dmid = s.graph().id(dest);
    let arrived = move |n: &Node| n.0 == dmid && at.is_none_or(|(x, y)| n.1 == x && n.2 == y);
    for _ in 0..80 {
        settle(s)?;
        let Some(cur) = here(s) else {
            return Err(PlayError::Fatal("no position".into()));
        };
        let cur_id = s.graph().maps[&cur.0].id.clone();
        // The plan's fights on this map (nearest by path first).
        let mut todo: Vec<(usize, String, String)> = vec![];
        for (tr, seg) in fights {
            if beaten(s, tr) {
                continue;
            }
            if let Some((m, _, x, y)) = trainer_obj(s, tr) {
                if m == cur_id {
                    let mid = cur.0;
                    let n = plan_nodes(
                        s,
                        cur,
                        &move |n: &Node| n.0 == mid && (n.1 - x).abs() + (n.2 - y).abs() <= 1,
                        o,
                    );
                    if let Some(n) = n {
                        if n.iter().all(|k| k.0 == mid) {
                            todo.push((n.len(), tr.clone(), seg.clone()));
                        }
                    }
                }
            }
        }
        todo.sort();
        if let Some((_, tr, seg)) = todo.into_iter().next() {
            if let Some(rs) = seg_of(&cur_id) {
                let mid = cur.0;
                let (m, _, x, y) = trainer_obj(s, &tr).unwrap();
                let _ = m;
                let nodes = plan_nodes_walled(
                    s,
                    cur,
                    &move |n: &Node| n.0 == mid && (n.1 - x).abs() + (n.2 - y).abs() <= 1,
                    o,
                )
                .unwrap_or_default();
                repel_for(s, rs, &nodes)?;
                // The walk to the trainer can outlast the Repel (Rock Tunnel: r2g / r2j / r2i):
                // re-use where it ends, then the trainer.
                if repel_waypoint(s, &cur_id, rs, &nodes, o)? {
                    continue;
                }
            }
            fight_trainer(s, &tr, &seg, o)?;
            continue;
        }
        if arrived(&cur) {
            return Ok(());
        }
        let nodes = plan_nodes_walled(s, cur, &arrived, o)
            .ok_or_else(|| PlayError::Fatal(format!("no path from {cur_id} to {dest} {at:?}")))?;
        // The first node past this map on the planned path: the warp's landing tile or the
        // connection's edge. Going to "anywhere on the next map" can take another ladder, from
        // which the plan leads back (Mt. Moon 1F <-> B1F).
        let next_node = nodes.iter().find(|n| n.0 != cur.0).copied();
        let next = next_node.map(|n| n.0);
        if let Some(rs) = seg_of(&cur_id) {
            let on_map: Vec<Node> = nodes.iter().take_while(|n| n.0 == cur.0).copied().collect();
            repel_for(s, rs, &on_map)?;
            if repel_waypoint(s, &cur_id, rs, &on_map, o)? {
                continue;
            }
        }
        let what = match next {
            Some(nx) => format!("{cur_id} -> {}", s.graph().maps[&nx].id),
            None => format!("{cur_id} {at:?}"),
        };
        go_to(
            s,
            &what,
            // Within a tile of it: a door's exit lands on the door and steps the player off it.
            &move |n: &Node| {
                next_node
                    .is_some_and(|nx| n.0 == nx.0 && (n.1 - nx.1).abs() + (n.2 - nx.2).abs() <= 1)
                    || arrived(n)
            },
            o,
        )?;
    }
    Err(PlayError::Death(format!("soft-lock: travel to {dest}")))
}

// ---------------------------------------------------------------------------------------------
// Items

/// Eat the Rare Candies the plan eats at `anchor` (`elixirs` of kind `candy` after it, while
/// `candy_early` and under `candy_early_n` in all): one each on the carry (party slot 0), as many as
/// are in the bag.
fn eat_candies(s: &mut Session, anchor: &str) -> PResult<()> {
    let p = &s.plan.params;
    if !p.candy_early {
        return Ok(());
    }
    let want = p
        .elixirs
        .iter()
        .filter(|e| e.after == anchor && e.kind == "candy")
        .count() as u16;
    if want == 0 {
        return Ok(());
    }
    let eaten_before = s
        .log
        .events
        .iter()
        .filter(|e| e.nums.get("candy_eaten").is_some())
        .count() as u32;
    let cap = p.candy_early_n.saturating_sub(eaten_before) as u16;
    let have_n = have(s, "ITEM_RARE_CANDY");
    let n = want.min(have_n).min(cap);
    if n < want {
        s.event(
            "model-vs-actual",
            format!("plan eats {want} Rare Candies after {anchor}; {have_n} in the bag, {n} eaten"),
            json!({"anchor": anchor, "want": want, "have": have_n, "cap": cap}),
        );
    }
    for _ in 0..n {
        let lv0 = Ram::new(&s.obs).party(s.hf.rec.emu())[0].level;
        super::shop::use_item_on(s, "ITEM_RARE_CANDY", 0)?;
        settle(s)?;
        let lv = Ram::new(&s.obs).party(s.hf.rec.emu())[0].level;
        s.event(
            "decision",
            format!(
                "Rare Candy on the carry (plan elixirs candy after {anchor}): Lv{lv0} -> Lv{lv}"
            ),
            json!({"candy_eaten": 1, "anchor": anchor, "level": lv}),
        );
    }
    Ok(())
}

/// Use the plan's PP Ups / vitamins found at `anchor` (`elixirs` kinds `ppup`, `calcium`) on the carry.
fn use_pickups(s: &mut Session, anchor: &str) -> PResult<()> {
    let kinds: Vec<String> = s
        .plan
        .params
        .elixirs
        .iter()
        .filter(|e| e.after == anchor)
        .map(|e| e.kind.clone())
        .collect();
    for k in kinds {
        match k.as_str() {
            "calcium" if have(s, "ITEM_CALCIUM") > 0 => {
                super::shop::use_item_on(s, "ITEM_CALCIUM", 0)?;
                settle(s)?;
                s.event(
                    "decision",
                    format!("Calcium on the carry (plan elixirs calcium after {anchor})"),
                    json!({"anchor": anchor}),
                );
            }
            "ppup" if have(s, "ITEM_PP_UP") > 0 => {
                // The sim's `apply_pp_ups`: the carry's main attack (the first `keep_moves` move
                // it knows), else slot 0.
                let mon = Ram::new(&s.obs).party(s.hf.rec.emu())[0].clone();
                let keep: Vec<u16> = s.plan.params.keep_moves.iter().map(|m| id(m)).collect();
                let slot = keep.iter().find_map(|k| mon.knows(*k)).unwrap_or(0);
                super::shop::use_item_on_move(s, "ITEM_PP_UP", 0, Some(slot))?;
                settle(s)?;
                s.event(
                    "decision",
                    format!(
                        "PP Up on the carry's {} (plan elixirs ppup after {anchor})",
                        short(names().mv(mon.moves[slot]))
                    ),
                    json!({"anchor": anchor, "slot": slot}),
                );
            }
            "candy" => {}
            other => {
                if ["calcium", "ppup"].contains(&other) {
                    s.event(
                        "model-vs-actual",
                        format!("plan elixirs {other} after {anchor}: not in the bag"),
                        json!({"anchor": anchor, "kind": other}),
                    );
                }
            }
        }
    }
    Ok(())
}

/// Top up the bag at the mart on `map`: Repels to `repels`, then Super Potions to `supers`, as far
/// as the money goes above `reserve` (Repel 350, Super Potion 700, `decompiled/src/data/items.json`).
/// The sim's potions are unlimited outside the League and it carries no Repel count: every stock
/// is a controller choice, logged as such.
fn stock_up(
    s: &mut Session,
    map: &str,
    repels: u16,
    supers: u16,
    reserve: u32,
    why: &str,
) -> PResult<()> {
    let mut money = Ram::new(&s.obs)
        .money(s.hf.rec.emu())
        .saturating_sub(reserve);
    let mut want: Vec<(&str, u16)> = vec![];
    for (it, target, price) in [
        ("ITEM_REPEL", repels, 350u32),
        ("ITEM_SUPER_POTION", supers, 700u32),
    ] {
        let h = have(s, it);
        let n = (target.saturating_sub(h) as u32).min(money / price) as u16;
        if n > 0 {
            money -= n as u32 * price;
            want.push((it, h + n));
        }
    }
    if want.is_empty() {
        return Ok(());
    }
    s.event(
        "model-vs-actual",
        format!(
            "{why}: {} (the sim's heals are unrationed and it counts no Repels)",
            want.iter()
                .map(|(i, n)| format!("{} to {n}", short(i)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        json!({"map": map}),
    );
    super::shop::mart_topup(s, map, None, &want, why)
}

/// A Center heal when the carry is under the plan's `heal_below` (the sim's between-fight heal
/// threshold, `Params::heal_below`): the bag-heal the sim prices as free (`sim.rs` pre-fight
/// heals) costs real potions here; a Center on the way costs none.
fn center_if_low(s: &mut Session, center: &str, why: &str) -> PResult<()> {
    let lead = Ram::new(&s.obs).party(s.hf.rec.emu())[0].clone();
    let frac = lead.hp as f64 / lead.max_hp.max(1) as f64;
    if frac >= s.plan.params.heal_below {
        return Ok(());
    }
    s.event(
        "model-vs-actual",
        format!(
            "Center stop ({why}): carry {}/{} under heal_below {}; the model heals from an unrationed bag",
            lead.hp, lead.max_hp, s.plan.params.heal_below
        ),
        json!({"hp": lead.hp, "max": lead.max_hp}),
    );
    center_heal(s, center)
}

/// A hidden Rare Candy the plan counts (`elixirs` candy), picked up when the detour from here to
/// `next_map` is at most `max_extra` tiles (`plan_len`: here -> facing the tile -> `next_map`,
/// minus here -> `next_map`); otherwise left, and logged as a model bug (the model counts it).
fn candy_on_way(
    s: &mut Session,
    map: &str,
    x: i16,
    y: i16,
    gone: &str,
    next_map: &str,
    max_extra: usize,
) -> PResult<()> {
    if flag(s, gone) {
        return Ok(());
    }
    let o = cut_opts();
    let mid = s.graph().id(map);
    let nmid = s.graph().id(next_map);
    let adj = move |n: &Node| n.0 == mid && (n.1 - x).abs() + (n.2 - y).abs() == 1;
    let to_next = move |n: &Node| n.0 == nmid;
    let direct = super::walk::plan_len(s, None, &to_next, &o);
    let there = here(s).and_then(|h| {
        plan_nodes(s, h, &adj, &o).map(|n| (n.len().saturating_sub(1), *n.last().unwrap()))
    });
    let extra = match (direct, there) {
        (Some(d), Some((a, end))) => {
            super::walk::plan_len(s, Some(end), &to_next, &o).map(|b| (a + b).saturating_sub(d))
        }
        _ => None,
    };
    match extra {
        Some(e) if e <= max_extra => {
            s.event(
                "decision",
                format!(
                    "hidden Rare Candy at {} ({x}, {y}): {e} extra tiles, picked up (plan elixirs candy)",
                    short(map)
                ),
                json!({"map": map, "x": x, "y": y, "extra": e}),
            );
            pick(s, map, x, y, gone, "ITEM_RARE_CANDY")
        }
        _ => {
            s.event(
                "model-vs-actual",
                format!(
                    "hidden Rare Candy at {} ({x}, {y}) left: {extra:?} extra tiles (> {max_extra}); \
                     the model counts it at no walking cost",
                    short(map)
                ),
                json!({"map": map, "x": x, "y": y, "extra": extra}),
            );
            Ok(())
        }
    }
}

/// A hidden item or item ball (`legs3::pickup`), logged against the plan's anchor.
fn pick(s: &mut Session, map: &str, x: i16, y: i16, gone: &str, item: &str) -> PResult<()> {
    super::legs3::pickup(s, map, x, y, gone, item)
}

// ---------------------------------------------------------------------------------------------
// Specials, tutors, teaches

fn special_at(s: &mut Session, after: &str, kind: &str) -> PResult<()> {
    let sps: Vec<crate::params::Special> = s
        .plan
        .params
        .specials
        .iter()
        .filter(|sp| sp.after == after && (kind.is_empty() || sp.kind == kind))
        .cloned()
        .collect();
    for sp in &sps {
        if sp.kind == "sell" {
            continue;
        }
        super::dex::special(s, sp)?;
    }
    Ok(())
}

/// The plan teaches `mv` from a TM at `anchor` (`tms`): to the carry, the forget slot by the sim.
fn teach_plan_tm(s: &mut Session, anchor: &str, mv: &str, item: &str) -> PResult<()> {
    if !s
        .plan
        .params
        .tms
        .iter()
        .any(|t| t.after == anchor && t.mv == mv)
    {
        return Ok(());
    }
    if have(s, item) == 0 {
        s.event(
            "model-vs-actual",
            format!("{item} not in the bag for the plan's teach after {anchor}"),
            json!({"item": item}),
        );
        return Ok(());
    }
    super::items::teach_tmhm(s, item, 0)
}

/// A move tutor (`decompiled/data/scripts/move_tutors.inc`): talk, YES to "teach?", YES to "only
/// once", the carry in the party menu, the forget list by the sim (`items::forget_for_move`).
fn tutor(s: &mut Session, map: &str, script: &str, mv: &str) -> PResult<()> {
    let lid = super::legs3::obj_id(s, map, script)?;
    let m = id(mv);
    if Ram::new(&s.obs).party(s.hf.rec.emu())[0].knows(m).is_some() {
        return Ok(());
    }
    talk_to(s, map, lid)?;
    let start = s.hf.fed();
    let mut forgot = false;
    loop {
        if s.hf.fed() - start > 6000 {
            return Err(PlayError::Death("soft-lock: move tutor".into()));
        }
        if super::prims::is_free(s) {
            settle(s)?;
            break;
        }
        if super::menu::yesno_up(s) {
            super::menu::answer(s, true)?;
            continue;
        }
        if super::menu::task(s, "Task_HandleChooseMonInput") {
            super::items::party_choose(s, 0)?;
            continue;
        }
        if !forgot && super::menu::task(s, "Task_InputHandler_SelectOrForgetMove") {
            super::items::forget_for_move(s, Some(m), short(mv), 0)?;
            forgot = true;
            continue;
        }
        s.hf.late()?;
        if super::prims::is_free(s) || super::menu::yesno_up(s) {
            continue;
        }
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
    let mon = Ram::new(&s.obs).party(s.hf.rec.emu())[0].clone();
    let ok = mon.knows(m).is_some();
    s.event(
        if ok { "decision" } else { "model-vs-actual" },
        format!(
            "tutor {}: {} ({})",
            short(mv),
            if ok {
                "learned by the carry"
            } else {
                "NOT learned"
            },
            mon.moves
                .iter()
                .map(|x| short(names().mv(*x)).to_string())
                .collect::<Vec<_>>()
                .join("/")
        ),
        json!({"move": mv, "learned": ok}),
    );
    Ok(())
}

/// The party slot (not the carry) that learns HM01 Cut, by the TM/HM learnsets
/// (`decompiled/src/data/pokemon/tmhm_learnsets.h`, `sim::can_learn`): first such slot.
fn cut_learner(s: &mut Session) -> Option<usize> {
    hm_learner(s, "MOVE_CUT")
}

/// The party slot (not the carry) that knows `mv` or can learn it into a free move slot, by the TM/HM
/// learnsets (`decompiled/src/data/pokemon/tmhm_learnsets.h`, `sim::can_learn`).
pub(crate) fn hm_learner(s: &mut Session, mv: &str) -> Option<usize> {
    let dex = s.ctx.dex;
    let m_id = super::battle::move_index(dex, id(mv))?;
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    if let Some(i) = party.iter().position(|m| m.knows(id(mv)).is_some()) {
        return Some(i);
    }
    party
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, m)| {
            let sp = super::battle::species_index(dex, m.species)?;
            (crate::sim::can_learn(&s.ctx, sp, m_id) && m.moves.contains(&0)).then_some(i)
        })
        .or_else(|| {
            party.iter().enumerate().skip(1).find_map(|(i, m)| {
                let sp = super::battle::species_index(dex, m.species)?;
                crate::sim::can_learn(&s.ctx, sp, m_id).then_some(i)
            })
        })
}

/// PLAN `cut_hunt_after` `none` with a Route 5 hunt anchored at `tm28-dig` (R3d plan 1952): no Route 25
/// hunt; the carrier is the first Meowth or Bellsprout of Route 5 (`sRoute5_LeafGreen`,
/// `decompiled/src/data/wild_encounters.json`), taught at `bike-redeem`.
fn cut_route5(s: &Session) -> bool {
    let p = &s.plan.params;
    p.cut_hunt_after == "none"
        && p.hunts
            .iter()
            .any(|h| h.after == "tm28-dig" && h.site == "MAP_ROUTE5")
}

/// A boxed Cut learner (a catch made with six in the party went to the PC, `GiveMonToPlayer`,
/// `decompiled/src/pokemon.c:3686-3704`): its species, from box 1 to 14.
fn cut_learner_boxed(s: &mut Session) -> Option<u16> {
    let dex = s.ctx.dex;
    let cut = super::battle::move_index(dex, id("MOVE_CUT"))?;
    (0..14u32).find_map(|b| {
        super::shop::box_species(s, b).into_iter().find(|&sp| {
            sp != 0
                && super::battle::species_index(dex, sp)
                    .is_some_and(|i| crate::sim::can_learn(&s.ctx, i, cut))
        })
    })
}

/// The route needs a Cut user from Route 9 on (the tree at (50, 18) of `CeruleanCity/map.json`
/// is the only way east: `bin/frlg-mapgraph` 60 tiles with Cut, 772 without), and the post-Giovanni
/// stages cut too. The model has no Cut carrier (MODEL BUG): when nobody in the party can learn
/// HM01, the human catches the first Bellsprout of Route 25's grass (LG land table,
/// `decompiled/src/data/wild_encounters.json`; Bellsprout learns HM01, `tmhm_learnsets.h`), leaving
/// every other species alone.
fn cut_carrier_hunt(s: &mut Session) -> PResult<()> {
    cut_carrier_hunt_at(s, "MAP_ROUTE25", "route25-trainers")
}

fn cut_carrier_hunt_at(s: &mut Session, site: &str, after: &str) -> PResult<()> {
    if cut_learner(s).is_some() {
        return Ok(());
    }
    let skip: Vec<String> = [
        "SPECIES_ABRA",
        "SPECIES_CATERPIE",
        "SPECIES_KAKUNA",
        "SPECIES_METAPOD",
        "SPECIES_PIDGEY",
        "SPECIES_WEEDLE",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    s.event(
        "model-vs-actual",
        "no party mon learns HM01 Cut (needed for Route 9 and later): hunting a Bellsprout on \
         Route 25 (the model has no Cut carrier)",
        json!({}),
    );
    let h = crate::params::Hunt {
        after: after.into(),
        site: site.into(),
        area: "land".into(),
        steps: 400,
        skip,
        on_route: true,
        stop_after: 1,
        ..Default::default()
    };
    let prev = std::mem::replace(&mut s.seg, "cut-carrier-hunt".into());
    let rep = super::dex::hunt(s, &h)?;
    s.seg = prev;
    if cut_learner(s).is_none() {
        return Err(PlayError::Death(format!(
            "no Cut carrier after the {site} hunt ({} encounters)",
            rep.encounters
        )));
    }
    Ok(())
}

fn teach_cut(s: &mut Session) -> PResult<()> {
    if super::items::slot_with_move(s, "MOVE_CUT").is_some() {
        return Ok(());
    }
    if cut_learner(s).is_none() && cut_route5(s) {
        if let Some(sp) = cut_learner_boxed(s) {
            // PLAN 1952 (`cut_box_honest`): the carrier is in the box; withdraw it at the Cerulean
            // Center, depositing the lowest-level helper first when the party is full.
            let party = Ram::new(&s.obs).party(s.hf.rec.emu());
            let mut ops = vec![];
            if party.len() >= 6 {
                let slot = party
                    .iter()
                    .enumerate()
                    .skip(1)
                    .min_by_key(|(_, m)| m.level)
                    .map(|(i, _)| i)
                    .unwrap_or(5);
                ops.push(super::shop::PcOp::Deposit(slot));
            }
            ops.push(super::shop::PcOp::Withdraw(sp));
            s.event(
                "decision",
                format!(
                    "Cut carrier {} is in the box: Cerulean Center PC",
                    short(names().species(sp))
                ),
                json!({"species": sp, "plan_key": "cut_box_honest"}),
            );
            super::shop::pc(s, "MAP_CERULEAN_CITY_POKEMON_CENTER_1F", &ops)?;
        } else {
            // PLAN 1952 `cut_rehunt`: the Route 5 hunt found nothing (or the balls ran out): balls to 5 at
            // the Cerulean mart, then Route 24's grass (Oddish / Bellsprout, `sRoute24_LeafGreen`).
            s.event(
                "model-vs-actual",
                "no Cut carrier after Route 5: balls to 5 at Cerulean, Route 24 hunt (plan cut_rehunt)",
                json!({"plan_key": "cut_rehunt"}),
            );
            super::shop::mart_topup(
                s,
                "MAP_CERULEAN_CITY_MART",
                None,
                &[("ITEM_POKE_BALL", 5)],
                "cut_rehunt",
            )?;
            cut_carrier_hunt_at(s, "MAP_ROUTE24", "cut-rehunt")?;
        }
    }
    let Some(slot) = cut_learner(s) else {
        return Err(PlayError::Fatal(
            "no party mon can learn Cut (tmhm_learnsets.h)".into(),
        ));
    };
    let sp = Ram::new(&s.obs).party(s.hf.rec.emu())[slot].species;
    s.event(
        "model-vs-actual",
        format!(
            "HM01 Cut taught to party slot {slot} ({}): the model has no Cut carrier, the route \
             needs Cut for Route 9 (`CeruleanCity/map.json` cut tree)",
            short(names().species(sp))
        ),
        json!({"slot": slot}),
    );
    super::items::teach_tmhm(s, "ITEM_HM01", slot)
}

// ---------------------------------------------------------------------------------------------
// Stages

/// Mt. Moon segments by map, for `repel_on`.
fn moon_seg(map: &str) -> Option<&'static str> {
    match map {
        "MAP_MT_MOON_1F" => Some("mt-moon-1f"),
        "MAP_MT_MOON_B1F" => Some("mt-moon-b1f"),
        "MAP_MT_MOON_B2F" => Some("mt-moon-b2f"),
        _ => None,
    }
}

/// `mt-moon-1f` + `mt-moon-extra` (`extra_fights`: Bug Catcher Robby, `MtMoon_1F/map.json`
/// (36, 30)): the Magikarp first if the salesman is still on hand (`specials` buy after route4:
/// he stands in this Center, `Route4_PokemonCenter_1F/scripts.inc:1,23-53`), the Mt. Moon Rare Candy
/// ball (42, 35) (eaten at Cerulean, `elixirs` candy after cerulean-heal), down to B1F.
fn mt_moon_1f(s: &mut Session) -> PResult<()> {
    s.seg = "mt-moon-1f".into();
    let here_map = here(s).map(|n| s.graph().maps[&n.0].id.clone());
    if here_map.as_deref() == Some("MAP_ROUTE4_POKEMON_CENTER_1F") {
        let sps: Vec<crate::params::Special> = s
            .plan
            .params
            .specials
            .iter()
            .filter(|sp| sp.species.iter().any(|x| x == "SPECIES_MAGIKARP"))
            .cloned()
            .collect();
        for sp in &sps {
            let money = Ram::new(&s.obs).money(s.hf.rec.emu());
            if money < sp.cost {
                s.event(
                    "model-vs-actual",
                    format!("Magikarp ({}): ${money} < ${}", sp.after, sp.cost),
                    json!({"money": money}),
                );
                continue;
            }
            s.event(
                "decision",
                format!(
                    "Magikarp salesman (plan specials buy after {}): he is in this Center, \
                     bought before Mt. Moon",
                    sp.after
                ),
                json!({"after": sp.after}),
            );
            super::dex::special(s, sp)?;
        }
    }
    let mut extra: Vec<(String, String)> = vec![];
    for ef in &s.plan.params.extra_fights.clone() {
        for sc in &ef.scripts {
            // The trainer of the script (`MtMoon_1F_EventScript_Robby` -> TRAINER_BUG_CATCHER_ROBBY).
            let g = s.graph();
            let f = g
                .maps
                .values()
                .flat_map(|m| m.objects.iter())
                .find(|o| &o.script == sc)
                .map(|o| o.beaten);
            if let Some(f) = f {
                let base = names().get("TRAINER_FLAGS_START").unwrap_or(0);
                if let Some(t) = super::ids::TRAINERS.get((f - base) as usize) {
                    extra.push((t.to_string(), ef.name.clone()));
                }
            }
        }
    }
    travel(s, "MAP_MT_MOON_1F", &Opts::default(), &moon_seg, &[])?;
    travel(s, "MAP_MT_MOON_1F", &Opts::default(), &moon_seg, &extra)?;
    let has_candy = s
        .plan
        .params
        .elixirs
        .iter()
        .any(|e| e.after == "cerulean-heal" && e.kind == "candy");
    if has_candy {
        pick(
            s,
            "MAP_MT_MOON_1F",
            42,
            35,
            "FLAG_HIDE_MT_MOON_1F_RARE_CANDY",
            "ITEM_RARE_CANDY",
        )?;
    }
    travel(s, "MAP_MT_MOON_B1F", &Opts::default(), &moon_seg, &[])
}

/// `mt-moon-b1f`, `mt-moon-b2f`, `mt-moon-miguel` (the audited forced pair: the Rocket
/// `MtMoon_B2F_EventScript_Grunt1` at (12, 20) and Super Nerd Miguel, whose coord trigger (14, 11)
/// stands between the player and the fossils, `MtMoon_B2F/scripts.inc` `MiguelTrigger`), then the
/// Helix Fossil (14, 7) (the plan's Omanyte gift after the Blaine/Sevii segment).
fn miguel(s: &mut Session) -> PResult<()> {
    s.seg = "mt-moon-b2f".into();
    let fights = vec![(
        "TRAINER_TEAM_ROCKET_GRUNT".to_string(),
        "mt-moon-miguel".to_string(),
    )];
    travel(s, "MAP_MT_MOON_B2F", &Opts::default(), &moon_seg, &[])?;
    // Passing encounters on B2F stay under `mt-moon-b2f` (the plan's `pass_off`); only the fights
    // are `mt-moon-miguel` (`fight_trainer` sets the segment).
    travel(s, "MAP_MT_MOON_B2F", &Opts::default(), &moon_seg, &fights)?;
    if !beaten(s, "TRAINER_SUPER_NERD_MIGUEL") {
        // Walk onto the trigger tile; the scene and the battle run in settle.
        travel_to(
            s,
            "MAP_MT_MOON_B2F",
            Some((14, 12)),
            &Opts::default(),
            &moon_seg,
            &[],
        )?;
        s.seg = "mt-moon-miguel".into();
        go(s, "MAP_MT_MOON_B2F", 14, 11, &Opts::default())?;
        settle(s)?;
    }
    if !beaten(s, "TRAINER_SUPER_NERD_MIGUEL") {
        return Err(PlayError::Death("Miguel not beaten".into()));
    }
    if !flag(s, "FLAG_GOT_FOSSIL_FROM_MT_MOON") {
        go_face(s, "MAP_MT_MOON_B2F", 14, 7, &Opts::default())?;
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    let helix = have(s, "ITEM_HELIX_FOSSIL") > 0;
    s.event(
        "decision",
        format!(
            "Mt. Moon fossil: {} (the plan's Omanyte gift after blaine-badge-and-sevii-scene)",
            if helix { "Helix" } else { "none" }
        ),
        json!({"helix": helix}),
    );
    Ok(())
}

/// `route4` + `tm-mega_kick@route4`: out of Mt. Moon to Route 4, the Mega Kick tutor
/// (`Route4/map.json` (50, 3), `Route4_EventScript_MegaKickTutor`, `move_tutors.inc:131-144`).
fn mega_kick(s: &mut Session) -> PResult<()> {
    s.seg = "route4".into();
    travel(s, "MAP_ROUTE4", &Opts::default(), &moon_seg, &[])?;
    if s.plan
        .params
        .tms
        .iter()
        .any(|t| t.after == "route4" && t.mv == "MOVE_MEGA_KICK")
    {
        s.seg = "tm-mega_kick@route4".into();
        tutor(
            s,
            "MAP_ROUTE4",
            "Route4_EventScript_MegaKickTutor",
            "MOVE_MEGA_KICK",
        )?;
    }
    Ok(())
}

/// `cerulean-heal`, the two `elixir-candy@cerulean-heal` (the Mt. Moon ball; the hidden one at
/// Cerulean (18, 7), `CeruleanCity/map.json`, if the walk passes it), `persim-and-potions` (the
/// Cerulean mart, `CeruleanCity_Mart/scripts.inc:25-34`: the plan's `awakenings`; Repels for the
/// `repel_on` stretches to come and Super Potions are controller choices the model does not price).
fn cerulean(s: &mut Session) -> PResult<()> {
    s.seg = "cerulean-heal".into();
    center_heal(s, "MAP_CERULEAN_CITY_POKEMON_CENTER_1F")?;
    // The second `elixir-candy@cerulean-heal`: the hidden candy (18, 7) of `CeruleanCity/map.json`
    // (`FLAG_HIDDEN_ITEM_CERULEAN_CITY_RARE_CANDY`), measured against the walk to the gym.
    candy_on_way(
        s,
        "MAP_CERULEAN_CITY",
        18,
        7,
        "FLAG_HIDDEN_ITEM_CERULEAN_CITY_RARE_CANDY",
        "MAP_CERULEAN_CITY_GYM",
        30,
    )?;
    eat_candies(s, "cerulean-heal")?;
    s.seg = "persim-and-potions".into();
    let p = &s.plan.params;
    let awak = p.awakenings as u16;
    let persim = p.persim_held;
    if persim {
        let held = Ram::new(&s.obs).party(s.hf.rec.emu())[0].held;
        if held != id("ITEM_PERSIM_BERRY") {
            let in_bag = have(s, "ITEM_PERSIM_BERRY");
            s.event(
                "model-vs-actual",
                format!(
                    "plan persim_held: the carry holds {} (the Persim is the Route 4 hidden berry \
                     (5, 4), `Route4/map.json`, west of Mt. Moon; giving it from the Berry Pouch is \
                     not scripted)",
                    short(names().item(held))
                ),
                json!({"held": held, "persim_in_bag": in_bag}),
            );
        }
    }
    let money = Ram::new(&s.obs).money(s.hf.rec.emu());
    let mut want: Vec<(&str, u16)> = vec![];
    if awak > 0 {
        want.push(("ITEM_AWAKENING", awak));
    }
    // PLAN `marts` after `persim-and-potions` (`mid_bag`: 6 Potions in 1910 / 1930 / 1952 / 1964) right after
    // the plan's Awakenings, before the controller's own extras: v5b-s41 had $1392, spent it on 3
    // Awakenings and 2 Antidotes, could not afford a Super Potion and met the Cerulean rival at 12/58 with
    // no heal (12 blackouts, fatal). Potion 300 (`decompiled/src/data/items.json`), a Cerulean mart row
    // (`CeruleanCity_Mart/scripts.inc:27`).
    let plan_potions = s
        .plan
        .params
        .marts
        .iter()
        .filter(|m| m.after == "persim-and-potions" && m.topup)
        .map(|m| m.potions as u16)
        .max()
        .unwrap_or(0);
    if plan_potions > 0 {
        want.push(("ITEM_POTION", plan_potions));
    }
    // The Route 25 Cut-carrier hunt (`cut_carrier_hunt`) needs balls, and the plan buys none after the
    // forest catches spend Oak's five (2026-11-27 r2d-s31: 12 of 12 hunts "skipped, no ball in the bag").
    // Poke Balls are the mart's first row (`CeruleanCity_Mart/scripts.inc:24-25`); Bellsprout's catch
    // rate is 255. A controller choice the model does not price (its Cut carrier is a MODEL BUG).
    if cut_learner(s).is_none() && !cut_route5(s) {
        want.push(("ITEM_POKE_BALL", 3));
    }
    // Two Antidotes for the walker's poison guard (`early::field_guard_due`) on Routes 24 / 25 and
    // Route 9: r2e-s31 entered Route 25 poisoned at 6/65 with no cure. Antidote is a Cerulean mart row
    // (`CeruleanCity_Mart/scripts.inc:28`); a controller choice, not priced.
    want.push(("ITEM_ANTIDOTE", 2));
    // Repels for Route 9 / Rock Tunnel are bought at the stop before Route 9 (`rock_tunnel`), with
    // the bridge and Route 25 money; Nugget Bridge's path has no grass.
    // Heals: Super Potions up to 4, while the money lasts (the sim's heals are unrationed).
    want.push(("ITEM_SUPER_POTION", 4));
    s.event(
        "model-vs-actual",
        format!(
            "Cerulean mart (${money}): the plan buys {awak} Awakenings here; Repels (Route 9 / Rock \
             Tunnel repel_on) and Super Potions are the controller's (the sim does not ration them)"
        ),
        json!({"money": money}),
    );
    super::shop::mart_topup(
        s,
        "MAP_CERULEAN_CITY_MART",
        None,
        &want,
        "persim-and-potions",
    )
}

/// `misty-walk`, `cerulean-gym-trainer` (fight list empty: Luis and Diana are walked around),
/// `misty` at (8, 6), `misty-badge`, then `tm-water_pulse@misty-badge` (TM03, given by Misty,
/// `CeruleanCity_Gym/scripts.inc`).
fn misty(s: &mut Session) -> PResult<()> {
    s.seg = "misty-walk".into();
    go_map(s, "MAP_CERULEAN_CITY_GYM", &Opts::default())?;
    let fights = plan_fights(s, "cerulean-gym-trainer", &["TRAINER_PICNICKER_DIANA"]);
    for t in &fights {
        fight_trainer(s, t, "cerulean-gym-trainer", &Opts::default())?;
    }
    s.seg = "misty".into();
    go_face(s, "MAP_CERULEAN_CITY_GYM", 8, 6, &Opts::default())?;
    s.hf.late()?;
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_BADGE02_GET") {
        return Err(PlayError::Death("no Cascade Badge after Misty".into()));
    }
    split(s, "Misty", "misty-badge")?;
    s.seg = "tm-water_pulse@misty-badge".into();
    teach_plan_tm(s, "misty-badge", "MOVE_WATER_PULSE", "ITEM_TM03")
}

/// `rival-cerulean`: the rival's coord triggers (22..24, 6) north of Cerulean
/// (`CeruleanCity/map.json`, `VAR_MAP_SCENE_CERULEAN_CITY_RIVAL` 0).
fn rival_cerulean(s: &mut Session) -> PResult<()> {
    s.seg = "rival-cerulean".into();
    go_map(s, "MAP_ROUTE24", &Opts::default())?;
    settle(s)?;
    let v = var(s, "VAR_MAP_SCENE_CERULEAN_CITY_RIVAL");
    if v == 0 {
        return Err(PlayError::Fatal(
            "on Route 24 without the Cerulean rival".into(),
        ));
    }
    Ok(())
}

/// Route segments by map for `repel_on` between Cerulean and Bill.
fn bill_seg(map: &str) -> Option<&'static str> {
    match map {
        "MAP_ROUTE24" => Some("nugget-bridge"),
        "MAP_ROUTE25" => Some("bill"),
        _ => None,
    }
}

/// `nugget-bridge` + `nugget-bridge-trainers` (the five bridge trainers and the Rocket,
/// `Route24/map.json`; the Rocket's Nugget is sold later, `specials` sell after celadon-tea).
fn nugget_bridge(s: &mut Session) -> PResult<()> {
    s.seg = "nugget-bridge-trainers".into();
    let fights: Vec<(String, String)> = plan_fights(
        s,
        "nugget-bridge-trainers",
        &[
            "TRAINER_CAMPER_ETHAN",
            "TRAINER_LASS_RELI",
            "TRAINER_YOUNGSTER_TIMMY",
            "TRAINER_LASS_ALI",
            "TRAINER_BUG_CATCHER_CALE",
        ],
    )
    .into_iter()
    .map(|t| (t, "nugget-bridge-trainers".to_string()))
    .collect();
    travel(s, "MAP_ROUTE24", &Opts::default(), &bill_seg, &fights)?;
    // The Rocket's scene: coord triggers (10, 15) / (11, 15) (`Route24/scripts.inc:5-51`).
    if var(s, "VAR_MAP_SCENE_ROUTE24") == 0 {
        go(s, "MAP_ROUTE24", 11, 14, &Opts::default())?;
        settle(s)?;
    }
    let nugget = have(s, "ITEM_NUGGET");
    s.event(
        "decision",
        format!("Nugget Bridge done: {nugget} Nugget in the bag"),
        json!({"nugget": nugget}),
    );
    Ok(())
}

/// `bill` + `route25-trainers` (`fight_lists`), `bill-cottage`: Bill's Clefairy (YES), the
/// teleporter PC (sign (4, 5), facing north), Bill again for the S.S. Ticket
/// (`Route25_SeaCottage/scripts.inc`).
fn bill(s: &mut Session) -> PResult<()> {
    s.seg = "route25-trainers".into();
    let fights: Vec<(String, String)> = plan_fights(s, "route25-trainers", &[])
        .into_iter()
        .map(|t| (t, "route25-trainers".to_string()))
        .collect();
    travel(s, "MAP_ROUTE25", &Opts::default(), &bill_seg, &fights)?;
    if !cut_route5(s) {
        cut_carrier_hunt(s)?;
    }
    travel(
        s,
        "MAP_ROUTE25_SEA_COTTAGE",
        &Opts::default(),
        &bill_seg,
        &fights,
    )?;
    s.seg = "bill-cottage".into();
    if !flag(s, "FLAG_HELPED_BILL_IN_SEA_COTTAGE") {
        talk_to(s, "MAP_ROUTE25_SEA_COTTAGE", 2)?;
        settle(s)?;
        go_face(s, "MAP_ROUTE25_SEA_COTTAGE", 4, 5, &Opts::default())?;
        super::walk::face(s, keys::UP)?;
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    if have(s, "ITEM_SS_TICKET") == 0 {
        talk_to(s, "MAP_ROUTE25_SEA_COTTAGE", 1)?;
        settle(s)?;
    }
    if have(s, "ITEM_SS_TICKET") == 0 {
        return Err(PlayError::Death("no S.S. Ticket after Bill".into()));
    }
    split(s, "Bill", "bill-cottage")
}

/// `teleport-cerulean` (no Abra yet: the plan's Abra is bought after Giovanni, `specials` buy after
/// scope-and-dig, so this is the walk), `dig-grunt` (`dig_after_bill`: the Rocket behind the robbed
/// house, `CeruleanCity/map.json` (33, 6), coord triggers (33, 5) / (33, 7)) and `tm28-dig`.
fn dig_grunt(s: &mut Session) -> PResult<()> {
    s.seg = "teleport-cerulean".into();
    let abra = super::dex::party_slot(s, id("SPECIES_ABRA")).is_some();
    if !abra && s.plan.params.teleport_abra {
        s.event(
            "model-vs-actual",
            "teleport-cerulean: no Abra in the party (the plan buys it after scope-and-dig); walking back",
            json!({}),
        );
    }
    go_map(s, "MAP_CERULEAN_CITY", &Opts::default())?;
    // The plan's `center-heal@nugget-bridge-trainers` (`extra_heals`): the only Center is in
    // Cerulean, so the human heals on the way back from Bill instead of walking back mid-route.
    if s.plan
        .params
        .extra_heals
        .iter()
        .any(|h| h == "nugget-bridge-trainers")
    {
        s.seg = "center-heal@nugget-bridge-trainers".into();
        s.event(
            "model-vs-actual",
            "center-heal@nugget-bridge-trainers taken at Cerulean after Bill (the Center is behind \
             the bridge; the model heals right after the bridge trainers)",
            json!({}),
        );
        center_heal(s, "MAP_CERULEAN_CITY_POKEMON_CENTER_1F")?;
        s.seg = "teleport-cerulean".into();
    }
    if s.plan.params.variant("dig_after_bill") {
        s.seg = "dig-grunt".into();
        if var(s, "VAR_MAP_SCENE_CERULEAN_CITY_ROCKET") == 0 {
            let mid = s.graph().id("MAP_CERULEAN_CITY");
            go_to(
                s,
                "the Cerulean Rocket's trigger",
                &move |n: &Node| n.0 == mid && n.1 == 33 && (n.2 == 5 || n.2 == 7),
                &Opts::default(),
            )?;
            settle(s)?;
        }
        let dig = have(s, "ITEM_TM28");
        s.event(
            "decision",
            format!("Cerulean Rocket beaten: TM28 x{dig} (picked up, not taught: no_dig)"),
            json!({"tm28": dig}),
        );
    }
    // PLAN 1952: the plan's anchors after `tm28-dig`, the Route 5 Cut-carrier hunt among them (the grass
    // the path to the Underground Path crosses), unless a Cut learner is already in the party.
    if cut_route5(s) {
        if cut_learner(s).is_some() {
            s.event(
                "decision",
                "Route 5 Cut hunt skipped: a party mon already learns HM01",
                json!({}),
            );
        } else {
            s.seg = "tm28-dig".into();
            super::dex::dex_hook(s, "tm28-dig")?;
        }
        // HM02 (Route 16 house, `leg_hm02`) needs a Fly learner; the model has none (MODEL BUG, like Cut
        // before R3d). Earlier runs had a Route 1 Pidgey caught against `pass_off`; with that fixed
        // (v5c-s41: fatal "no Pidgey to teach Fly"), the human takes the first Pidgey of the same Route 5
        // grass, 40 % of `sRoute5_LeafGreen` (`decompiled/src/data/wild_encounters.json`; Pidgey learns
        // HM02, `tmhm_learnsets.h`).
        if hm_learner(s, "MOVE_FLY").is_none() {
            s.event(
                "model-vs-actual",
                "no party mon learns HM02 Fly: a Route 5 Pidgey (the model has no Fly carrier)",
                json!({}),
            );
            let h = crate::params::Hunt {
                after: "tm28-dig".into(),
                site: "MAP_ROUTE5".into(),
                area: "land".into(),
                steps: 150,
                skip: vec!["SPECIES_MEOWTH".into(), "SPECIES_BELLSPROUT".into()],
                on_route: true,
                stop_after: 1,
                ..Default::default()
            };
            let prev = std::mem::replace(&mut s.seg, "fly-carrier-hunt".into());
            let rep = super::dex::hunt(s, &h)?;
            s.seg = prev;
            if hm_learner(s, "MOVE_FLY").is_none() {
                s.event(
                    "warn",
                    format!(
                        "no Fly learner after the Route 5 hunt ({} encounters, {})",
                        rep.encounters, rep.why
                    ),
                    json!({"encounters": rep.encounters}),
                );
            }
        }
    }
    Ok(())
}

fn r56_seg(map: &str) -> Option<&'static str> {
    match map {
        "MAP_ROUTE5" => Some("route5-6"),
        "MAP_ROUTE6" => Some("route6-encounters"),
        _ => None,
    }
}

/// `route5-6` (+ the Nidoran trade at the Underground Path's north entrance, `specials` trade after
/// route5-6), `route6-encounters`, `route6-trainers` (Camper Jeff, `fight_lists`), `fan-club` +
/// `bike-voucher` (the Chairman, `VermilionCity_PokemonFanClub/map.json` (5, 4)).
fn bike_voucher(s: &mut Session) -> PResult<()> {
    s.seg = "route5-6".into();
    travel(
        s,
        "MAP_UNDERGROUND_PATH_NORTH_ENTRANCE",
        &Opts::default(),
        &r56_seg,
        &[],
    )?;
    special_at(s, "route5-6", "trade")?;
    s.seg = "route6-trainers".into();
    let fights: Vec<(String, String)> = plan_fights(s, "route6-trainers", &[])
        .into_iter()
        .map(|t| (t, "route6-trainers".to_string()))
        .collect();
    travel(s, "MAP_ROUTE6", &Opts::default(), &r56_seg, &[])?;
    // One of the three `elixir-candy@hideout-grunts`: Route 6's hidden candy (19, 5)
    // (`Route6/map.json`, `FLAG_HIDDEN_ITEM_ROUTE6_RARE_CANDY`).
    candy_on_way(
        s,
        "MAP_ROUTE6",
        19,
        5,
        "FLAG_HIDDEN_ITEM_ROUTE6_RARE_CANDY",
        "MAP_VERMILION_CITY",
        // 34 extra tiles in r2b-full2 (~5 s running plus the found-item text) for a level.
        40,
    )?;
    travel(s, "MAP_VERMILION_CITY", &Opts::default(), &r56_seg, &fights)?;
    s.seg = "bike-voucher".into();
    if have(s, "ITEM_BIKE_VOUCHER") == 0 {
        let lid = super::legs3::obj_id(
            s,
            "MAP_VERMILION_CITY_POKEMON_FAN_CLUB",
            "VermilionCity_PokemonFanClub_EventScript_Chairman",
        )?;
        talk_to(s, "MAP_VERMILION_CITY_POKEMON_FAN_CLUB", lid)?;
        settle(s)?;
    }
    if have(s, "ITEM_BIKE_VOUCHER") == 0 {
        return Err(PlayError::Death("no Bike Voucher".into()));
    }
    Ok(())
}

/// `ss-anne` (+ the Farfetch'd trade, `specials` trade after ss-anne, `VermilionCity_House2`),
/// `rival-ss-anne` (coord triggers (30..32, 6) of `SSAnne_2F_Corridor`), `captain-hm01` (the
/// Captain (5, 4) of `SSAnne_CaptainsOffice`).
fn ss_anne(s: &mut Session) -> PResult<()> {
    s.seg = "ss-anne".into();
    special_at(s, "ss-anne", "trade")?;
    // The rival's Kadabra / Ivysaur killed two of three branches with an empty bag (r2b-dev8/9).
    // Vermilion's Center and mart are on the way to the dock.
    center_if_low(
        s,
        "MAP_VERMILION_CITY_POKEMON_CENTER_1F",
        "before the S.S. Anne rival",
    )?;
    stock_up(
        s,
        "MAP_VERMILION_CITY_MART",
        0,
        5,
        500,
        "Super Potions for the S.S. Anne rival",
    )?;
    s.seg = "rival-ss-anne".into();
    go_map(s, "MAP_SSANNE_2F_CORRIDOR", &Opts::default())?;
    if var(s, "VAR_MAP_SCENE_S_S_ANNE_2F_CORRIDOR") == 0 {
        let mid = s.graph().id("MAP_SSANNE_2F_CORRIDOR");
        go_to(
            s,
            "the S.S. Anne rival's trigger",
            &move |n: &Node| n.0 == mid && n.2 == 6 && (30..=32).contains(&n.1),
            &Opts::default(),
        )?;
        settle(s)?;
    }
    s.seg = "captain-hm01".into();
    if have(s, "ITEM_HM01") == 0 {
        talk_to(s, "MAP_SSANNE_CAPTAINS_OFFICE", 1)?;
        settle(s)?;
    }
    if have(s, "ITEM_HM01") == 0 {
        return Err(PlayError::Death("no HM01 from the Captain".into()));
    }
    Ok(())
}

/// `teleport-cerulean-2` (walked: no Abra), `bike-shop` + `bike-redeem` (the clerk (9, 3),
/// `CeruleanCity_BikeShop/scripts.inc`), the Bicycle registered to SELECT.
fn bike(s: &mut Session) -> PResult<()> {
    s.seg = "teleport-cerulean-2".into();
    go_map(s, "MAP_VERMILION_CITY", &Opts::default())?;
    travel(
        s,
        "MAP_CERULEAN_CITY_BIKE_SHOP",
        &Opts::default(),
        &r56_seg,
        &[],
    )?;
    s.seg = "bike-redeem".into();
    if have(s, "ITEM_BICYCLE") == 0 {
        // The clerk stands behind the counter (`CeruleanCity_BikeShop/map.json` (9, 3)).
        super::shop::talk_at(s, "MAP_CERULEAN_CITY_BIKE_SHOP", 9, 3)?;
        settle(s)?;
    }
    if have(s, "ITEM_BICYCLE") == 0 {
        return Err(PlayError::Death("no Bicycle".into()));
    }
    split(s, "Bike", "bike-redeem")?;
    super::items::register_bike(s)?;
    Ok(())
}

fn rt_seg(map: &str) -> Option<&'static str> {
    match map {
        "MAP_ROUTE9" => Some("route9"),
        "MAP_ROUTE10" => Some("route10-north"),
        "MAP_ROCK_TUNNEL_1F" | "MAP_ROCK_TUNNEL_B1F" => Some("rock-tunnel"),
        _ => None,
    }
}

/// HM01 to a Cut learner, `route9`, `route10-north`, `rock-tunnel`, `rock-tunnel-trainers`
/// (`fight_lists`, ten trainers on Route 9 and in the tunnel) to Lavender. Flash: the model assumes
/// HM05; the tunnel is walked dark (the walker reads the map, not the screen).
fn rock_tunnel(s: &mut Session) -> PResult<()> {
    s.seg = "route9".into();
    teach_cut(s)?;
    // Not in the model: ten trainers between Cerulean and Lavender with no Center, and the sim's
    // potions are unlimited outside the League ("no heal item in the bag" deaths on every branch
    // of the first try: Martha, Lenny, Jeremy, Allen). The human heals at the Cerulean Center next
    // to the bike shop and stocks Super Potions at the mart (`CeruleanCity_Mart/scripts.inc:26`),
    // keeping $1500 for the Celadon X items (the Nugget sale pays the rest).
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let lead = &party[0];
    if lead.hp < lead.max_hp {
        s.event(
            "model-vs-actual",
            format!(
                "Center stop before Route 9 (lead {}/{}): the model has no heal between \
                 cerulean-heal and lavender-heal",
                lead.hp, lead.max_hp
            ),
            json!({"hp": lead.hp, "max": lead.max_hp}),
        );
        center_heal(s, "MAP_CERULEAN_CITY_POKEMON_CENTER_1F")?;
    }
    // Repels for `repel_on` route9 / rock-tunnel (every step counts, `wild_encounter.c:576-598`: the
    // played Route 9 - tunnel stretch is ~470 bike tiles, 5 Repels), then Super Potions; $500 kept (the Nugget pays for the X items).
    stock_up(
        s,
        "MAP_CERULEAN_CITY_MART",
        5,
        10,
        500,
        "Repels and Super Potions for Route 9 / Rock Tunnel",
    )?;
    s.event(
        "model-vs-actual",
        "Rock Tunnel walked without Flash: the sim assumes HM05 in the bag (a guess, route_wr.rs); \
         the game only darkens the screen (overworld.c:955-963), HM05 needs 10 species (Route 2 aide)",
        json!({"segment": "rock-tunnel"}),
    );
    let fights: Vec<(String, String)> = plan_fights(s, "rock-tunnel-trainers", &[])
        .into_iter()
        .map(|t| (t, "rock-tunnel-trainers".to_string()))
        .collect();
    travel(s, "MAP_ROUTE9", &cut_opts(), &rt_seg, &fights)?;
    // One of the three `elixir-candy@hideout-grunts`: Route 9's hidden candy (63, 2)
    // (`Route9/map.json`, `FLAG_HIDDEN_ITEM_ROUTE9_RARE_CANDY`), near the Route 10 end.
    candy_on_way(
        s,
        "MAP_ROUTE9",
        63,
        2,
        "FLAG_HIDDEN_ITEM_ROUTE9_RARE_CANDY",
        "MAP_ROUTE10",
        30,
    )?;
    travel(s, "MAP_LAVENDER_TOWN", &cut_opts(), &rt_seg, &fights)?;
    split(s, "Rock Tunnel", "rock-tunnel-trainers")
}

/// `lavender-heal`.
fn lavender(s: &mut Session) -> PResult<()> {
    s.seg = "lavender-heal".into();
    center_heal(s, "MAP_LAVENDER_TOWN_POKEMON_CENTER_1F")
}

fn r87_seg(map: &str) -> Option<&'static str> {
    match map {
        "MAP_ROUTE8" => Some("route8-7"),
        _ => None,
    }
}

/// `route8-7` + `route8-trainers` (`fight_lists`: Gamer Rich), the Underground Path to Route 7,
/// Celadon; then `celadon-tea`: the hidden PP Up (55, 20) (`CeladonCity/map.json`, `elixirs` ppup
/// after celadon-tea), `xbuy@celadon-tea` (5F X items clerk (1, 7)), the Nugget sale (`specials` sell
/// after celadon-tea), Eevee (`specials` gift: the Condominiums roof room ball), the Tea (the old
/// woman (2, 9) of `CeladonCity_Condominiums_1F`, `scripts.inc:4-16`).
fn celadon(s: &mut Session) -> PResult<()> {
    s.seg = "route8-trainers".into();
    let fights: Vec<(String, String)> = plan_fights(s, "route8-trainers", &[])
        .into_iter()
        .map(|t| (t, "route8-trainers".to_string()))
        .collect();
    travel(s, "MAP_CELADON_CITY", &cut_opts(), &r87_seg, &fights)?;
    s.seg = "celadon-tea".into();
    if s.plan
        .params
        .elixirs
        .iter()
        .any(|e| e.after == "celadon-tea" && e.kind == "ppup")
    {
        pick(
            s,
            "MAP_CELADON_CITY",
            55,
            20,
            "FLAG_HIDDEN_ITEM_CELADON_CITY_PP_UP",
            "ITEM_PP_UP",
        )?;
        use_pickups(s, "celadon-tea")?;
    }
    Ok(())
}

/// The Celadon Dept. Store: the Nugget sold (`specials` sell after celadon-tea; half of 10000,
/// `item_menu.c:1933`) at the 2F clerk, then the plan's X items at 5F.
fn xbuy(s: &mut Session) -> PResult<()> {
    s.seg = "sell-@celadon-tea".into();
    let sell = s
        .plan
        .params
        .specials
        .iter()
        .any(|sp| sp.after == "celadon-tea" && sp.kind == "sell");
    if sell && have(s, "ITEM_NUGGET") > 0 {
        let got = super::shop::sell(
            s,
            "MAP_CELADON_CITY_DEPARTMENT_STORE_2F",
            Some((1, 8)),
            &["ITEM_NUGGET"],
        )?;
        s.event(
            "decision",
            format!("sold the Nugget for ${got} (plan specials sell after celadon-tea)"),
            json!({"money": got}),
        );
    }
    s.seg = "xbuy@celadon-tea".into();
    let xb: Vec<crate::params::XBuy> = s
        .plan
        .params
        .xbuys
        .iter()
        .filter(|x| x.after == "celadon-tea")
        .cloned()
        .collect();
    let mut x_items: Vec<(&str, u16)> = vec![];
    for x in &xb {
        for (it, n) in [
            ("ITEM_X_ATTACK", x.attack),
            ("ITEM_X_DEFEND", x.defend),
            ("ITEM_X_SPEED", x.speed),
            ("ITEM_X_SPECIAL", x.special),
            ("ITEM_DIRE_HIT", x.dire),
            ("ITEM_X_ACCURACY", x.accuracy),
            ("ITEM_GUARD_SPEC", x.guard),
        ] {
            if n > 0 {
                x_items.push((it, n as u16));
            }
        }
    }
    if !x_items.is_empty() {
        let _ = super::shop::buy(
            s,
            "MAP_CELADON_CITY_DEPARTMENT_STORE_5F",
            Some((1, 7)),
            &x_items,
        )?;
    }
    Ok(())
}

/// `gift-eevee@celadon-tea` and `tea`.
fn tea(s: &mut Session) -> PResult<()> {
    s.seg = "gift-eevee@celadon-tea".into();
    special_at(s, "celadon-tea", "gift")?;
    s.seg = "tea".into();
    if !flag(s, "FLAG_GOT_TEA") {
        let lid = super::legs3::obj_id(
            s,
            "MAP_CELADON_CITY_CONDOMINIUMS_1F",
            "CeladonCity_Condominiums_1F_EventScript_TeaWoman",
        )?;
        talk_to(s, "MAP_CELADON_CITY_CONDOMINIUMS_1F", lid)?;
        settle(s)?;
    }
    if !flag(s, "FLAG_GOT_TEA") {
        return Err(PlayError::Death("no Tea".into()));
    }
    Ok(())
}

/// `hideout-walk`, `hideout-grunts` (`fight_lists`: the Game Corner Rocket GRUNT_7 guarding the
/// poster, GRUNT_18 by the Lift Key, GRUNT_16 / GRUNT_17 at Giovanni's door), the poster switch
/// (`CeladonCity_GameCorner/map.json` sign (11, 1)), the Rare Candy ball on B3F (12, 12), the hidden
/// PP Up on B1F (16, 17) (`elixirs` after hideout-grunts), the Lift Key (3, 2) of B4F.
fn hideout_key(s: &mut Session) -> PResult<()> {
    s.seg = "hideout-walk".into();
    let fights = plan_fights(s, "hideout-grunts", &[]);
    if fights.iter().any(|t| t == "TRAINER_TEAM_ROCKET_GRUNT_7") {
        fight_trainer(
            s,
            "TRAINER_TEAM_ROCKET_GRUNT_7",
            "hideout-grunts",
            &Opts::default(),
        )?;
    }
    // The poster: face it from (11, 2) and press A (`CeladonCity_GameCorner_EventScript_Poster`).
    if here(s).is_some_and(|n| s.graph().maps[&n.0].id == "MAP_CELADON_CITY_GAME_CORNER") {
        go_face(s, "MAP_CELADON_CITY_GAME_CORNER", 11, 1, &Opts::default())?;
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    let elixirs: Vec<String> = s
        .plan
        .params
        .elixirs
        .iter()
        .filter(|e| e.after == "hideout-grunts")
        .map(|e| e.kind.clone())
        .collect();
    let o = Opts::default();
    if elixirs.iter().any(|k| k == "ppup") {
        pick(
            s,
            "MAP_ROCKET_HIDEOUT_B1F",
            16,
            17,
            "FLAG_HIDDEN_ITEM_ROCKET_HIDEOUT_B1F_PP_UP",
            "ITEM_PP_UP",
        )?;
    }
    if elixirs.iter().any(|k| k == "candy") {
        pick(
            s,
            "MAP_ROCKET_HIDEOUT_B3F",
            12,
            12,
            "FLAG_HIDE_ROCKET_HIDEOUT_B3F_RARE_CANDY",
            "ITEM_RARE_CANDY",
        )?;
    }
    s.seg = "hideout-grunts".into();
    if fights.iter().any(|t| t == "TRAINER_TEAM_ROCKET_GRUNT_18") {
        fight_trainer(s, "TRAINER_TEAM_ROCKET_GRUNT_18", "hideout-grunts", &o)?;
    }
    if have(s, "ITEM_LIFT_KEY") == 0 {
        go_face(s, "MAP_ROCKET_HIDEOUT_B4F", 3, 2, &o)?;
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    if have(s, "ITEM_LIFT_KEY") == 0 {
        return Err(PlayError::Death("no Lift Key".into()));
    }
    eat_candies(s, "hideout-grunts")?;
    use_pickups(s, "hideout-grunts")?;
    Ok(())
}

/// The elevator to B4F (`RocketHideout_Elevator/scripts.inc`: the floor list B1F, B2F, B4F; row 2),
/// the door Rockets GRUNT_16 / GRUNT_17 (`RocketHideout_B4F/scripts.inc:5-13`: the barrier opens
/// when both are beaten), `hideout-giovanni` (19, 4), the Silph Scope (20, 5), the Calcium ball
/// (6, 23) (`elixirs` calcium after hideout-giovanni).
fn giovanni(s: &mut Session) -> PResult<()> {
    // B1F's barrier (20..21, 19..20) stands until GRUNT_12 is beaten
    // (`RocketHideout_B1F/scripts.inc` `OnLoad` -> `SetBarrier`): planned as walls from the other
    // floors, so the way to the elevator is B2F's door (28, 16), not B1F's layout corridor.
    let barrier = !beaten(s, "TRAINER_TEAM_ROCKET_GRUNT_12");
    let o = Opts {
        patch: if barrier {
            vec![
                ("MAP_ROCKET_HIDEOUT_B1F", 20, 19, true),
                ("MAP_ROCKET_HIDEOUT_B1F", 21, 19, true),
                ("MAP_ROCKET_HIDEOUT_B1F", 20, 20, true),
                ("MAP_ROCKET_HIDEOUT_B1F", 21, 20, true),
            ]
        } else {
            vec![]
        },
        ..Opts::default()
    };
    s.seg = "hideout-grunts".into();
    if !(beaten(s, "TRAINER_TEAM_ROCKET_GRUNT_16") && beaten(s, "TRAINER_TEAM_ROCKET_GRUNT_17")) {
        // The B2F spinner maze from the stairs to the elevator door (28, 16) runs through the
        // Moon Stone ball at (2, 5) (`RocketHideout_B2F/map.json`; a BFS over the layout with the
        // spin tiles finds no way around it): the human picks it up on the way.
        if barrier && !flag(s, "FLAG_HIDE_ROCKET_HIDEOUT_B2F_MOON_STONE") {
            s.event(
                "model-vs-actual",
                "Rocket Hideout B2F: the Moon Stone ball (2, 5) blocks the spinner maze to the \
                 elevator; picked up (not a plan pickup)",
                json!({}),
            );
            pick(
                s,
                "MAP_ROCKET_HIDEOUT_B2F",
                2,
                5,
                "FLAG_HIDE_ROCKET_HIDEOUT_B2F_MOON_STONE",
                "ITEM_MOON_STONE",
            )?;
        }
        go_map(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", &o)?;
        go_face(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", 0, 2, &o)?;
        s.hf.press(keys::A)?;
        super::menu::text_until_multichoice(s)?;
        super::menu::choose(s, 2)?;
        settle(s)?;
        super::legs::take_warp(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", 2, 5, keys::DOWN)?;
        for t in [
            "TRAINER_TEAM_ROCKET_GRUNT_16",
            "TRAINER_TEAM_ROCKET_GRUNT_17",
        ] {
            fight_trainer(s, t, "hideout-grunts", &o)?;
        }
    }
    s.seg = "hideout-giovanni".into();
    if !beaten(s, "TRAINER_BOSS_GIOVANNI") {
        // The barrier closed at map load: leave and re-enter is not needed in the live grid
        // (`setmetatile` patches VMap), walk to Giovanni and talk.
        talk_to(s, "MAP_ROCKET_HIDEOUT_B4F", 1)?;
        settle(s)?;
    }
    if !beaten(s, "TRAINER_BOSS_GIOVANNI") {
        return Err(PlayError::Death("Giovanni not beaten".into()));
    }
    if have(s, "ITEM_SILPH_SCOPE") == 0 {
        go_face(s, "MAP_ROCKET_HIDEOUT_B4F", 20, 5, &o)?;
        s.hf.late()?;
        s.hf.press(keys::A)?;
        settle(s)?;
    }
    if have(s, "ITEM_SILPH_SCOPE") == 0 {
        return Err(PlayError::Death("no Silph Scope".into()));
    }
    if s.plan
        .params
        .elixirs
        .iter()
        .any(|e| e.after == "hideout-giovanni" && e.kind == "calcium")
    {
        pick(
            s,
            "MAP_ROCKET_HIDEOUT_B4F",
            6,
            23,
            "FLAG_HIDE_ROCKET_HIDEOUT_B4F_CALCIUM",
            "ITEM_CALCIUM",
        )?;
        use_pickups(s, "hideout-giovanni")?;
    }
    split(s, "Giovanni 1", "hideout-giovanni")
}

pub fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "mt-moon-1f",
            run: mt_moon_1f,
        },
        Stage {
            name: "miguel",
            run: miguel,
        },
        Stage {
            name: "mega-kick",
            run: mega_kick,
        },
        Stage {
            name: "cerulean",
            run: cerulean,
        },
        Stage {
            name: "misty",
            run: misty,
        },
        Stage {
            name: "rival-cerulean",
            run: rival_cerulean,
        },
        Stage {
            name: "nugget-bridge",
            run: nugget_bridge,
        },
        Stage {
            name: "bill",
            run: bill,
        },
        Stage {
            name: "dig-grunt",
            run: dig_grunt,
        },
        Stage {
            name: "bike-voucher",
            run: bike_voucher,
        },
        Stage {
            name: "hm01",
            run: ss_anne,
        },
        Stage {
            name: "bike",
            run: bike,
        },
        Stage {
            name: "rock-tunnel",
            run: rock_tunnel,
        },
        Stage {
            name: "lavender",
            run: lavender,
        },
        Stage {
            name: "celadon",
            run: celadon,
        },
        Stage {
            name: "xbuy",
            run: xbuy,
        },
        Stage {
            name: "tea",
            run: tea,
        },
        Stage {
            name: "lift-key",
            run: hideout_key,
        },
        Stage {
            name: "giovanni1-beaten",
            run: giovanni,
        },
    ]
}
