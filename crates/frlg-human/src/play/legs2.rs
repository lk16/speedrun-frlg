//! The route from the Koga split on, in the plan's order (`route_wr.rs` `wr_order`, the `route.rs`
//! `leg_*` functions): HM03 taught, the Power Plant ("fly-lavender-2", "powerplant-walk",
//! "powerplant-out"), Cinnabar (`leg_mansion_blaine`) -> the "Blaine" split, the first Sevii trip
//! (`leg_sevii`) -> "Sevii", Silph and Sabrina (`leg_silph`), Erika (`erika_leg`), Giovanni
//! (`leg_viridian`), `leg_league_approach`, the first League up to the first Hall of Fame.
use frlg_emu::keys;
use serde_json::json;

use super::legs::{center_heal, dex_hook, split};
use super::names::{id, names};
use super::prims::settle;
use super::ram::Ram;
use super::session::{PResult, PlayError, Session, Stage};
use super::walk::{go_face, go_map, go_to, here, plan_len, Node, Opts};

pub fn have(s: &mut Session, item: &str) -> u16 {
    Ram::new(&s.obs).count(s.hf.rec.emu(), id(item))
}

pub fn flag(s: &mut Session, name: &str) -> bool {
    let f = names()
        .get(name)
        .unwrap_or_else(|| panic!("no flag {name}"));
    s.obs.flag(s.hf.rec.emu(), f).unwrap_or(false)
}

/// Opts with Cut and Surf usable.
pub fn hm_opts() -> Opts {
    Opts {
        cut: true,
        surf: true,
        ..Opts::default()
    }
}

/// Pick up the item ball of `map` whose script contains `script` (`map.json` object events).
pub fn pick_ball(s: &mut Session, map: &str, script: &str, o: &Opts) -> PResult<()> {
    let g = s.graph();
    let ob = g.maps[&g.id(map)]
        .objects
        .iter()
        .find(|ob| ob.script.contains(script))
        .map(|ob| (ob.x, ob.y, ob.flag))
        .ok_or_else(|| PlayError::Fatal(format!("no object {script} on {map}")))?;
    if ob.2 != 0 && s.obs.flag(s.hf.rec.emu(), ob.2).unwrap_or(false) {
        return Ok(());
    }
    go_face(s, map, ob.0, ob.1, o)?;
    s.hf.press(keys::A)?;
    settle(s)?;
    s.event(
        "decision",
        format!("picked up the item ball {script}"),
        json!({"map": map, "script": script}),
    );
    Ok(())
}

/// Fly out from wherever the player is (outdoors first if needed: Fly works outdoors only).
pub fn fly_from_here(s: &mut Session, outdoor: &str, mapsec: &str) -> PResult<()> {
    go_map(s, outdoor, &hm_opts())?;
    super::items::fly(s, mapsec)
}

/// "warden-strength" (1400 frames in the model) is where the model teaches the HMs. HM03 goes to
/// the carry: the plan's `keep_moves` list MOVE_SURF, and only the lead learns it of this party
/// (`decompiled/src/data/pokemon/tmhm_learnsets.h`: Blastoise HM03; Pidgey, Weedle, Paras, Eevee,
/// Doduo do not). HM04 has no carrier in the party: none of the five others learns Strength
/// (same file), and Blastoise's four slots are all `keep_moves` after Surf and Blizzard. It waits
/// for the Lapras gift (Silph 7F, learns HM04), before Victory Road needs it.
fn teach_surf(s: &mut Session) -> PResult<()> {
    dex_hook(s, "koga-badge");
    dex_hook(s, "warden-strength");
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let surf = id("MOVE_SURF");
    if !party.iter().any(|m| m.knows(surf).is_some()) {
        super::items::teach_tmhm(s, "ITEM_HM03", 0)?;
    }
    s.event(
        "model-vs-actual",
        "HM04 Strength not taught: no party mon learns it (tmhm_learnsets.h) and the carry's four \
         slots are keep_moves; the model's warden-strength teaches it with no carrier",
        json!({"segment": "warden-strength", "cite": "decompiled/src/data/pokemon/tmhm_learnsets.h"}),
    );
    Ok(())
}

/// "fly-lavender-2" + "powerplant-walk" (260 tiles of surf in the model): the Power Plant's door is
/// Route 10 (7, 40) (`Route10/map.json` warp 2), on the river between the Route 10 Pokémon Center
/// (13, 20) and Lavender. The Route 10 center is a fly point once visited
/// (`FLAG_WORLD_MAP_ROUTE10_POKEMON_CENTER_1F`, `include/constants/flags.h:1431`), set on the Rock
/// Tunnel walk; the human flies there, 20 rows closer than Lavender.
fn power_plant(s: &mut Session) -> PResult<()> {
    let r10 = flag(s, "FLAG_WORLD_MAP_ROUTE10_POKEMON_CENTER_1F");
    let (sec, outdoor) = if r10 {
        ("MAPSEC_ROUTE_10", "MAP_FUCHSIA_CITY")
    } else {
        ("MAPSEC_LAVENDER_TOWN", "MAP_FUCHSIA_CITY")
    };
    if r10 {
        s.event(
            "model-vs-actual",
            "flying to the Route 10 Pokemon Center, not Lavender (fly-lavender-2): the plant's door \
             is 20 rows from it",
            json!({"segment": "fly-lavender-2"}),
        );
    }
    fly_from_here(s, outdoor, sec)?;
    s.seg = "powerplant-walk".into();
    let _ = super::items::mount_bike(s)?;
    go_map(s, "MAP_POWER_PLANT", &hm_opts())?;
    Ok(())
}

/// "powerplant-out": the plan's Thunder Stone (`elixirs` kind thunder, after powerplant-out: the
/// item ball at (45, 4), `PowerPlant/map.json`) and the hunt (`hunts` MAP_POWER_PLANT, 200 steps,
/// the dex module's), then out the way in.
fn power_plant_out(s: &mut Session) -> PResult<()> {
    s.seg = "powerplant-out".into();
    pick_ball(s, "MAP_POWER_PLANT", "ItemThunderStone", &Opts::default())?;
    dex_hook(s, "powerplant-out");
    go_map(s, "MAP_ROUTE10", &hm_opts())?;
    Ok(())
}

/// `leg_mansion_blaine` "fly-pallet" + "surf-cinnabar": Pallet, Route 21 by Surf, Cinnabar.
fn to_cinnabar(s: &mut Session) -> PResult<()> {
    super::items::fly(s, "MAPSEC_PALLET_TOWN")?;
    s.seg = "surf-cinnabar".into();
    go_map(s, "MAP_CINNABAR_ISLAND", &hm_opts())?;
    Ok(())
}

const MANSION: [&str; 4] = [
    "MAP_POKEMON_MANSION_1F",
    "MAP_POKEMON_MANSION_2F",
    "MAP_POKEMON_MANSION_3F",
    "MAP_POKEMON_MANSION_B1F",
];

/// The Mansion's door tiles for switch state `on`: every `setmetatile x, y, tile, collision` of
/// `PokemonMansion_EventScript_PressSwitch_<floor>` (on) or `ResetSwitch_<floor>` (off)
/// (`decompiled/data/scripts/pokemon_mansion.inc`). One flag, `FLAG_POKEMON_MANSION_SWITCH_STATE`,
/// drives all four floors; each floor's `OnLoad` replays the list (`PokemonMansion_1F/scripts.inc`).
fn mansion_patch(on: bool) -> Vec<(&'static str, i16, i16, bool)> {
    let root = std::env::var_os("FRLG_DECOMP_RO")
        .or_else(|| std::env::var_os("FRLG_DECOMP"))
        .map(std::path::PathBuf::from)
        .expect("$FRLG_DECOMP");
    let text = std::fs::read_to_string(root.join("data/scripts/pokemon_mansion.inc"))
        .expect("pokemon_mansion.inc");
    let mut out = vec![];
    let mut cur: Option<&'static str> = None;
    let want = if on { "PressSwitch_" } else { "ResetSwitch_" };
    for line in text.lines() {
        let l = line.trim();
        if let Some(lbl) = l.strip_suffix("::") {
            cur = lbl
                .strip_prefix("PokemonMansion_EventScript_")
                .and_then(|r| {
                    let fl = r.strip_prefix(want)?;
                    ["1F", "2F", "3F", "B1F"]
                        .iter()
                        .position(|f| *f == fl)
                        .map(|i| MANSION[i])
                });
            continue;
        }
        let (Some(m), Some(args)) = (cur, l.strip_prefix("setmetatile")) else {
            continue;
        };
        let a: Vec<&str> = args.split(',').map(str::trim).collect();
        if a.len() == 4 {
            if let (Ok(x), Ok(y)) = (a[0].parse(), a[1].parse()) {
                out.push((m, x, y, a[3] != "0"));
            }
        }
    }
    out
}

fn mansion_opts(on: bool) -> Opts {
    Opts {
        patch: mansion_patch(on),
        ..Opts::default()
    }
}

/// Walk inside the Mansion to a node satisfying `goal`, pressing statue switches on the way when
/// the doors are wrong: each round takes the direct path if the current switch state has one,
/// else the statue minimising (steps to it now) + (steps from it to the goal after the toggle).
/// Statues: the `*_EventScript_Statue` signs, faced north (`PokemonMansion_*/map.json`).
fn mansion_to(s: &mut Session, what: &str, goal: &dyn Fn(&Node) -> bool) -> PResult<()> {
    let statues: [(&str, i16, i16); 5] = [
        (MANSION[0], 5, 5),
        (MANSION[1], 2, 16),
        (MANSION[2], 12, 5),
        (MANSION[3], 24, 29),
        (MANSION[3], 27, 5),
    ];
    let mut last: Option<usize> = None;
    for _round in 0..8 {
        let on = flag(s, "FLAG_POKEMON_MANSION_SWITCH_STATE");
        let o = mansion_opts(on);
        if plan_len(s, None, goal, &o).is_some() {
            return go_to(s, what, goal, &o);
        }
        // Dijkstra over (where, switch state): from the player, or from in front of statue k
        // after pressing it; edges walk to a statue under the current state and toggle it. The
        // first statue of the cheapest chain that ends in a path to the goal is pressed, then
        // everything is planned again.
        let g = s.graph();
        let stands: Vec<Node> = statues
            .iter()
            .map(|&(m, x, y)| (g.id(m), x, y + 1))
            .collect();
        let opts = [mansion_opts(false), mansion_opts(true)];
        // node index: 0 = the player now (state `on`); 1 + 2k + st = in front of statue k, state st.
        let n_nodes = 1 + 2 * statues.len();
        let mut dist = vec![usize::MAX; n_nodes];
        let mut first = vec![usize::MAX; n_nodes];
        let mut done = vec![false; n_nodes];
        dist[0] = 0;
        let mut best: Option<(usize, usize)> = None; // (first statue, total)
        while let Some(u) = (0..n_nodes)
            .filter(|&i| !done[i] && dist[i] != usize::MAX)
            .min_by_key(|&i| dist[i])
        {
            done[u] = true;
            if best.is_some_and(|(_, b)| dist[u] >= b) {
                break;
            }
            let (from, st) = if u == 0 {
                (None, on)
            } else {
                (Some(stands[(u - 1) / 2]), (u - 1) % 2 == 1)
            };
            let os = &opts[usize::from(st)];
            if u != 0 {
                if let Some(c) = plan_len(s, from, goal, os) {
                    if best.is_none_or(|(_, b)| dist[u] + c < b) {
                        best = Some((first[u], dist[u] + c));
                    }
                }
            }
            for (k, &sk) in stands.iter().enumerate() {
                if u == 0 && last == Some(k) {
                    continue;
                }
                let v = 1 + 2 * k + usize::from(!st);
                if done[v] {
                    continue;
                }
                let Some(c) = plan_len(s, from, &|n: &Node| *n == sk, os) else {
                    continue;
                };
                // A press costs about as much as a dozen steps of text.
                let d = dist[u] + c + 12;
                if d < dist[v] {
                    dist[v] = d;
                    first[v] = if u == 0 { k } else { first[u] };
                }
            }
        }
        let Some((i, cost)) = best else {
            return Err(PlayError::Fatal(format!(
                "Mansion: no chain of statue switches reaches {what}"
            )));
        };
        let (m, x, y) = statues[i];
        super::walk::go(s, m, x, y + 1, &o)?;
        super::walk::face(s, keys::UP)?;
        // "A secret switch! Press it?" (MSGBOX_YESNO): answered explicitly, and the closing text
        // ended with B so that no A lands on the statue again after `releaseall` (a second toggle).
        s.hf.press(keys::A)?;
        super::menu::text_until_yesno(s)?;
        super::menu::answer(s, true)?;
        super::prims::press_until(s, "the switch text", keys::B, 600, |s| {
            super::prims::is_free(s)
        })?;
        settle(s)?;
        let now = flag(s, "FLAG_POKEMON_MANSION_SWITCH_STATE");
        if now == on {
            s.event(
                "warn",
                "Mansion: the statue press did not toggle the doors",
                json!({"statue": i}),
            );
        }
        s.event(
            "decision",
            format!(
                "Mansion: pressed the statue switch on {} ({x}, {y}) toward {what}: doors {}",
                m.trim_start_matches("MAP_POKEMON_MANSION_"),
                if now { "flipped" } else { "back" }
            ),
            json!({"statue": i, "switch_on": now, "est_steps": cost}),
        );
        last = Some(i);
    }
    Err(PlayError::Death(format!(
        "soft-lock: Mansion: 8 switch rounds toward {what}"
    )))
}

/// Next to `(x, y)` of `map` inside the Mansion, facing it, A.
fn mansion_face(s: &mut Session, map: &'static str, x: i16, y: i16) -> PResult<()> {
    let mid = s.graph().id(map);
    mansion_to(s, &format!("next to {map} ({x}, {y})"), &move |n: &Node| {
        n.0 == mid && (n.1 - x).abs() + (n.2 - y).abs() == 1
    })?;
    let cur = here(s).unwrap();
    let dir = match (x - cur.1, y - cur.2) {
        (0, -1) => keys::UP,
        (0, 1) => keys::DOWN,
        (-1, 0) => keys::LEFT,
        _ => keys::RIGHT,
    };
    super::walk::face(s, dir)?;
    s.hf.press(keys::A)?;
    settle(s)
}

/// `leg_mansion_blaine` "mansion" + "secret-key": the Secret Key ball at B1F (5, 7) (the gym door
/// is locked without it: `CinnabarIsland/scripts.inc`), with the plan's pickups on the way: TM14
/// Blizzard at B1F (23, 4) (`tms` MOVE_BLIZZARD after mansion; `keep_moves`), the hidden Elixir at
/// B1F (35, 5) and the hidden Moon Stone at 1F (2, 21) (`elixirs` elixir / moon after mansion;
/// bg events of `PokemonMansion_B1F/map.json`, `PokemonMansion_1F/map.json`).
fn mansion(s: &mut Session) -> PResult<()> {
    s.seg = "mansion".into();
    let _ = super::items::mount_bike(s)?;
    go_map(s, MANSION[0], &Opts::default())?;
    if !flag(s, "FLAG_HIDDEN_ITEM_POKEMON_MANSION_1F_MOON_STONE") {
        mansion_face(s, MANSION[0], 2, 21)?;
        let n = have(s, "ITEM_MOON_STONE");
        s.event(
            "decision",
            "Mansion 1F: hidden Moon Stone",
            json!({"moon": n}),
        );
    }
    if !flag(s, "FLAG_HIDE_POKEMON_MANSION_B1F_TM14") {
        mansion_face(s, MANSION[3], 23, 4)?;
        let n = have(s, "ITEM_TM14");
        s.event("decision", "Mansion B1F: TM14 Blizzard", json!({"tm14": n}));
    }
    if !flag(s, "FLAG_HIDDEN_ITEM_POKEMON_MANSION_B1F_ELIXIR") {
        mansion_face(s, MANSION[3], 35, 5)?;
        let n = have(s, "ITEM_ELIXIR");
        s.event(
            "decision",
            "Mansion B1F: hidden Elixir",
            json!({"elixir": n}),
        );
    }
    s.seg = "secret-key".into();
    mansion_face(s, MANSION[3], 5, 7)?;
    if have(s, "ITEM_SECRET_KEY") == 0 {
        return Err(PlayError::Fatal("no Secret Key after the B1F ball".into()));
    }
    s.event("decision", "Mansion B1F: Secret Key", json!({}));
    dex_hook(s, "mansion");
    // Out on foot (no Escape Rope in the bag), through whatever doors the switch left.
    let cin = s.graph().id("MAP_CINNABAR_ISLAND");
    mansion_to(s, "out of the Mansion", &move |n: &Node| n.0 == cin)
}

/// Walk to stand on one of `stands` of `map`, then face `dir`.
pub fn go_any_face(
    s: &mut Session,
    map: &str,
    stands: &[(i16, i16)],
    dir: u16,
    o: &Opts,
) -> PResult<()> {
    let mid = s.graph().id(map);
    let st: Vec<(i16, i16)> = stands.to_vec();
    go_to(
        s,
        &format!("{map} {stands:?}"),
        &move |n: &Node| n.0 == mid && st.contains(&(n.1, n.2)),
        o,
    )?;
    super::walk::face(s, dir)
}

/// The plan's TM teach (`tms` after `anchor`) of `item` to the carry (party slot 0).
fn teach_tm(s: &mut Session, item: &str, anchor: &str) -> PResult<()> {
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

/// Cinnabar Gym (`leg_mansion_blaine` "blaine-walk", "blaine-quiz-text", "blaine"): the six quiz
/// machines, each a pair of signs faced north, each a yes/no whose right answer opens the next
/// door (`CinnabarIsland_Gym/scripts.inc` `Quiz1`..`Quiz6`: YES, NO, NO, NO, YES, NO; a wrong
/// answer starts the guarding trainer's battle). The human knows the answers. A quiz is skipped
/// when Blaine (5, 4) can already be reached.
fn blaine(s: &mut Session) -> PResult<()> {
    teach_tm(s, "ITEM_TM14", "mansion")?;
    // Not in the model (no heal between the Mansion and Blaine): Blastoise leaves the Mansion at
    // half HP with 8 Surf PP. The center is next to the gym door.
    s.event(
        "model-vs-actual",
        "healing at the Cinnabar center before Blaine (model: none after the Mansion)",
        json!({"segment": "blaine-walk"}),
    );
    center_heal(s, "MAP_CINNABAR_ISLAND_POKEMON_CENTER_1F")?;
    s.seg = "blaine-walk".into();
    go_map(s, "MAP_CINNABAR_ISLAND_GYM", &Opts::default())?;
    let gym = "MAP_CINNABAR_ISLAND_GYM";
    let quiz: [((i16, i16), bool); 6] = [
        ((22, 10), true),
        ((15, 2), false),
        ((13, 10), false),
        ((13, 17), false),
        ((1, 18), true),
        ((1, 10), false),
    ];
    let mid = s.graph().id(gym);
    s.seg = "blaine-quiz-text".into();
    for (i, &((x, y), yes)) in quiz.iter().enumerate() {
        let near_blaine = |n: &Node| n.0 == mid && (n.1 - 5).abs() + (n.2 - 4).abs() == 1;
        if plan_len(s, None, &near_blaine, &Opts::default()).is_some() {
            break;
        }
        go_any_face(
            s,
            gym,
            &[(x, y + 1), (x + 1, y + 1)],
            keys::UP,
            &Opts::default(),
        )?;
        s.hf.press(keys::A)?;
        super::menu::text_until_yesno(s)?;
        super::menu::answer(s, yes)?;
        settle(s)?;
        let done = flag(s, &format!("FLAG_CINNABAR_GYM_QUIZ_{}", i + 1));
        s.event(
            "decision",
            format!(
                "Blaine quiz {}: answered {}",
                i + 1,
                if yes { "YES" } else { "NO" }
            ),
            json!({"quiz": i + 1, "door_open": done}),
        );
    }
    s.seg = "blaine".into();
    go_face(s, gym, 5, 4, &Opts::default())?;
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_BADGE07_GET") {
        return Err(PlayError::Fatal("no Volcano Badge after Blaine".into()));
    }
    split(s, "Blaine", "blaine-badge")
}

pub fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "teach-surf",
            run: teach_surf,
        },
        Stage {
            name: "power-plant",
            run: power_plant,
        },
        Stage {
            name: "power-plant-out",
            run: power_plant_out,
        },
        Stage {
            name: "to-cinnabar",
            run: to_cinnabar,
        },
        Stage {
            name: "mansion",
            run: mansion,
        },
        Stage {
            name: "blaine",
            run: blaine,
        },
    ]
}
