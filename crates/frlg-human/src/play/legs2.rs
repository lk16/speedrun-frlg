//! The route from the Koga split on, in the plan's order (`route_wr.rs` `wr_order`, the `route.rs`
//! `leg_*` functions): HM03 taught, the Power Plant ("fly-lavender-2", "powerplant-walk",
//! "powerplant-out"), Cinnabar (`leg_mansion_blaine`) -> the "Blaine" split, the first Sevii trip
//! (`leg_sevii`) -> "Sevii", Silph and Sabrina (`leg_silph`), Erika (`erika_leg`), Giovanni
//! (`leg_viridian`), `leg_league_approach`, the first League up to the first Hall of Fame.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::legs::{center_heal, dex_hook, split, talk_to};
use super::names::{id, names};
use super::prims::settle;
use super::ram::Ram;
use super::session::{PResult, PlayError, Session, Stage};
use super::walk::{go_face, go_map, go_to, here, plan_len, MapId, Node, Opts};

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
    dex_hook(s, "koga-badge")?;
    dex_hook(s, "warden-strength")?;
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
    dex_hook(s, "powerplant-out")?;
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
    dex_hook(s, "mansion")?;
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

pub fn var(s: &mut Session, name: &str) -> u16 {
    let v = names().get(name).unwrap_or_else(|| panic!("no var {name}"));
    s.obs.var(s.hf.rec.emu(), v).unwrap_or(0)
}

/// Settle, and keep settling (a free frame between two map scripts is not the end of a chain of
/// scenes) until `done` holds; `budget` frames.
pub fn settle_until(
    s: &mut Session,
    what: &str,
    budget: u64,
    done: impl Fn(&mut Session) -> bool,
) -> PResult<()> {
    let start = s.frame();
    loop {
        settle(s)?;
        if done(s) {
            return Ok(());
        }
        if s.frame() - start > budget {
            return Err(PlayError::Death(format!(
                "soft-lock: {what} not reached in {budget} frames"
            )));
        }
        s.hf.step(0)?;
    }
}

/// Talk to the Seagallop sailor of `harbor` (object 2 at (8, 6), `*_Harbor/map.json`) and pick
/// row `row` of the destination list (`decompiled/data/scripts/seagallop.inc`
/// `EventScript_ChooseDestFrom*`: before the Rainbow Pass, One Island lists Two, Three; Two Island
/// lists One, Three), then ride to the destination harbor.
fn sail(s: &mut Session, harbor: &str, row: i8, dest: &str) -> PResult<()> {
    talk_to(s, harbor, 2)?;
    super::menu::text_until_multichoice(s)?;
    super::menu::choose(s, row)?;
    let d = s.graph().id(dest);
    settle_until(s, &format!("the Seagallop to {dest}"), 6000, move |s| {
        here(s).is_some_and(|n| n.0 == d)
    })?;
    s.event(
        "decision",
        format!(
            "sailed to {}",
            dest.trim_start_matches("MAP_").trim_end_matches("_HARBOR")
        ),
        json!({"from": harbor, "to": dest}),
    );
    Ok(())
}

/// `leg_sevii` "sevii-bill-boat" + "celio-tri-pass": out of the gym, Bill's scene on Cinnabar
/// (`CinnabarIsland/scripts.inc` `BillScene`, set by Blaine's script `VAR_MAP_SCENE_CINNABAR_ISLAND`
/// 1; the yes/no defaults to YES), the boat, the One Island harbor and town scenes, and Celio's
/// scene in the Pokémon Center (`OneIsland_PokemonCenter_1F/scripts.inc` `MeetCelioScene`: the
/// Meteorite and the Tri-Pass; `VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F` 1 when done).
fn sevii_boat(s: &mut Session) -> PResult<()> {
    dex_hook(s, "blaine-badge")?;
    s.seg = "sevii-bill-boat".into();
    // Out of the gym; the scene takes over on the doorstep (the walk ends wherever it leaves us).
    let gym = s.graph().id("MAP_CINNABAR_ISLAND_GYM");
    go_to(
        s,
        "out of the gym",
        &move |n: &Node| n.0 != gym,
        &Opts::default(),
    )?;
    settle_until(s, "Celio's scene", 12_000, |s| {
        var(s, "VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F") >= 1
    })?;
    let (met, pass) = (have(s, "ITEM_METEORITE"), have(s, "ITEM_TRI_PASS"));
    s.event(
        "decision",
        "One Island: met Celio; Meteorite and Tri-Pass",
        json!({"meteorite": met, "tri_pass": pass}),
    );
    if pass == 0 {
        return Err(PlayError::Fatal("no Tri-Pass after Celio's scene".into()));
    }
    Ok(())
}

/// "one-island-out", "sail-to-two", "two-island-in", "corner-lostelle-scene": the Joyful Game
/// Corner's first-visit scene (`TwoIsland_JoyfulGameCorner/scripts.inc` `HelpFindLostelleScene`)
/// sends the biker off and opens the Three Island biker wall (`VAR_MAP_SCENE_THREE_ISLAND` 2).
fn two_island(s: &mut Session) -> PResult<()> {
    s.seg = "one-island-out".into();
    sail(s, "MAP_ONE_ISLAND_HARBOR", 0, "MAP_TWO_ISLAND_HARBOR")?;
    s.seg = "two-island-in".into();
    go_map(s, "MAP_TWO_ISLAND_JOYFUL_GAME_CORNER", &Opts::default())?;
    s.seg = "corner-lostelle-scene".into();
    settle_until(s, "the Game Corner scene", 6000, |s| {
        var(s, "VAR_MAP_SCENE_TWO_ISLAND_JOYFUL_GAME_CORNER") >= 1
    })
}

/// "two-island-out", "sail-to-three", "three-island-town", the four forced fights: the boss's intro
/// on the coord row y 27 (`BikerBossIntroTrigger`, `VAR_MAP_SCENE_THREE_ISLAND` 2 -> 3), then the
/// fights on row y 26 (`BattleBikersScene`: Goon 1-3 and Paxton; the yes/no defaults to YES;
/// `VAR_MAP_SCENE_THREE_ISLAND` 4 after) (`ThreeIsland/map.json` coord events,
/// `ThreeIsland/scripts.inc`).
fn three_island(s: &mut Session) -> PResult<()> {
    s.seg = "two-island-out".into();
    sail(s, "MAP_TWO_ISLAND_HARBOR", 1, "MAP_THREE_ISLAND_HARBOR")?;
    s.seg = "three-island-town".into();
    for (row, want) in [(27, 3), (26, 4)] {
        if var(s, "VAR_MAP_SCENE_THREE_ISLAND") >= want {
            continue;
        }
        let mid = s.graph().id("MAP_THREE_ISLAND");
        go_to(
            s,
            &format!("Three Island row {row}"),
            &move |n: &Node| n.0 == mid && n.2 == row && (7..=11).contains(&n.1),
            &Opts::default(),
        )?;
        if want == 4 {
            s.seg = "three-island-goon-1".into();
        }
        settle_until(s, "the biker scene", 30_000, move |s| {
            var(s, "VAR_MAP_SCENE_THREE_ISLAND") >= want
        })?;
    }
    s.event("decision", "Three Island: biker wall cleared", json!({}));
    Ok(())
}

/// "bond-bridge" (the plan's hunt there is the dex module's), the two Aroma Ladies in sight
/// (`ThreeIsland_BondBridge/map.json`: Nikki sight 5, Violet sight 2), "berry-forest" to Lostelle
/// at (4, 8) (`ThreeIsland_BerryForest/map.json`), her forced wild Hypno Lv30
/// (`ThreeIsland_BerryForest/scripts.inc` `Lostelle`: `setwildbattle SPECIES_HYPNO, 30`), fled as
/// the model has it ("lostelle-hypno"), and the warp back to the Game Corner.
fn berry_forest(s: &mut Session) -> PResult<()> {
    s.seg = "bond-bridge".into();
    go_map(s, "MAP_THREE_ISLAND_BOND_BRIDGE", &hm_opts())?;
    dex_hook(s, "bond-bridge")?;
    s.seg = "berry-forest".into();
    go_face(s, "MAP_THREE_ISLAND_BERRY_FOREST", 4, 8, &hm_opts())?;
    dex_hook(s, "berry-forest")?;
    s.seg = "lostelle-hypno".into();
    s.hf.press(keys::A)?;
    settle_until(s, "Lostelle's rescue", 8000, |s| {
        flag(s, "FLAG_RESCUED_LOSTELLE")
            && here(s).is_some_and(|n| {
                let g = s.graph();
                n.0 == g.id("MAP_TWO_ISLAND_JOYFUL_GAME_CORNER")
            })
    })?;
    settle_until(s, "the found-Lostelle scene", 4000, |s| {
        var(s, "VAR_MAP_SCENE_TWO_ISLAND_JOYFUL_GAME_CORNER") >= 3
    })?;
    s.event(
        "decision",
        "Berry Forest: Lostelle rescued (Hypno fled)",
        json!({}),
    );
    Ok(())
}

/// "deliver-meteorite": Lostelle's daddy at (5, 5) takes the Meteorite and gives the Moon Stone
/// (`TwoIsland_JoyfulGameCorner/scripts.inc` `GiveDaddyMeteorite`;
/// `VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F` 2), then "two-island-out-2", "sail-to-one",
/// "one-island-in" and Bill's leave scene on the coord column x 12 of the One Island center
/// (`LeaveOneIslandScene`: the Seagallop home, `VAR_MAP_SCENE_CINNABAR_ISLAND` 3, and the return
/// scene on Cinnabar) -> the "Sevii" split.
fn sevii_return(s: &mut Session) -> PResult<()> {
    s.seg = "deliver-meteorite".into();
    talk_to(s, "MAP_TWO_ISLAND_JOYFUL_GAME_CORNER", 1)?;
    settle(s)?;
    if var(s, "VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F") < 2 {
        return Err(PlayError::Fatal("the Meteorite was not delivered".into()));
    }
    let moon = have(s, "ITEM_MOON_STONE");
    s.event(
        "decision",
        "delivered the Meteorite (Moon Stone)",
        json!({"moon_stones": moon}),
    );
    dex_hook(s, "deliver-meteorite")?;
    s.seg = "two-island-out-2".into();
    sail(s, "MAP_TWO_ISLAND_HARBOR", 0, "MAP_ONE_ISLAND_HARBOR")?;
    s.seg = "one-island-in".into();
    let pc = s.graph().id("MAP_ONE_ISLAND_POKEMON_CENTER_1F");
    let cin = s.graph().id("MAP_CINNABAR_ISLAND");
    // The scene fires on the column and runs to Cinnabar inside the walk's own settle.
    go_to(
        s,
        "the One Island center's x 12 column",
        &move |n: &Node| n.0 == cin || (n.0 == pc && n.1 == 12 && (6..=9).contains(&n.2)),
        &Opts::default(),
    )?;
    s.seg = "blaine-badge-and-sevii-scene".into();
    settle_until(s, "back on Cinnabar", 12_000, move |s| {
        here(s).is_some_and(|n| n.0 == cin) && var(s, "VAR_MAP_SCENE_CINNABAR_ISLAND") >= 4
    })?;
    split(s, "Sevii", "blaine-badge-and-sevii-scene")?;
    dex_hook(s, "blaine-badge-and-sevii-scene")?;
    dex_hook(s, "stones")
}

/// `xbuy@fly-celadon-2` (the plan's `xbuys` after fly-celadon-2) plus what the dex module needs
/// bought on this visit: Great Balls (`ball: great`, `ball_batch`) at 2F
/// (`CeladonCity_DepartmentStore_2F_Items`), the evolution stones the plan buys (`evolutions`
/// with `buy: true`) at 4F (`..._4F_Items`), the X items at 5F (`..._5F_XItems`); the League
/// heals are Hyper Potions, sold at the Saffron Mart (`SaffronCity_Mart/scripts.inc`), on the way
/// to Silph.
fn celadon_shopping(s: &mut Session) -> PResult<()> {
    s.event(
        "model-vs-actual",
        "the model's 'stones' segment (right after Sevii) evolves with stones the plan buys \
         (evolutions buy: true) only at the Celadon store 4F on fly-celadon-2, after it",
        json!({"segment": "stones"}),
    );
    s.seg = "fly-celadon-2".into();
    fly_from_here(s, "MAP_CINNABAR_ISLAND", "MAPSEC_CELADON_CITY")?;
    s.seg = "xbuy@fly-celadon-2".into();
    let xb: Vec<crate::params::XBuy> = s
        .plan
        .params
        .xbuys
        .iter()
        .filter(|x| x.after == "fly-celadon-2")
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
    let heals: u32 = xb.iter().map(|x| x.heals).sum();
    let revives: u32 = xb.iter().map(|x| x.revives).sum();
    let balls = s.plan.params.ball_batch.max(10) as u16;
    let _ = super::shop::buy(
        s,
        "MAP_CELADON_CITY_DEPARTMENT_STORE_2F",
        Some((1, 8)),
        &[("ITEM_GREAT_BALL", balls)],
    )?;
    let mut stones: Vec<String> = vec![];
    for e in &s.plan.params.evolutions {
        if e.via == "stone" && e.buy && !e.item.is_empty() {
            let it = format!("ITEM_{}_STONE", e.item);
            if !stones.contains(&it) {
                stones.push(it);
            }
        }
    }
    let stones: Vec<String> = stones.into_iter().filter(|it| have(s, it) == 0).collect();
    if !stones.is_empty() {
        let want: Vec<(&str, u16)> = stones.iter().map(|it| (it.as_str(), 1)).collect();
        let _ = super::shop::buy(
            s,
            "MAP_CELADON_CITY_DEPARTMENT_STORE_4F",
            Some((3, 13)),
            &want,
        )?;
    }
    if !x_items.is_empty() {
        let _ = super::shop::buy(
            s,
            "MAP_CELADON_CITY_DEPARTMENT_STORE_5F",
            Some((1, 7)),
            &x_items,
        )?;
    }
    if s.model
        .names
        .iter()
        .any(|n| n == "tm-ice_beam@fly-celadon-2")
    {
        s.event(
            "model-vs-actual",
            "TM13 Ice Beam (tms after fly-celadon-2, cost 80000) is a Game Corner prize: no Coin \
             Case in this run, not bought",
            json!({"segment": "tm-ice_beam@fly-celadon-2"}),
        );
    }
    let mut saffron: Vec<(&str, u16)> = vec![];
    if heals > 0 {
        saffron.push(("ITEM_HYPER_POTION", heals as u16));
    }
    if revives > 0 {
        saffron.push(("ITEM_REVIVE", revives as u16));
    }
    if !saffron.is_empty() {
        let _ = super::items::mount_bike(s)?;
        let _ = super::shop::buy(s, "MAP_SAFFRON_CITY_MART", None, &saffron)?;
    }
    Ok(())
}

/// One Silph card-key door: its floor map, the flag that says it is open, and the barrier tiles a
/// closed door has (`EventScript_Close<F>Door<N>`, the `setmetatile ..., 1` lines of
/// `decompiled/data/scripts/silphco_doors.inc`; each floor's `OnLoad` closes the doors whose flag
/// is clear, e.g. `SilphCo_5F/scripts.inc:5-8`).
struct SilphDoor {
    map: &'static str,
    flag: String,
    tiles: Vec<(i16, i16)>,
}

fn silph_doors() -> Vec<SilphDoor> {
    const FLOORS: [&str; 10] = ["2F", "3F", "4F", "5F", "6F", "7F", "8F", "9F", "10F", "11F"];
    const MAPS: [&str; 10] = [
        "MAP_SILPH_CO_2F",
        "MAP_SILPH_CO_3F",
        "MAP_SILPH_CO_4F",
        "MAP_SILPH_CO_5F",
        "MAP_SILPH_CO_6F",
        "MAP_SILPH_CO_7F",
        "MAP_SILPH_CO_8F",
        "MAP_SILPH_CO_9F",
        "MAP_SILPH_CO_10F",
        "MAP_SILPH_CO_11F",
    ];
    let root = std::env::var_os("FRLG_DECOMP_RO")
        .or_else(|| std::env::var_os("FRLG_DECOMP"))
        .map(std::path::PathBuf::from)
        .expect("$FRLG_DECOMP");
    let text = std::fs::read_to_string(root.join("data/scripts/silphco_doors.inc"))
        .expect("silphco_doors.inc");
    let mut out: Vec<SilphDoor> = vec![];
    for line in text.lines() {
        let l = line.trim();
        if let Some(head) = l.split("::").next().filter(|_| l.contains("::")) {
            if let Some(rest) = head.strip_prefix("EventScript_Close") {
                // "5FDoor1" / "6FDoor"
                if let Some((fl, n)) = rest.split_once("Door") {
                    if let Some(i) = FLOORS.iter().position(|f| *f == fl) {
                        let flag = if n.is_empty() {
                            format!("FLAG_SILPH_{fl}_DOOR")
                        } else {
                            format!("FLAG_SILPH_{fl}_DOOR_{n}")
                        };
                        out.push(SilphDoor {
                            map: MAPS[i],
                            flag,
                            tiles: vec![],
                        });
                        continue;
                    }
                }
            }
        }
        if l.starts_with("return") || l.ends_with("::") {
            continue;
        }
        if let (Some(d), Some(args)) = (out.last_mut(), l.strip_prefix("setmetatile")) {
            let a: Vec<&str> = args.split(',').map(str::trim).collect();
            if a.len() == 4 && a[3] != "0" {
                if let (Ok(x), Ok(y)) = (a[0].parse(), a[1].parse()) {
                    if !d.tiles.contains(&(x, y)) {
                        d.tiles.push((x, y));
                    }
                }
            }
        }
    }
    out
}

/// Walk inside Silph Co. to `goal`, opening card-key doors on the way: plan with the closed doors
/// shut; if there is no way, plan as if every closed door were open (Card Key in the bag), walk up
/// to the first closed door the path crosses, face it and press A (`EventScript_TryUnlockDoor`,
/// `silphco_doors.inc`), and plan again.
fn silph_to(s: &mut Session, what: &str, goal: &dyn Fn(&Node) -> bool) -> PResult<()> {
    let doors = silph_doors();
    for _ in 0..12 {
        let mut shut = Opts::default();
        let mut open = Opts::default();
        let mut closed_tiles: Vec<(MapId, i16, i16, usize)> = vec![];
        let g = s.graph();
        for (k, d) in doors.iter().enumerate() {
            let is_open = flag(s, &d.flag);
            for &(x, y) in &d.tiles {
                shut.patch.push((d.map, x, y, !is_open));
                open.patch.push((d.map, x, y, false));
                if !is_open {
                    closed_tiles.push((g.id(d.map), x, y, k));
                }
            }
        }
        if plan_len(s, None, goal, &shut).is_some() {
            return go_to(s, what, goal, &shut);
        }
        if have(s, "ITEM_CARD_KEY") == 0 {
            return Err(PlayError::Fatal(format!(
                "Silph: no way to {what} without the Card Key"
            )));
        }
        let cur = here(s).ok_or("no position")?;
        let nodes = super::walk::plan_nodes(s, cur, goal, &open)
            .ok_or_else(|| PlayError::Fatal(format!("Silph: no way to {what} even with doors")))?;
        let Some(i) = nodes
            .iter()
            .position(|n| closed_tiles.iter().any(|&(m, x, y, _)| (m, x, y) == *n))
        else {
            // The live grid disagrees with the patch: walk it anyway.
            return go_to(s, what, goal, &open);
        };
        let (door_node, stand) = (nodes[i], nodes[i.saturating_sub(1)]);
        let k = closed_tiles
            .iter()
            .find(|&&(m, x, y, _)| (m, x, y) == door_node)
            .map(|c| c.3)
            .unwrap();
        go_to(
            s,
            "a Silph card-key door",
            &move |n: &Node| *n == stand,
            &shut,
        )?;
        let dir = match (door_node.1 - stand.1, door_node.2 - stand.2) {
            (0, -1) => keys::UP,
            (0, 1) => keys::DOWN,
            (-1, 0) => keys::LEFT,
            _ => keys::RIGHT,
        };
        super::walk::face(s, dir)?;
        s.hf.press(keys::A)?;
        settle(s)?;
        let ok = flag(s, &doors[k].flag);
        s.event(
            "decision",
            format!(
                "Silph: opened {} with the Card Key{}",
                doors[k].flag.trim_start_matches("FLAG_SILPH_"),
                if ok { "" } else { " (did not open)" }
            ),
            json!({"door": doors[k].flag, "open": ok}),
        );
    }
    Err(PlayError::Death(format!(
        "soft-lock: Silph: 12 doors toward {what}"
    )))
}

/// `leg_silph_walk` "silph-walk" (to the Card Key: 5F (22, 21), `SilphCo_5F/map.json`).
fn silph_card_key(s: &mut Session) -> PResult<()> {
    // Not in the model (no heal between Blaine and Sabrina's badge): the Sevii fights and hunts
    // leave the carry low, and the first branch into Silph blacked out to Scientist Beau.
    s.event(
        "model-vs-actual",
        "healing at the Saffron center before Silph (model: no heal from Blaine to sabrina-badge)",
        json!({"segment": "silph-walk"}),
    );
    center_heal(s, "MAP_SAFFRON_CITY_POKEMON_CENTER_1F")?;
    s.seg = "silph-walk".into();
    let _ = super::items::mount_bike(s)?;
    go_map(s, "MAP_SILPH_CO_1F", &Opts::default())?;
    let f5 = s.graph().id("MAP_SILPH_CO_5F");
    silph_to(s, "the Card Key", &move |n: &Node| {
        n.0 == f5 && (n.1 - 22).abs() + (n.2 - 21).abs() == 1
    })?;
    let cur = here(s).unwrap();
    let dir = match (22 - cur.1, 21 - cur.2) {
        (0, -1) => keys::UP,
        (0, 1) => keys::DOWN,
        (-1, 0) => keys::LEFT,
        _ => keys::RIGHT,
    };
    super::walk::face(s, dir)?;
    s.hf.press(keys::A)?;
    settle(s)?;
    if have(s, "ITEM_CARD_KEY") == 0 {
        return Err(PlayError::Fatal("no Card Key after the 5F ball".into()));
    }
    s.event("decision", "Silph 5F: Card Key", json!({}));
    Ok(())
}

/// "rival-silph", "silph-card-doors", "silph-grunts", "silph-giovanni": Giovanni's scene fires on
/// the coord events (5, 15) / (6, 15) of 11F (`SilphCo_11F/map.json`,
/// `GiovanniTriggerLeft/Right`); the rival's on (2, 4) / (2, 5) of 7F
/// (`SilphCo_7F/map.json`), met only if the path crosses it.
fn silph_giovanni(s: &mut Session) -> PResult<()> {
    s.seg = "silph-grunts".into();
    let f11 = s.graph().id("MAP_SILPH_CO_11F");
    silph_to(s, "Giovanni's trigger", &move |n: &Node| {
        n.0 == f11 && n.2 == 15 && (5..=6).contains(&n.1)
    })?;
    s.seg = "silph-giovanni".into();
    settle(s)?;
    let rival = var(s, "VAR_MAP_SCENE_SILPH_CO_7F");
    if rival == 0 {
        s.event(
            "model-vs-actual",
            "the Silph rival (rival-silph, forced in the model) was not met: the path to 11F \
             does not cross his 7F trigger (SilphCo_7F/map.json coord (2, 4)/(2, 5))",
            json!({"segment": "rival-silph"}),
        );
    }
    if var(s, "VAR_MAP_SCENE_SILPH_CO_11F") == 0 {
        return Err(PlayError::Fatal("Giovanni's scene did not run".into()));
    }
    s.event("decision", "Silph 11F: Giovanni beaten", json!({}));
    Ok(())
}

/// "master-ball": the president at (9, 9) of 11F gives the Master Ball
/// (`SilphCo_11F/scripts.inc` `President`); then the dex module's specials after silph-giovanni
/// (the Lapras gift on 7F), and out of Silph.
fn silph_out(s: &mut Session) -> PResult<()> {
    s.seg = "master-ball".into();
    talk_to(s, "MAP_SILPH_CO_11F", 1)?;
    settle(s)?;
    let mb = have(s, "ITEM_MASTER_BALL");
    s.event(
        "decision",
        "Silph 11F: the president's Master Ball",
        json!({"master_ball": mb}),
    );
    dex_hook(s, "silph-giovanni")?;
    let saffron = s.graph().id("MAP_SAFFRON_CITY");
    silph_to(s, "out of Silph", &move |n: &Node| n.0 == saffron)
}

/// "sabrina-walk", "sabrina" -> the "Sabrina" split (`leg_silph_walk`): the gym's teleport pads
/// are warp events (`SaffronCity_Gym/map.json`, 33 warps); Sabrina at (14, 11).
fn sabrina(s: &mut Session) -> PResult<()> {
    s.seg = "sabrina-walk".into();
    go_face(s, "MAP_SAFFRON_CITY_GYM", 14, 11, &Opts::default())?;
    s.seg = "sabrina".into();
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_BADGE06_GET") {
        return Err(PlayError::Fatal("no Marsh Badge after Sabrina".into()));
    }
    split(s, "Sabrina", "sabrina-badge")?;
    // The plan's heal after the badge (`extra_heals` sabrina-badge) and its gift (Hitmonlee:
    // the dex module's).
    center_heal(s, "MAP_SAFFRON_CITY_POKEMON_CENTER_1F")?;
    dex_hook(s, "sabrina-badge")
}

/// "fly-celadon-3", `erika_leg`: Erika at (6, 4) behind the gym's Cut trees
/// (`CeladonCity_Gym/map.json`) -> the "Erika" split; the plan's heal and evolutions after the
/// badge (`extra_heals`, `evolutions` after erika-badge).
fn erika(s: &mut Session) -> PResult<()> {
    s.seg = "fly-celadon-3".into();
    fly_from_here(s, "MAP_SAFFRON_CITY", "MAPSEC_CELADON_CITY")?;
    s.seg = "erika-walk".into();
    let _ = super::items::mount_bike(s)?;
    go_face(s, "MAP_CELADON_CITY_GYM", 6, 4, &hm_opts())?;
    s.seg = "erika".into();
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_BADGE04_GET") {
        return Err(PlayError::Fatal("no Rainbow Badge after Erika".into()));
    }
    split(s, "Erika", "erika-badge")?;
    center_heal(s, "MAP_CELADON_CITY_POKEMON_CENTER_1F")?;
    dex_hook(s, "erika-badge")
}

/// HM04 needs a carrier (see `teach_surf`): Lapras, the Silph 7F gift (it went to the PC with a
/// full party; `tmhm_learnsets.h` Lapras HM04). At the Viridian center the human deposits Doduo
/// (no role left in the plan: `evo-dodrio@koga-badge` needs Lv31, it is Lv26 and never fights)
/// and withdraws Lapras, then teaches Strength for Victory Road
/// (`VictoryRoad_*F/map.json` `EventScript_StrengthBoulder`).
fn strength_carrier(s: &mut Session) -> PResult<()> {
    let lapras = id("SPECIES_LAPRAS");
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    if !party.iter().any(|m| m.species == lapras) {
        let slot = party
            .iter()
            .position(|m| m.species == id("SPECIES_DODUO"))
            .unwrap_or(party.len() - 1);
        // Two PC sessions: switching from deposit to withdraw inside one sometimes closes the PC.
        if party.len() >= 6 {
            super::shop::pc(
                s,
                "MAP_VIRIDIAN_CITY_POKEMON_CENTER_1F",
                &[super::shop::PcOp::Deposit(slot)],
            )?;
        }
        super::shop::pc(
            s,
            "MAP_VIRIDIAN_CITY_POKEMON_CENTER_1F",
            &[super::shop::PcOp::Withdraw(lapras)],
        )?;
    }
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let Some(slot) = party.iter().position(|m| m.species == lapras) else {
        return Err(PlayError::Fatal("no Lapras for Strength".into()));
    };
    if party[slot].knows(id("MOVE_STRENGTH")).is_none() {
        super::items::teach_tmhm(s, "ITEM_HM04", slot)?;
    }
    s.event(
        "model-vs-actual",
        "Strength carrier: Lapras withdrawn at Viridian (PC visit not in the model) and taught HM04",
        json!({"segment": "fly-viridian"}),
    );
    Ok(())
}

/// `leg_viridian`: "fly-viridian" (and the plan's trade and Diglett's Cave hunt after it), the
/// Strength carrier, the gym (spin tiles: the planner treats them as floor and replans after
/// each slide), Giovanni at (2, 2) -> the "Giovanni" split.
fn giovanni(s: &mut Session) -> PResult<()> {
    s.seg = "fly-viridian".into();
    fly_from_here(s, "MAP_CELADON_CITY", "MAPSEC_VIRIDIAN_CITY")?;
    dex_hook(s, "fly-viridian")?;
    strength_carrier(s)?;
    s.seg = "viridian-gym-walk".into();
    // A gym trainer who walked up to the player stays where the battle left him
    // (`trainerbattle` approach, no walk back) and can wall off a spinner pocket (Takashi at
    // (10, 2) ends on (10, 4), the only way out of the x 9-10 pocket). The human steps out of the
    // gym and back in, which reloads the trainers at their map positions.
    let strict = Opts {
        strict: true,
        ..hm_opts()
    };
    let mut reentries = 0;
    loop {
        match go_face(s, "MAP_VIRIDIAN_CITY_GYM", 2, 2, &strict) {
            Ok(()) => break,
            Err(PlayError::Death(m)) if m.starts_with("blocked") && reentries < 3 => {
                reentries += 1;
                s.event(
                    "model-vs-actual",
                    "Viridian Gym: a beaten trainer blocks the way back; out of the gym and in \
                     again to reset him (not in the model)",
                    json!({"reentry": reentries}),
                );
                go_map(s, "MAP_VIRIDIAN_CITY", &hm_opts())?;
            }
            Err(e) => return Err(e),
        }
    }
    s.seg = "giovanni".into();
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_BADGE08_GET") {
        return Err(PlayError::Fatal("no Earth Badge after Giovanni".into()));
    }
    split(s, "Giovanni", "giovanni-badge")?;
    center_heal(s, "MAP_VIRIDIAN_CITY_POKEMON_CENTER_1F")?;
    dex_hook(s, "giovanni-badge")
}

/// The floor switches of the current map: the coord events whose script is a `FloorSwitch`
/// (`VictoryRoad_*F/map.json`), each with the tiles its script opens (`setmetatile x, y, ..., 0`
/// in that label of the map's `scripts.inc`, e.g. `VictoryRoad_1F/scripts.inc`
/// `FloorSwitch`: the rock barrier at (12, 14..15)). A boulder pushed onto the switch runs it.
type FloorSwitch = ((i16, i16), Vec<(i16, i16)>, String);
type Barrier = (&'static str, Vec<(i16, i16)>, bool);

fn floor_switches(map_name: &str) -> Vec<FloorSwitch> {
    let root = std::env::var_os("FRLG_DECOMP_RO")
        .or_else(|| std::env::var_os("FRLG_DECOMP"))
        .map(std::path::PathBuf::from)
        .expect("$FRLG_DECOMP");
    let dir = root.join("data/maps").join(map_name);
    let Ok(mj) = std::fs::read_to_string(dir.join("map.json")) else {
        return vec![];
    };
    let Ok(j) = serde_json::from_str::<serde_json::Value>(&mj) else {
        return vec![];
    };
    let scripts = std::fs::read_to_string(dir.join("scripts.inc")).unwrap_or_default();
    let mut out = vec![];
    for c in j["coord_events"].as_array().into_iter().flatten() {
        let Some(sc) = c["script"].as_str().filter(|sc| sc.contains("FloorSwitch")) else {
            continue;
        };
        let (Some(x), Some(y)) = (c["x"].as_i64(), c["y"].as_i64()) else {
            continue;
        };
        let mut opens = vec![];
        let mut var = String::new();
        let mut inside = false;
        for line in scripts.lines() {
            let l = line.trim();
            if l.ends_with("::") {
                inside = l.trim_end_matches("::") == sc;
                continue;
            }
            if !inside {
                continue;
            }
            if let Some(v) = l
                .strip_prefix("setvar ")
                .and_then(|r| r.strip_suffix(", 100"))
            {
                var = v.trim().to_string();
            }
            if let Some(args) = l.strip_prefix("setmetatile") {
                let a: Vec<&str> = args.split(',').map(str::trim).collect();
                if a.len() == 4 && a[3] == "0" {
                    if let (Ok(tx), Ok(ty)) = (a[0].parse(), a[1].parse()) {
                        opens.push((tx, ty));
                    }
                }
            }
        }
        out.push(((x as i16, y as i16), opens, var));
    }
    out
}

/// One push of a Strength boulder: the player stands on `stand` and walks `dir`.
#[derive(Clone, Copy, Debug)]
struct Push {
    stand: (i16, i16),
    dir: (i16, i16),
}

/// Shortest (in player moves) sequence of pushes on the current map that puts a boulder on a
/// floor switch whose barrier is still closed. Breadth-first over (player, boulders); a boulder
/// moves one tile when the player walks into it and the tile behind it is free floor (no wall,
/// water, ledge, warp, person or boulder); the player stays where he stood
/// (`decompiled/src/field_player_avatar.c` `TryPushBoulder`).
fn boulder_plan(
    m: &super::walk::MapInfo,
    start: (i16, i16),
    boulders: Vec<(i16, i16)>,
    people: &std::collections::HashSet<(i16, i16)>,
    targets: &[(i16, i16)],
    z_player: u8,
) -> Option<Vec<Push>> {
    use std::collections::{HashMap, VecDeque};
    // Elevation rules of `ObjectEventUpdateElevation` / `IsElevationMismatchAt`
    // (`decompiled/src/event_object_movement.c`): 0 and 15 never block, 15 keeps the old value.
    let elev = |p: (i16, i16)| m.tile(p.0, p.1).map_or(0, |t| t.elevation);
    let z0 = match z_player % 15 {
        0 => elev(start) % 15,
        z => z,
    };
    let mismatch = |z: u8, e: u8| z != 0 && e != 0 && e != 15 && e != z;
    let new_z = |z: u8, from: (i16, i16), e: u8| {
        if e == 15 || elev(from) == 15 {
            z
        } else {
            e
        }
    };
    // Plain floor a boulder can sit on or the player can walk on.
    let floor = |x: i16, y: i16| -> bool {
        m.tile(x, y).is_some_and(|t| {
            t.collision == 0
                && !super::walk::MB_WATER.contains(&t.behavior)
                && super::walk::ledge(t.behavior).is_none()
                && !super::walk::is_warp_tile(t.behavior)
        }) && !people.contains(&(x, y))
    };
    // (player, player elevation, boulders)
    type St = ((i16, i16), u8, Vec<(i16, i16)>);
    let mut b0 = boulders;
    b0.sort();
    let s0: St = (start, z0, b0);
    let mut prev: HashMap<St, Option<(St, Option<Push>)>> = HashMap::new();
    prev.insert(s0.clone(), None);
    let mut q = VecDeque::from([s0]);
    let mut found: Option<St> = None;
    while let Some(st) = q.pop_front() {
        if st.2.iter().any(|b| targets.contains(b)) {
            found = Some(st);
            break;
        }
        if prev.len() > 400_000 {
            break;
        }
        let (p, z, bs) = &st;
        let (p, z) = (*p, *z);
        for d in [(0i16, -1i16), (0, 1), (-1, 0), (1, 0)] {
            let n = (p.0 + d.0, p.1 + d.1);
            let next: St;
            let mut push = None;
            if let Some(k) = bs.iter().position(|&b| b == n) {
                // A boulder is pushed from the same elevation only.
                if mismatch(z, elev(n)) {
                    continue;
                }
                let t = (n.0 + d.0, n.1 + d.1);
                if !(floor(t.0, t.1) || targets.contains(&t)) || bs.contains(&t) {
                    continue;
                }
                let mut nb = bs.clone();
                nb[k] = t;
                nb.sort();
                next = (p, z, nb);
                push = Some(Push { stand: p, dir: d });
            } else if m
                .tile(n.0, n.1)
                .and_then(|t| super::walk::ledge(t.behavior))
                == Some(d)
            {
                // A ledge in the walking direction: hop over it.
                let l = (n.0 + d.0, n.1 + d.1);
                if !floor(l.0, l.1) || bs.contains(&l) {
                    continue;
                }
                let e = elev(l);
                next = (l, if e % 15 == 0 { z } else { e }, bs.clone());
            } else {
                // The player may also stand on an arrow warp (it only fires when its arrow is
                // pressed: `TryArrowWarp`), e.g. the Victory Road 1F entrance (11, 20).
                let arrow = m
                    .tile(n.0, n.1)
                    .is_some_and(|t| (0x62..=0x65).contains(&t.behavior) && t.collision == 0);
                if !(floor(n.0, n.1) || (arrow && !people.contains(&n))) {
                    continue;
                }
                let e = elev(n);
                if mismatch(z, e) {
                    continue;
                }
                next = (n, new_z(z, p, e), bs.clone());
            }
            if prev.contains_key(&next) {
                continue;
            }
            prev.insert(next.clone(), Some((st.clone(), push)));
            q.push_back(next);
        }
    }
    let mut cur = found?;
    let mut pushes = vec![];
    while let Some(Some((p, push))) = prev.get(&cur).cloned() {
        if let Some(x) = push {
            pushes.push(x);
        }
        cur = p;
    }
    pushes.reverse();
    Some(pushes)
}

const VR: [&str; 3] = ["VictoryRoad_1F", "VictoryRoad_2F", "VictoryRoad_3F"];
const VR_MAPS: [&str; 3] = [
    "MAP_VICTORY_ROAD_1F",
    "MAP_VICTORY_ROAD_2F",
    "MAP_VICTORY_ROAD_3F",
];

/// Every Victory Road rock barrier with its state: (map, tiles, closed). A barrier is closed until
/// its switch's var is 100 (`VictoryRoad_*F/scripts.inc` `OnLoad` / `SetRockBarrier*`).
fn vr_barriers(s: &mut Session) -> Vec<Barrier> {
    let mut out = vec![];
    for (i, fl) in VR.iter().enumerate() {
        for (_, opens, v) in floor_switches(fl) {
            let closed = v.is_empty() || var(s, &v) != 100;
            out.push((VR_MAPS[i], opens, closed));
        }
    }
    out
}

/// Walk to `goal` through Victory Road, solving the floor-switch boulder puzzles
/// (`VictoryRoad_1F/2F/3F/map.json`: `EventScript_StrengthBoulder` objects, `FloorSwitch` coord
/// events): plan with the closed barriers shut; with no way, plan as if they were open, walk to the
/// floor of the first closed barrier the path crosses, and push a boulder onto that barrier's
/// switch (shortest push sequence, [`boulder_plan`]). Strength is switched on at the first push
/// (`FLAG_SYS_USE_STRENGTH`; the boulder's script asks to use it, YES by default).
fn strength_to(s: &mut Session, what: &str, goal: &dyn Fn(&Node) -> bool) -> PResult<()> {
    for _ in 0..12 {
        let bars = vr_barriers(s);
        let mut shut = hm_opts();
        let mut open = hm_opts();
        for (m, tiles, closed) in &bars {
            for &(x, y) in tiles {
                shut.patch.push((m, x, y, *closed));
                open.patch.push((m, x, y, false));
            }
        }
        if plan_len(s, None, goal, &shut).is_some() {
            return go_to(s, what, goal, &shut);
        }
        let cur = here(s).ok_or("no position")?;
        let g = s.graph();
        let nodes = super::walk::plan_nodes(s, cur, goal, &open).ok_or_else(|| {
            PlayError::Fatal(format!("{what}: no way even with the barriers open"))
        })?;
        let closed_at = |n: &Node| {
            bars.iter()
                .any(|(m, tiles, closed)| *closed && g.id(m) == n.0 && tiles.contains(&(n.1, n.2)))
        };
        let Some(i) = nodes.iter().position(closed_at) else {
            return go_to(s, what, goal, &open);
        };
        let bar_map = nodes[i].0;
        if bar_map != cur.0 {
            let j = nodes.iter().position(|n| n.0 == bar_map).unwrap_or(i);
            let to = nodes[j];
            go_to(
                s,
                "the next Victory Road floor",
                &move |n: &Node| *n == to,
                &shut,
            )?;
            continue;
        }
        let minfo = g.maps.get(&cur.0).cloned().ok_or("no map")?;
        let mut m = super::walk::live_map(s, cur.0).unwrap_or_else(|| minfo.clone());
        // The barriers as their switches left them (the live grid can lag a map load).
        for (bm, tiles, closed) in vr_barriers(s) {
            if s.graph().id(bm) == cur.0 {
                for (x, y) in tiles {
                    m.set_collision(x, y, closed);
                }
            }
        }
        let bar_tile = (nodes[i].1, nodes[i].2);
        let targets: Vec<(i16, i16)> = floor_switches(&minfo.name)
            .into_iter()
            .filter(|(_, opens, _)| opens.contains(&bar_tile))
            .map(|(sw, _, _)| sw)
            .collect();
        let live = Ram::new(&s.obs).objects(s.hf.rec.emu());
        let is_boulder = |id: u8| {
            minfo
                .objects
                .iter()
                .any(|o| o.local_id == id && o.is_boulder())
        };
        // Objects only spawn near the camera: boulders not loaded now stand where the map puts
        // them (unless their flag hides them).
        let mut boulders: Vec<(i16, i16)> = live
            .iter()
            .filter(|o| is_boulder(o.local_id))
            .map(|o| (o.x, o.y))
            .collect();
        for o in minfo.objects.iter().filter(|o| o.is_boulder()) {
            let loaded = live.iter().any(|l| l.local_id == o.local_id);
            let hidden = o.flag != 0 && s.obs.flag(s.hf.rec.emu(), o.flag).unwrap_or(false);
            if !loaded && !hidden {
                boulders.push((o.x, o.y));
            }
        }
        let people: std::collections::HashSet<(i16, i16)> = live
            .iter()
            // The player's own object (local id 0xFF, `OBJ_EVENT_ID_PLAYER`) is not an obstacle.
            .filter(|o| !is_boulder(o.local_id) && o.local_id != 0xFF)
            .map(|o| (o.x, o.y))
            .collect();
        // One boulder moves, the others stay where they are: a search per boulder keeps the
        // state space at (player, that boulder), and the shortest wins.
        let zp = Ram::new(&s.obs).player_elevation(s.hf.rec.emu());
        let mut best: Option<Vec<Push>> = None;
        for (k, &b) in boulders.iter().enumerate() {
            let mut fixed = people.clone();
            for (j, &o) in boulders.iter().enumerate() {
                if j != k {
                    fixed.insert(o);
                }
            }
            if let Some(p) = boulder_plan(&m, (cur.1, cur.2), vec![b], &fixed, &targets, zp) {
                if best.as_ref().is_none_or(|bp| p.len() < bp.len()) {
                    best = Some(p);
                }
            }
        }
        let Some(pushes) = best else {
            // Victory Road 2F's second switch (14, 19) is fed from 3F: the boulder at 3F (33, 18)
            // pushed into the hole (34, 18) (`MB_FALL_WARP`, a 3F warp to 2F) drops onto 2F and
            // shows the hidden 2F boulder at (33, 19) (`FLAG_HIDE_VICTORY_ROAD_2F_BOULDER`, the
            // link is in the 3F object's `trainer_type` field, `VictoryRoad_3F/map.json`).
            if minfo.name == "VictoryRoad_2F" && flag(s, "FLAG_HIDE_VICTORY_ROAD_2F_BOULDER") {
                vr_hole(s)?;
                continue;
            }
            return Err(PlayError::Fatal(format!(
                "{}: no boulder reaches the switch of the barrier at {bar_tile:?}",
                minfo.name
            )));
        };
        s.event(
            "decision",
            format!(
                "{}: {} Strength pushes onto the floor switch {:?}",
                minfo.name,
                pushes.len(),
                targets
            ),
            json!({"pushes": pushes.len(), "switches": targets}),
        );
        do_pushes(s, cur.0, &pushes)?;
    }
    Err(PlayError::Death(format!(
        "soft-lock: 12 boulder rounds toward {what}"
    )))
}

/// `leg_league_approach` "route22" (the plan's hunt there) and the late rival on the coord column
/// x 33 of Route 22 (`Route22/map.json`, `LateRivalTrigger*`, `VAR_MAP_SCENE_ROUTE22` 3).
fn route22(s: &mut Session) -> PResult<()> {
    s.seg = "route22".into();
    let _ = super::items::mount_bike(s)?;
    let r22 = s.graph().id("MAP_ROUTE22");
    go_to(
        s,
        "the Route 22 rival column",
        &move |n: &Node| n.0 == r22 && n.1 == 33 && (4..=6).contains(&n.2),
        &hm_opts(),
    )?;
    s.seg = "rival-route22".into();
    settle(s)?;
    dex_hook(s, "route22")
}

/// "route23", "badge-gates" (the guards' coord events only talk once the badges are in the bag:
/// `Route23/map.json`), the plan's Route 23 hunt, "victory-road" with its boulder puzzles, out on
/// Route 23's north part, Indigo Plateau -> "indigo-heal" and the split.
fn victory_road(s: &mut Session) -> PResult<()> {
    s.seg = "route23".into();
    go_map(s, "MAP_ROUTE23", &hm_opts())?;
    dex_hook(s, "route23")?;
    s.seg = "victory-road".into();
    go_map(s, "MAP_VICTORY_ROAD_1F", &hm_opts())?;
    let plateau = s.graph().id("MAP_INDIGO_PLATEAU_EXTERIOR");
    strength_to(s, "Indigo Plateau", &move |n: &Node| n.0 == plateau)?;
    dex_hook(s, "victory-road")?;
    dex_hook(s, "dex-fill")?;
    s.seg = "indigo-heal".into();
    center_heal(s, "MAP_INDIGO_PLATEAU_POKEMON_CENTER_1F")?;
    split(s, "Victory Road", "indigo-heal")
}

/// One Elite Four room: walk in from the previous room (or the Indigo center's north door), face
/// the member and talk; the battle; the door behind them opens (`PokemonLeague_*Room/scripts.inc`).
/// Positions from `PokemonLeague_*Room/map.json`: Lorelei, Bruno, Agatha at (6, 5), Lance (6, 8).
fn e4_member(
    s: &mut Session,
    map: &str,
    who: &str,
    x: i16,
    y: i16,
    flag_name: &str,
) -> PResult<()> {
    s.seg = format!("e4-transit-{who}");
    // In through the door; the room's entry scene walks the player in (`EnterRoom`, the entrance
    // row is wall in the layout until then).
    let mid = s.graph().id(map);
    e4_door(s)?;
    go_to(s, map, &move |n: &Node| n.0 == mid, &Opts::default())?;
    settle(s)?;
    go_face(s, map, x, y, &Opts::default())?;
    s.seg = who.to_string();
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, flag_name) {
        return Err(PlayError::Fatal(format!("{who} not beaten")));
    }
    let t = s.timer().unwrap_or(0.0);
    let p50 = s.model.p50(who);
    s.event(
        "split",
        match p50 {
            Some(p) => format!("{who}: {t:.0}s vs model p50 {p:.0}s ({:+.0}s)", t - p),
            None => format!("{who}: {t:.0}s"),
        },
        json!({"actual": t, "p50": p50, "name": who, "seg": who}),
    );
    Ok(())
}

fn lorelei(s: &mut Session) -> PResult<()> {
    e4_member(
        s,
        "MAP_POKEMON_LEAGUE_LORELEIS_ROOM",
        "lorelei",
        6,
        5,
        "FLAG_DEFEATED_LORELEI",
    )
}

fn bruno(s: &mut Session) -> PResult<()> {
    e4_member(
        s,
        "MAP_POKEMON_LEAGUE_BRUNOS_ROOM",
        "bruno",
        6,
        5,
        "FLAG_DEFEATED_BRUNO",
    )
}

fn agatha(s: &mut Session) -> PResult<()> {
    e4_member(
        s,
        "MAP_POKEMON_LEAGUE_AGATHAS_ROOM",
        "agatha",
        6,
        5,
        "FLAG_DEFEATED_AGATHA",
    )
}

fn lance(s: &mut Session) -> PResult<()> {
    e4_member(
        s,
        "MAP_POKEMON_LEAGUE_LANCES_ROOM",
        "lance",
        6,
        8,
        "FLAG_DEFEATED_LANCE",
    )
}

/// "champion" and "hall-of-fame-1": the Champion's room runs its scene on entry
/// (`PokemonLeague_ChampionsRoom/scripts.inc` `EnterRoom`: the rival walks up, the battle, Oak),
/// then the Hall of Fame room's (`PokemonLeague_HallOfFame/scripts.inc` `EnterRoom`:
/// `special EnterHallOfFame`). The split is the first frame of `CB2_DoHallOfFameScreen`
/// (`decompiled/src/hall_of_fame.c:361`), the first Hall of Fame.
fn champion(s: &mut Session) -> PResult<()> {
    s.seg = "champion".into();
    let room = s.graph().id("MAP_POKEMON_LEAGUE_CHAMPIONS_ROOM");
    e4_door(s)?;
    go_to(
        s,
        "the Champion's room",
        &move |n: &Node| n.0 == room,
        &Opts::default(),
    )?;
    let start = s.frame();
    loop {
        if s.obs.callback2_is(s.hf.rec.emu(), "CB2_DoHallOfFameScreen") {
            break;
        }
        if s.frame() - start > 60_000 {
            return Err(PlayError::Death(
                "soft-lock: no Hall of Fame after the Champion".into(),
            ));
        }
        if s.obs.in_battle(s.hf.rec.emu()) {
            super::fallback_battle::battle(s)?;
            continue;
        }
        if super::prims::is_free(s) {
            // Free between the scenes (after the battle, before Oak's walk): walk north into the
            // Hall of Fame (`PokemonLeague_ChampionsRoom/map.json` warp (6, 2)).
            if here(s).is_some_and(|n| n.0 == room) {
                super::legs::take_warp(s, "MAP_POKEMON_LEAGUE_CHAMPIONS_ROOM", 6, 3, keys::UP)?;
                continue;
            }
            s.hf.step(0)?;
            continue;
        }
        s.hf.late()?;
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
    split(s, "Hall of Fame", "hall-of-fame-1")
}

/// Walk to each push's standing tile, face the boulder, (switch Strength on), and walk into it
/// until it has moved.
fn do_pushes(s: &mut Session, mid: MapId, pushes: &[Push]) -> PResult<()> {
    for p in pushes {
        let stand = p.stand;
        go_to(
            s,
            "a boulder",
            &move |n: &Node| *n == (mid, stand.0, stand.1),
            &Opts::default(),
        )?;
        let key = match p.dir {
            (0, -1) => keys::UP,
            (0, 1) => keys::DOWN,
            (-1, 0) => keys::LEFT,
            _ => keys::RIGHT,
        };
        super::walk::face(s, key)?;
        if !flag(s, "FLAG_SYS_USE_STRENGTH") {
            s.hf.press(keys::A)?;
            settle(s)?;
            super::walk::face(s, key)?;
        }
        let at = (stand.0 + p.dir.0, stand.1 + p.dir.1);
        let boulder_at = |s: &mut Session| {
            Ram::new(&s.obs)
                .objects(s.hf.rec.emu())
                .iter()
                .any(|o| (o.x, o.y) == at)
        };
        let mut fed = 0;
        while boulder_at(s) {
            if fed > 240 {
                return Err(PlayError::Death(
                    "soft-lock: the boulder did not move".into(),
                ));
            }
            let k = if super::prims::is_free(s) { key } else { 0 };
            s.hf.step(k)?;
            fed += 1;
        }
        settle(s)?;
    }
    Ok(())
}

/// To Victory Road 3F and the 3F boulder (33, 18) into the hole (34, 18) (see `strength_to`);
/// the 3F floor switch (7, 7) first if its barrier (12, 12..13) is in the way.
fn vr_hole(s: &mut Session) -> PResult<()> {
    let f3 = s.graph().id("MAP_VICTORY_ROAD_3F");
    let bars = vr_barriers(s);
    let mut shut = hm_opts();
    for (m, tiles, closed) in &bars {
        for &(x, y) in tiles {
            shut.patch.push((m, x, y, *closed));
        }
    }
    go_to(s, "Victory Road 3F", &move |n: &Node| n.0 == f3, &shut)?;
    // The 3F switch (7, 7) first (its barrier (12, 12..13) closes the way round to the hole
    // pocket), then the boulder at (33, 18) into the hole.
    for target in [(7i16, 7i16), (34, 18)] {
        if !flag(s, "FLAG_HIDE_VICTORY_ROAD_2F_BOULDER") {
            break;
        }
        if target == (7, 7) && var(s, "VAR_MAP_SCENE_VICTORY_ROAD_3F") == 100 {
            continue;
        }
        let cur = here(s).ok_or("no position")?;
        let g = s.graph();
        let minfo = g.maps.get(&f3).cloned().ok_or("no map")?;
        let mut m = super::walk::live_map(s, f3).unwrap_or_else(|| minfo.clone());
        // The barriers as their switches left them (the live grid can lag a map load).
        for (bm, tiles, closed) in vr_barriers(s) {
            if s.graph().id(bm) == f3 {
                for (x, y) in tiles {
                    m.set_collision(x, y, closed);
                }
            }
        }
        let live = Ram::new(&s.obs).objects(s.hf.rec.emu());
        let is_boulder = |id: u8| {
            minfo
                .objects
                .iter()
                .any(|o| o.local_id == id && o.is_boulder())
        };
        // Objects only spawn near the camera: boulders not loaded now stand where the map puts
        // them (unless their flag hides them).
        let mut boulders: Vec<(i16, i16)> = live
            .iter()
            .filter(|o| is_boulder(o.local_id))
            .map(|o| (o.x, o.y))
            .collect();
        for o in minfo.objects.iter().filter(|o| o.is_boulder()) {
            let loaded = live.iter().any(|l| l.local_id == o.local_id);
            let hidden = o.flag != 0 && s.obs.flag(s.hf.rec.emu(), o.flag).unwrap_or(false);
            if !loaded && !hidden {
                boulders.push((o.x, o.y));
            }
        }
        let people: std::collections::HashSet<(i16, i16)> = live
            .iter()
            .filter(|o| !is_boulder(o.local_id) && o.local_id != 0xFF)
            .map(|o| (o.x, o.y))
            .collect();
        let zp = Ram::new(&s.obs).player_elevation(s.hf.rec.emu());
        let mut best: Option<Vec<Push>> = None;
        for (k, &b) in boulders.iter().enumerate() {
            let mut fixed = people.clone();
            for (j, &o) in boulders.iter().enumerate() {
                if j != k {
                    fixed.insert(o);
                }
            }
            if let Some(p) = boulder_plan(&m, (cur.1, cur.2), vec![b], &fixed, &[target], zp) {
                if best.as_ref().is_none_or(|bp| p.len() < bp.len()) {
                    best = Some(p);
                }
            }
        }
        let Some(pushes) = best else {
            eprintln!(
                "vr_hole: no push to {target:?} from {:?} z {zp}; boulders {boulders:?}; people {people:?}",
                (cur.1, cur.2)
            );
            continue;
        };
        s.event(
            "decision",
            format!(
                "VictoryRoad_3F: {} Strength pushes to {target:?} from {:?}: {:?}",
                pushes.len(),
                (cur.1, cur.2),
                pushes.iter().map(|p| (p.stand, p.dir)).collect::<Vec<_>>()
            ),
            json!({"pushes": pushes.len(), "target": target, "from": [cur.1, cur.2],
                   "stands": pushes.iter().map(|p| [p.stand.0, p.stand.1]).collect::<Vec<_>>()}),
        );
        do_pushes(s, f3, &pushes)?;
    }
    if flag(s, "FLAG_HIDE_VICTORY_ROAD_2F_BOULDER") {
        return Err(PlayError::Fatal(
            "VictoryRoad_3F: the boulder did not go down the hole".into(),
        ));
    }
    Ok(())
}

/// Out of a beaten member's room by its north door. The door is wall in the layout (its metatile is
/// swapped by the room's script after the win, and the planner only reads the live collision, not
/// the new warp behavior): stand below it and hold UP until the map changes. Exits: (6, 2) in
/// Lorelei's, Bruno's and Agatha's rooms, (6, 5) in Lance's (`PokemonLeague_*Room/map.json`).
fn e4_door(s: &mut Session) -> PResult<()> {
    let rooms: [(&str, i16, i16); 4] = [
        ("MAP_POKEMON_LEAGUE_LORELEIS_ROOM", 6, 2),
        ("MAP_POKEMON_LEAGUE_BRUNOS_ROOM", 6, 2),
        ("MAP_POKEMON_LEAGUE_AGATHAS_ROOM", 6, 2),
        ("MAP_POKEMON_LEAGUE_LANCES_ROOM", 6, 5),
    ];
    let Some(cur) = here(s) else {
        return Ok(());
    };
    let g = s.graph();
    let Some(&(m, x, y)) = rooms.iter().find(|r| g.id(r.0) == cur.0) else {
        return Ok(());
    };
    super::legs::take_warp(s, m, x, y + 1, keys::UP)
}

/// The League heals (`league_heals` 14: "heal items the bag holds at the start of each League
/// round ... the Indigo mart tops up to this many at `league_heal_price`", `params.rs`): Max
/// Potions from the Indigo Plateau clerk at (0, 7) (`IndigoPlateau_PokemonCenter_1F_Items`),
/// counting the Hyper and Max Potions already in the bag, as far as the money goes.
fn indigo_mart(s: &mut Session) -> PResult<()> {
    s.seg = "indigo-heal".into();
    let want = s.plan.params.league_heals as u16;
    let have_now = have(s, "ITEM_HYPER_POTION") + have(s, "ITEM_MAX_POTION");
    let n = want.saturating_sub(have_now);
    if n > 0 {
        let _ = super::shop::buy(
            s,
            "MAP_INDIGO_PLATEAU_POKEMON_CENTER_1F",
            Some((0, 7)),
            &[("ITEM_MAX_POTION", n)],
        )?;
    }
    Ok(())
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
        Stage {
            name: "sevii-boat",
            run: sevii_boat,
        },
        Stage {
            name: "two-island",
            run: two_island,
        },
        Stage {
            name: "three-island",
            run: three_island,
        },
        Stage {
            name: "berry-forest",
            run: berry_forest,
        },
        Stage {
            name: "sevii",
            run: sevii_return,
        },
        Stage {
            name: "celadon-shopping",
            run: celadon_shopping,
        },
        Stage {
            name: "silph-card-key",
            run: silph_card_key,
        },
        Stage {
            name: "silph-giovanni",
            run: silph_giovanni,
        },
        Stage {
            name: "silph-out",
            run: silph_out,
        },
        Stage {
            name: "sabrina",
            run: sabrina,
        },
        Stage {
            name: "erika",
            run: erika,
        },
        Stage {
            name: "giovanni",
            run: giovanni,
        },
        Stage {
            name: "route22",
            run: route22,
        },
        Stage {
            name: "victory-road",
            run: victory_road,
        },
        Stage {
            name: "indigo-mart",
            run: indigo_mart,
        },
        Stage {
            name: "lorelei",
            run: lorelei,
        },
        Stage {
            name: "bruno",
            run: bruno,
        },
        Stage {
            name: "agatha",
            run: agatha,
        },
        Stage {
            name: "lance",
            run: lance,
        },
        Stage {
            name: "hall-of-fame",
            run: champion,
        },
    ]
}
