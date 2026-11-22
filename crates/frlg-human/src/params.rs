//! Plan parameters: the machine-readable block of a `human-run/plans/NNN-*.md` file.

use serde::Deserialize;

/// One execution-loss event (`Params::slow`).
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SlowEvent {
    /// Segment name; a trailing `*` matches every segment with that prefix.
    pub seg: String,
    pub p: f64,
    /// Mean of the exponential cost, seconds.
    pub mean_s: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Hunt {
    /// Segment name this hunt is inserted after.
    pub after: String,
    pub site: String,
    /// `land`, `water` or `fish`.
    pub area: String,
    /// Step budget (fishing: casts).
    pub steps: u32,
    /// Species the human leaves alone even if new (too rare to be worth the steps).
    pub skip: Vec<String>,
    /// Frames to get to the hunting spot and back (walking out of the way), run speed.
    pub detour_tiles: u32,
    /// The hunting spot is on the route (the WR order catches in the Safari Zone, the Power Plant,
    /// the Mansion on the way): `detour_tiles` is the whole extra walk (the Fly-point table is not
    /// used) and a Safari entry is already paid by the route's own fee segment.
    pub on_route: bool,
    /// Leave after this many new species were caught on this visit (0 = stay until the site is
    /// complete or the step budget is spent).
    pub stop_after: u32,
    /// Lead-level Repel (0 = none): the first healthy party mon is a Lv N mon and a Repel is
    /// active, so land encounters below N never start (`wild_encounter.c:601`).
    pub repel_level: u32,
}

impl Default for Hunt {
    fn default() -> Self {
        Hunt {
            after: String::new(),
            site: String::new(),
            area: "land".into(),
            steps: 200,
            skip: vec![],
            detour_tiles: 0,
            on_route: false,
            stop_after: 0,
            repel_level: 0,
        }
    }
}

/// Wild fights for exp at a fixed point of the route.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Grind {
    pub after: String,
    pub battles: u32,
    pub level: u32,
    pub yield_: u32,
}

impl Default for Grind {
    fn default() -> Self {
        Grind {
            after: String::new(),
            battles: 5,
            level: 20,
            yield_: 70,
        }
    }
}

/// A gift mon or a one-off wild battle (`givemon` / `setwildbattle` in the decomp scripts).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Special {
    pub after: String,
    /// `gift`, `trade`, `buy` or `static`.
    pub kind: String,
    pub species: Vec<String>,
    pub level: u32,
    /// Frames for the whole event (walking out of the way, cutscene, menus), before text_factor.
    pub frames: u32,
    /// Static catches: wear it down to this HP fraction first.
    pub hp_frac: f64,
    pub ball: String,
    pub max_throws: u32,
    /// Species that must already be caught or the event is skipped for free (in-game trades).
    pub requires: Vec<String>,
    /// `buy` events (Celadon Game Corner prizes): money paid, at 20 per coin
    /// (`CeladonCity_GameCorner/scripts.inc:38-61`); skipped when the money model is on and the
    /// money is not there, or when every species is already caught.
    pub cost: u32,
}

impl Default for Special {
    fn default() -> Self {
        Special {
            after: String::new(),
            kind: "gift".into(),
            species: vec![],
            level: 30,
            frames: 600,
            hp_frac: 0.2,
            ball: "ultra".into(),
            max_throws: 30,
            requires: vec![],
            cost: 0,
        }
    }
}

/// One evolution the human performs to bank a dex entry without a catch ("evos", the WR runner's
/// word). `species` is the evolved form obtained; its pre-evolution comes from the species table
/// (`decompiled/src/data/pokemon/evolution.h`) and must already be caught, else the event is skipped
/// for free. `via`:
///
/// * `stone`: the evolution item `item` (`MOON`, `LEAF`, `FIRE`, `THUNDER`, `WATER`). A stone that is
///   found costs nothing; `buy` pays the Celadon price (`CeladonCity_DepartmentStore_4F/scripts.inc:31-34`,
///   `items.json`, 2100) and the event is skipped when the money is not there.
/// * `candy`: Rare Candies from the stock the pickups fill: one candy per level the mon is short of its
///   evolution level, at least one (a mon caught above its level evolves on its next level-up).
/// * `level`: the pre-evolution takes part in a fight (lead swap) and levels past its evolution level:
///   frames only (a guess; the WR's bug chains, `wr-3.25.25-main97.md` 2:05-2:07).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Evolve {
    pub after: String,
    pub species: String,
    pub via: String,
    pub item: String,
    pub buy: bool,
    /// Candies for `candy` (0 = derive from the catch level).
    pub candies: u32,
    /// Scene frames before `text_factor` (0 = the default of the kind).
    pub frames: u32,
}

impl Default for Evolve {
    fn default() -> Self {
        Evolve {
            after: String::new(),
            species: String::new(),
            via: "stone".into(),
            item: String::new(),
            buy: false,
            candies: 0,
            frames: 0,
        }
    }
}

/// A TM, HM or tutor move taught to the carry (or Mewtwo) after the named segment.
/// `frames` is the whole event before `text_factor` (walking to the tutor, menus);
/// `cost` is money when the TM is bought (TM28 Dig is also a free pickup in Cerulean,
/// `decompiled/data/maps/CeruleanCity/scripts.inc:179-187`; the Celadon mart sells it for
/// 2000, `CeladonCity_DepartmentStore_2F/scripts.inc:57`, `items.json`).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Teach {
    pub after: String,
    pub mv: String,
    pub frames: u32,
    pub cost: u32,
    /// Teach Mewtwo instead of the carry (post-catch).
    pub mewtwo: bool,
}

impl Default for Teach {
    fn default() -> Self {
        Teach {
            after: String::new(),
            mv: String::new(),
            frames: 800,
            cost: 0,
            mewtwo: false,
        }
    }
}

/// An Elixir-class pickup after the named segment (`frames` covers the detour), used
/// between trainer fights when attacking PP runs low. On-route item balls, decomp-cited:
/// Elixir in Pokémon Mansion B1F and Silph Co 5F, Max Elixir + Max Ether on Route 23,
/// Max Elixir on Route 17 and in the Power Plant, Elixir + Ether on Route 25 (each map's
/// `map.json`).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ElixirPickup {
    pub after: String,
    /// "elixir" (10 PP per move), "max" (all PP), or "candy" (a Rare Candy, eaten just
    /// before the League: on-route balls in Cerulean City, Route 6/9/12/17 and
    /// Mansion 3F, each map's `map.json`).
    pub kind: String,
    pub frames: u32,
}

impl Default for ElixirPickup {
    fn default() -> Self {
        ElixirPickup {
            after: String::new(),
            kind: "elixir".into(),
            frames: 400,
        }
    }
}

/// Battle items bought after the named segment (Celadon Dept. 5F sells the X line,
/// `decompiled/data/maps/CeladonCity_DepartmentStore_5F/scripts.inc:29-35`; prices
/// `items.json`: X Attack 500, X Defend 550, X Speed 350, X Special 350, Dire Hit 650).
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct XBuy {
    pub after: String,
    pub attack: u32,
    pub defend: u32,
    pub speed: u32,
    pub special: u32,
    pub dire: u32,
    /// X Accuracy (950, `items.json`; the WR bag holds 2-3) and Guard Spec (700; the WR bag holds 2).
    pub accuracy: u32,
    pub guard: u32,
    /// Revives (1500 each, `items.json`): bring sacrificed teammates back between
    /// League fights.
    pub revives: u32,
    /// Hyper Potions for the League (1200 each, `items.json`; Saffron Mart sells them,
    /// `data/maps/SaffronCity_Mart/scripts.inc`); each is one heal use in a League fight.
    pub heals: u32,
    pub frames: u32,
}

/// Optional trainers the route fights beyond the cone-forced ones (the WR run buys experience: Josh
/// in Mt. Moon, the Route 3 bug catchers; `STRATEGY.md` 4): a fight segment inserted after `after`.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ExtraFight {
    pub name: String,
    pub after: String,
    /// Event-script labels of the trainers (`decompiled/data/maps/*/scripts.inc`).
    pub scripts: Vec<String>,
}

/// A conditional skip (`Params::cond_skips`): when the attempt reaches one of `segs` and its clock at the
/// most recent WR split it has passed (`wrsplits::WR_SPLITS`) is more than `behind` seconds past the WR's
/// time at that split, the segments are not played (no frames, no catches, no items). The human's
/// emergency lever when behind pace (`STRATEGY.md` 5.3, "skip Plants and Pewter"; T09 4:53:31); a segment
/// name the route does not contain is ignored.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CondSkip {
    pub segs: Vec<String>,
    pub behind: f64,
}

/// One window of the extended risk lane (`Params::risk_windows`): the lane fights after the previous
/// window's `until` segment, up to and including this one, use `power` as the carry's Atk/SpA factor
/// and `foe` for the foes' (0 = the global value). Fitted to the runner's windowed pass rate.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct RiskWindow {
    pub until: String,
    pub power: f64,
    pub foe: f64,
    /// Potions per fight in the window (None = `risk_potions`); a trainer group is one fight.
    pub potions: Option<u32>,
}

/// Per-fight battle tactics, keyed by segment name in `Params::fight_tactics`.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FightTactic {
    /// X items used on the opening turns, in order: `attack`, `defend`, `speed`,
    /// `special`, `dire` (limited by the bag).
    pub x: Vec<String>,
    /// Heal when HP falls below this fraction (overrides the global `heal_below`).
    pub heal_below: f64,
    /// Expendable teammates sent out to eat a hit while the fighter sets up.
    pub sacs: u32,
    /// Torrent tactic in this fight: Withdraw when its Defense stage saves a would-be KO, stall
    /// turns while above 1/3 HP (`battle.rs` `torrent_action`).
    pub torrent: bool,
    /// Before this fight, a potion heal only up to the Torrent window: heal when the HP is below the
    /// foes' best hit (with Withdraw) plus one, otherwise leave the HP alone (never heal up to full).
    pub entry: bool,
    /// Multiplier on the carry's fitted `carry_power` in this fight (1 = as fitted): the fit is one number
    /// over 16 fights, a fight whose action count it misses can be nudged (Misty: 4.4 actions against
    /// the video's 6-8).
    pub power: f64,
    /// Heal items (potions) the risk lane may spend in this fight, entry heal included (-1 = the
    /// global `risk_potions`; 0 = "never heal in Brock").
    pub potions: i32,
    /// This fight's risk-lane `risk_power` / `risk_foe` (0 = the global value). The rivals' and
    /// leaders' fights carry the human's own skill (switching, sacrifices, X items) that the timing
    /// lane's `carry_power` stands in for; their risk lane runs at that same stand-in.
    pub risk_power: f64,
    pub risk_foe: f64,
    /// The carry's Torrent boost is on for the whole fight (timing lane only): the WR enters these fights at
    /// or under a third of its HP (Koga 33/120, Sabrina 43/135, Giovanni 35/143, Bruno 48/156, Agatha 51/159:
    /// `human-run/video-analysis/data/calib/v2-mid-fights.md`, `v3-e4r1.md`), the model's carry at full HP.
    pub free_torrent: bool,
}

impl Default for FightTactic {
    fn default() -> Self {
        FightTactic {
            x: vec![],
            heal_below: -1.0,
            sacs: 0,
            torrent: false,
            entry: false,
            power: 1.0,
            potions: -1,
            risk_power: 0.0,
            risk_foe: 0.0,
            free_torrent: false,
        }
    }
}

/// A stop at a Poké Mart after the named segment, buying balls with the money on hand
/// (money model only): up to `poke`, `great`, `ultra` balls in that order, if the mart on that leg
/// of the route sells them (`LavenderTown_Mart` Great from Lavender, `FuchsiaCity_Mart` Ultra from
/// Fuchsia). With any visit listed the on-demand buying of `money_model` is off.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Mart {
    pub after: String,
    pub poke: u32,
    pub great: u32,
    pub ultra: u32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Params {
    pub name: String,
    pub starter: String,
    /// Route order: `old` (the original: Surge before Rock Tunnel, dex fill after the first Hall of
    /// Fame) or `wr` (the world record's order, all 60 catches before the League:
    /// `human-run/video-analysis/wr-3.25.25-main97.md`, `route_wr.rs`).
    pub route_style: String,
    /// `FR` or `LG`: selects the wild tables (trainers are identical, `docs/glitchless-run/plan.md` §5).
    pub version: String,
    /// Reset the starter unless the sum of its six level-5 stats reaches this.
    pub starter_min_total: u32,
    /// Instead of restarting from New Game, save in Oak's lab before choosing and soft-reset to
    /// Continue for every bad starter. The run timer keeps going (an assumption about the rules:
    /// the category starts the timer at New Game), each re-roll costs `reroll_frames`.
    pub starter_save_reroll: bool,
    /// Boot, logo, title, Continue, and choosing the ball again (guess: 646 boot + ~400 load +
    /// ~900 choose, before `text_factor`).
    pub reroll_frames: u32,
    /// Per-stat minimums at level 5, in the order HP, Atk, Def, Spe, SpA, SpD (0 = no check).
    pub starter_min_stats: [u32; 6],
    /// Each mashed input is late by U{0..jitter_n} frames.
    pub jitter_n: u32,
    /// Reaction delay after an unpredictable event: U{react_lo..react_hi} frames.
    pub react_lo: u32,
    pub react_hi: u32,
    /// Human slowdown on text/menu frames relative to the frame-perfect floor.
    pub text_factor: f64,
    /// Human slowdown on walking relative to a straight tile count.
    pub path_overhead: f64,
    pub bike: bool,
    pub teleport_abra: bool,
    /// Split-reset aggressiveness: 0 resets only when even the best case cannot make the
    /// target, 1 resets whenever the mean remaining time cannot.
    pub reset_aggr: f64,
    /// Per-split override of `reset_aggr`, by segment name.
    pub reset_aggr_at: std::collections::HashMap<String, f64>,
    /// Optimal-stopping reset table (`optstop.rs`): per rung name, per split, the run-clock seconds
    /// above which the human resets at that split. A split without an entry keeps the
    /// `reset_aggr` formula; the best-case floor test always applies.
    pub reset_table: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
    /// Second variable of the reset rule (`optstop.rs`): per rung name, per split,
    /// `[kind, reference, seconds per unit]` with kind 1 = species caught, 2 = lead level; the
    /// split's cutoff becomes `reset_table + seconds * (x - reference)`.
    pub reset_adj:
        std::collections::HashMap<String, std::collections::HashMap<String, (u8, f64, f64)>>,
    /// More segments after which the human checks the reset rule (the badge splits, the Hall of
    /// Fame and Oak's check are always splits).
    pub extra_splits: Vec<String>,
    /// Reset estimates before Oak's check use the species caught so far: the fill still to come is a
    /// fitted `base + slope * (60 - species)` from the pilot, instead of the average fill. At Oak's
    /// check the remaining fill is added too.
    pub dex_resets: bool,
    /// Fight with the turn-level simulation of `battle.rs` (decomp damage formula, stats, moves, the
    /// trainers' real mons) instead of the level-ratio shortcuts; `type_floor`, `team_bonus`,
    /// `type_strength` and the logistic loss curve are then unused.
    pub battle_sim: bool,
    /// Experimental overrides of uncalibrated model constants by name (sensitivity studies):
    /// `level_up`, `trainer_outro`, `whiteout`, `p_event`, `p_lose_center`, `p_lose_slope`,
    /// `weaken_ko`, `weaken_hp`.
    pub tune: std::collections::HashMap<String, f64>,
    /// `poke`, `great` or `ultra`.
    pub ball: String,
    pub weaken: bool,
    /// Catch new Kanto species met while walking (otherwise flee everything).
    pub catch_on_pass: bool,
    /// While passing (not in a dedicated hunt) only throw at species with at least this catch
    /// rate (`species_info.h` `catchRate`).
    pub pass_min_catch: u32,
    /// Hunts leave species below this catch rate alone (and do not wait for them).
    pub hunt_min_catch: u32,
    /// Fill hunts ride the bike where the map allows it (`allow_cycling`).
    pub hunt_bike: bool,
    /// Model the Safari Zone's own rules for hunts in it: 600 steps and 30 Safari Balls per
    /// entry, the Safari catch factor and the species' flee rate
    /// (`decompiled/src/battle_main.c:2284-2287`, `battle_script_commands.c:9497`,
    /// `battle_ai_script_commands.c:1715-1732`).
    pub safari_model: bool,
    /// Rocks thrown before the first Safari Ball (each doubles the catch factor and the flee rate).
    pub safari_rocks: u32,
    /// Track prize money, ball stock and prices: balls are bought in batches as marts (and money)
    /// allow instead of being free and unlimited. Prices `items.json`, prize money
    /// `battle_script_commands.c:5373`.
    pub money_model: bool,
    pub marts: Vec<Mart>,
    /// Wild-encounter frame costs measured on libmgba (`human-run/calibration.md`) instead of the
    /// original guesses.
    pub calib_enc: bool,
    /// Grass steps per walking segment from the decomp's metatile encounter attribute along the shortest
    /// path (`human-run/tools/pathenc.py`, `human-run/data/pathenc.json`) instead of guesses.
    pub enc_path: bool,
    /// Hunts walk the tiles from the nearest Fly point to the site (`flydist.py`) instead of the plan's
    /// guessed `detour_tiles` (the table wins; sites it lacks keep their guess).
    pub detour_map: bool,
    /// The walk to a hunting site is by bike (6 frames per tile instead of the run's 8).
    pub detour_bike: bool,
    /// With `enc_path`: walk the path that pays up to 50 tiles per grass tile (more tiles, fewer
    /// encounters) instead of the shortest.
    pub avoid_grass: bool,
    /// Battle turn, per-mon and trainer-intro frames measured on libmgba (`calibration.md`)
    /// instead of `battle_scale` times the guessed constants.
    pub calib_battle: bool,
    /// Battle frame constants fitted to the world-record video's measured fight durations
    /// (`human-run/video-analysis/data/calib/v1..v4`; overrides `calib_battle`): the human factor
    /// is already in them, `text_factor` is not applied again.
    pub calib_wr: bool,
    /// Fidelity audit 2026-11-14 (`human-run/journal/2026-11-14-18-00-fidelity-audit.md`): walking,
    /// doors, speeches and bag trips between Elite Four rooms, which the fight constants (fitted to
    /// the video's battle-to-battle durations) do not contain. Frames per room are the WR's split
    /// window minus the WR's fight duration (`wr-3.25.25-main97.md`, `wrsplits.rs`). `route_style = "wr"`
    /// only; 0 = off.
    pub league_transit: bool,
    /// Frames taken off every `league_transit` segment: the constants were the WR split window minus
    /// the WR fight duration (intro to the defeated box), but the model charges the defeat text
    /// (`outro`, 240 frames) after each trainer fight on top, so the room-to-room walk was counted
    /// with it twice (R2 chain model windows +14 s each). 240 = one outro. 0 = off.
    pub transit_outro: u32,
    /// Same audit: extra death probability per named segment (trainer fights and groups), for the
    /// losses the fight engine's tidy human does not have: the runner's deaths list of the
    /// 2026-11-12 stream (brock x5, sammy x2, liam, rival 1, rival 2 in 12 attempts past the
    /// starter gate, `stream-2026-11-12-main97.md`) and "Brock-split win 75% on a good day, 2/12 and
    /// 3/14 on bad days" (`STRATEGY.md` 4, 7). guess for every value; empty = off.
    pub hazard: std::collections::HashMap<String, f64>,
    /// Same audit: multiplier on the fitted trainer intro and per-enemy-mon overhead for every
    /// battle that is not an Elite Four / Champion fight (0 or 1 = off). The model's eight
    /// non-League fights of the WR video run 727 s against the video's 596 s (1.22x) with the
    /// League fights at 1.0x (`frlg-human splits`, fight table); 0.71 closes that gap.
    pub trainer_fit: f64,
    /// Same audit: the post-Hall-of-Fame chain (Mt. Ember, Icefall Cave, Five Island, the Warehouse
    /// interior, the Celio hand-in) costs 342 s more than the guessed frames of `route_wr.rs`, the
    /// difference between the WR's Champ 1 -> Rocket Warehouse splits (18:58) and the model's
    /// (13:15), spread over the scenes the WR scene log places there.
    pub postgame_calib: bool,
    /// R2 fidelity (journal 2026-11-15): the post-game chain's events from the decomp instead of
    /// text lumps: the Mt. Ember cave door's two Rocket grunts (GRUNT_43 / 44), Kindle Road and the
    /// Ruby Path at their `frlg-mapgraph` tile counts, the Icefall Cave and Five Island walks at their
    /// tile counts, and the Meadow's forced grunt (GRUNT_50). With `postgame_calib` the text lumps
    /// `mt-ember-ruby-climb` and `five-island-resort` shrink to `ruby_text` / `resort_text` frames so
    /// the Icefall and Warehouse split windows keep the WR's times.
    pub postgame_events: bool,
    /// `postgame_events` + `postgame_calib`: frames of the Ruby-at-Celio text lump left after the events.
    pub ruby_text: u32,
    /// Same for the Five Island lump.
    pub resort_text: u32,
    /// Same audit: Persim Berry (Misty), Black Glasses swaps (Lavender rival, Lorelei) and the
    /// Mark-guy / potion shopping menus (`STRATEGY.md` sections 4, 6): bag trips in frames.
    pub item_swaps: bool,
    /// `item_swaps`: the Persim Berry is fetched for Misty (default). false skips the berry trip,
    /// saving `persim_frames` of the 810-frame `persim-and-potions` bag segment; the cost (confusion
    /// self-hits, extra potions) is a plan choice in `fight_tactics.misty` (stream 2026-11-14 A1:
    /// no Persim, potions 7 -> 1, "really bad Misty fight"). guess: 300 frames.
    pub persim: bool,
    pub persim_frames: u32,
    /// Same audit: probability that one player action in a battle is a menu slip (wrong move row,
    /// Right+A on the same frame, a late A): the turn goes to another move of the set and the foe
    /// acts as usual. guess; `STRATEGY.md` section 4 (T10 2:49:21 "kick-select input errors nearly
    /// killed runs"), section 1.3 (late-day errors, T03/T10). 0 = off.
    pub menu_slip: f64,
    /// Value factor of a scarce-PP move (Blizzard / Fire Blast, base PP <= 5) in the player's move choice unless it
    /// kills. 0.6 rations Blizzard for Lance; the WR video and the 2026-11-14 stream Blizzard Gyarados at once
    /// (`video-analysis/data/calib/league-*.json`), 1.0 = no rationing.
    pub scarce_factor: f64,
    /// Fidelity fit 2026-11-14 (`human-run/journal/2026-11-14-22-00-fidelity-fit.md`): per segment
    /// probability that the attempt ends there for good (a death the human does not salvage:
    /// a whiteout in the League sends him back to the first room, one in a mid-game fight loses
    /// the pace and the items). Fitted to the runner's observed attempts
    /// (`video-analysis/data/accepted-runs.json`, `tools/fit_attempts.py`); applies only in
    /// evaluation runs (a rung is set), not in the no-reset pilot or `splits`/`dist`.
    /// Segments absent from the map have none. Empty = off.
    pub fatal: std::collections::HashMap<String, f64>,
    /// Same fit: execution-loss events (a run of missed Mega Kicks and the special-move fallback,
    /// a bad Safari, Power Plant duplicates, extra cave encounters, ...) that cost `Exp(mean_s)`
    /// seconds with probability `p` when the named segment completes. Costs from the runner's own
    /// quotes (`STRATEGY.md` section 8), probabilities mostly guesses (each entry says which).
    pub slow: Vec<SlowEvent>,
    /// Multiplier on every `slow` probability (sensitivity knob, 1 = the plan's values).
    pub slow_scale: f64,
    /// Same fit: extra execution noise on every WR split window (the stretch between two LiveSplit
    /// splits, `wrsplits::WR_SPLITS`): the window's elapsed time is scaled by a mean-preserving
    /// lognormal with this sigma, on top of whatever the window's own events draw. The runner's
    /// windows vary 7.3% rms (deep run against the LG PB, 13 windows) against the model's 6.5%;
    /// the model's own Icefall, Warehouse and Bike windows are near-deterministic (1-4%).
    /// 0 = off.
    pub win_cv: f64,
    /// R2 fidelity (journal 2026-11-15): frames Mewtwo's fights wait for its own HP bar per full bar of
    /// HP moved (damage taken plus items healed). The bar slides 1/384 of its length per frame
    /// (`decompiled/src/battle_interface.c:1844-1845,1955-2012`, one `MoveBattleBar` call per frame,
    /// `battle_controller_player.c:999-1013`): 384 reproduces the game. Applied to the lone-Mewtwo
    /// R2 fights only (the fitted `turn` already averages the carry's bar in the others). 0 = off.
    pub mewtwo_hp_bar: f64,
    /// R2 fidelity: extra frames for each in-fight heal item turn (Hyper Potion / Full Restore) of a
    /// Mewtwo fight, on top of `turn`: the bag has to be scrolled to the item and the party screen
    /// opened. Fitted to one observation: Champion 2's Arcanine window runs 32 s for two actions with a
    /// heal turn against ~22 s for two plain actions (`video-analysis/data/calib/v4-e4r2.md`, 4 s
    /// sampling, so +-4 s). 0 = off.
    pub mewtwo_heal_extra: f64,
    /// R2 fidelity: Mewtwo gains exp from every R2 foe (Slow growth, `GROWTH_SLOW`
    /// `decompiled/src/data/pokemon/species_info.h:4381`, `EXP_SLOW(n) = 5 n^3 / 4`
    /// `experience_tables.h:4`), so it levels 70 -> 73 across the round as in the video
    /// (`human-run/video-analysis/data/calib/v4-e4r2.md`): each level-up costs `level_up()` frames and
    /// its stats follow the level.
    pub mewtwo_levels: bool,
    /// Mewtwo's four slots carry their PP between fights (`Cmd_ppreduce`; refilled at a Centre, a
    /// whiteout, an Elixir / Max Elixir before a trainer fight). Off = full PP every fight (old model).
    pub mewtwo_pp: bool,
    /// With `mewtwo_pp`: top Mewtwo up before a League fight when its Psychic PP is below this (0 = only
    /// when a slot is empty or Fire Blast is below 3, the carry's rule).
    pub mewtwo_pp_floor: i32,
    /// Mechanics model (journal 2026-11-16): `MonGainEVs` stops at 510 total EVs (255 per stat);
    /// `decompiled/src/pokemon.c:5512-5595`. Off = per-stat cap only (the old model).
    pub ev_cap: bool,
    /// Mechanics model: a mon's stats change only when `CalculateMonStats` runs (level-up, Rare Candy,
    /// evolution), not when `MonGainEVs` writes the EV bytes (`pokemon.c:5591-5593` only `SetMonData`s the
    /// EV; stats are cached). The carry and Mewtwo fight with the EVs they had at their last level-up.
    pub ev_lag: bool,
    /// Mechanics model: Mewtwo gains EVs from every R2 foe it defeats (`MonGainEVs`, the foe's `evYield_*`).
    pub mewtwo_evs: bool,
    /// Mechanics model: the carry holds Black Glasses from the equip swap on (`black-glasses-swap`):
    /// Dark moves (Bite) use `spAttack * 110 / 100` (`HOLD_EFFECT_DARK_POWER`, `pokemon.c:1468`, param 10
    /// in `items.json`).
    pub black_glasses: bool,
    /// Mechanics model: in League fights the human plays the carry inside Torrent (<= 1/3 HP: Water moves
    /// x1.5, `CalculateBaseDamage`) like the WR runner (Lorelei/Bruno/Agatha at 28-54 of 153-162 HP in
    /// `v3-e4r1.md`): no pre-fight heal while the carry sits in the window the next foes' best hit allows,
    /// and the in-fight Torrent tactic (Withdraw, stall turns) is on.
    pub league_torrent: bool,
    /// Mechanics model: after this segment the human buys a Mystic Water at the Celadon Game Corner prize
    /// counter (1,000 coins, `src/strings.c:537`; 50 coins cost 1,000 yen, `strings.c:425`, so
    /// `mystic_cost` = 20,000 yen) and the carry holds it instead of the Black Glasses: Water moves use
    /// `spAttack * 110 / 100` (`HOLD_EFFECT_WATER_POWER`, `pokemon.c:1471`). Empty = never bought.
    pub mystic_after: String,
    pub mystic_cost: u32,
    /// Frames of the prize-counter trip (walk, exchange text, equip).
    pub mystic_frames: u32,
    /// Mewtwo's moves that a TM teach never replaces (default Psychic and Recover, the old model).
    /// The WR's Mewtwo runs Psychic / Fire Blast / Safeguard / Earthquake (`v4-e4r2.md`), i.e. it keeps
    /// Safeguard and loses Recover.
    pub mewtwo_keep: Vec<String>,
    /// Which rod the fish hunts use: `mixed` (the old model: all ten slots), `old`, `good` or
    /// `super`. A real rod costs a trip: Old Rod Vermilion, Good Rod Fuchsia, Super Rod Route 12
    /// south of the Snorlax (`decompiled/data/maps/*House*/scripts.inc`).
    pub rod: String,
    /// Segments (by name) where Repels are used on the grass once a mart sells them (Pewter: Repel 100 steps, Lavender: Super
    /// 200, Fuchsia: Max 250; `items.json`, `wild_encounter.c:286,601`). A repel blocks every
    /// encounter whose level is below the lead's.
    pub repel_on: Vec<String>,
    /// Balls bought per mart visit.
    pub ball_batch: u32,
    /// Throw bait first at species whose escape factor (`safariZoneFleeRate * 100 / 1275`, at
    /// least 2) is at least this (0 = never). Bait halves the catch factor (min 3) and quarters
    /// the flee factor for 2-6 turns.
    pub safari_bait_from: u32,
    /// Fight (instead of fleeing) wild encounters that are not catch targets.
    pub fight_wild: bool,
    /// Fight non-target wild encounters while the carry is below this level (else flee).
    pub fight_until_level: u32,
    /// Effective-level multiplier of a full team over the lone carry (guess).
    pub team_bonus: f64,
    pub hunts: Vec<Hunt>,
    pub specials: Vec<Special>,
    /// Evolutions as dex currency (stones, candies, lead-swap levels), see `Evolve`.
    pub evolutions: Vec<Evolve>,
    /// Safari: leave the zone for good once this many new species were caught in it (0 = use the
    /// whole 600 steps / 30 balls; the WR route plans 7-10 and leaves,
    /// `video-analysis/STRATEGY.md` 5.3).
    pub safari_stop: u32,
    /// Ball rule under the money model, by the species' catch rate (`species_info.h`): at least
    /// `poke_from` is thrown a Poke Ball, below `ultra_below` an Ultra Ball, else a Great Ball
    /// (0 / 0 = the plan's single `ball`).
    pub poke_from: u32,
    pub ultra_below: u32,
    /// Drop the Power Plant detour (fly to Lavender, surf to the plant and back) and every hunt
    /// anchored to it (`route_wr.rs`).
    pub skip_powerplant: bool,
    /// Frames of one PC visit (withdraw / deposit between a catch and the evolution or trade that
    /// needs it): charged once per anchor when the mon was caught after the party was full (0 = free,
    /// the old model).
    pub pc_frames: u32,
    /// Segments (by name) where a passing wild encounter is never caught even with `catch_on_pass`
    /// (the WR runner's "never catch anything on Route 1", `STRATEGY.md` 4).
    pub pass_off: Vec<String>,
    /// `Evolve` via `level`: the pre-evolution must be within this many levels of its evolution level
    /// when caught (it levels by taking part in fights; a guess).
    pub level_gap: u32,
    pub grinds: Vec<Grind>,
    /// TM / tutor teaches (battle_sim).
    pub tms: Vec<Teach>,
    /// X-item purchases (battle_sim; money model pays for them).
    pub xbuys: Vec<XBuy>,
    /// Elixir pickups (battle_sim).
    pub elixirs: Vec<ElixirPickup>,
    /// Per-fight tactics by segment name (battle_sim).
    pub fight_tactics: std::collections::HashMap<String, FightTactic>,
    /// Global heal threshold in battle: use an item below this HP fraction.
    pub heal_below: f64,
    /// Rare Candies picked up on the route are eaten on the spot instead of kept for the League
    /// (the WR run is Lv20/27/41 at Misty/SS Anne/Koga but only Lv50 at the League:
    /// `human-run/video-analysis/data/calib`).
    pub candy_early: bool,
    /// With `candy_early`, only the first this-many candies are eaten on the spot; the rest are kept
    /// for the League.
    pub candy_early_n: u32,
    /// Multiplier on the starter-line carry's Attack and Special Attack in `battle_sim` fights.
    /// The exact engine needs 1.5-4x the WR video's player actions in 10 of its 16 measured fights
    /// (`frlg-human splits` fight table); the WR carry has a 30-IV SpA and a Rash nature, 6-12
    /// X Specials per big fight sequence and plays at 1/3 HP for Torrent, none of which the engine's
    /// carry reproduces. 1.0 = the engine as audited; a value fitted to the video is a stand-in, not a
    /// mechanism.
    pub carry_power: f64,
    /// The same stand-in for the caught Mewtwo (R2 fights: the engine needs ~1.5x the WR video's
    /// actions with Mewtwo; no Torrent there, so this is the residual the Rash/IV/X lines do not explain).
    pub mewtwo_power: f64,
    /// `carry_power` for League fights only (0 = same as `carry_power`). The League is where the
    /// model's lone carry most visibly falls short of the WR run's (EVs, held type boosters, X lines
    /// the engine does not play); fitted to the video's League action counts, see the journal.
    pub league_power: f64,
    /// Max Elixirs a non-League fight leaves in the bag for the League ("Max Elixir between E4
    /// fights", `human-run/video-analysis/STRATEGY.md`; the exact engine's Blizzard (5 PP) is empty
    /// by Lance otherwise).
    pub league_max_reserve: u32,
    /// Outside the League the last `league_elixir_reserve` plain Elixirs also stay in the bag (the WR
    /// carries an Elixir and a Max Elixir into Round 2, `video-analysis/data/calib/v4-e4r2.md`). Only
    /// matters with `mewtwo_pp`; 0 = the old rule.
    pub league_elixir_reserve: u32,
    /// Heal items the bag holds at the start of each League round: `xbuys[].heals` (Hyper Potions)
    /// count first, the Indigo mart tops up to this many at `league_heal_price` while the money
    /// lasts (the WR run: "two heals + a Max Elixir between E4 fights; Full Restores sparingly",
    /// `STRATEGY.md`; the bag contents themselves are a guess).
    pub league_heals: u32,
    /// Price of a heal topped up at the Indigo Plateau mart at a round's start: Max Potion 2500
    /// (`IndigoPlateau_PokemonCenter_1F_Items`, `items.json`; Full Restore is 3000).
    pub league_heal_price: u32,
    /// Between League fights the human spends one heal item when the fighter is below this
    /// HP fraction (bag trip, `ctx.turn()` frames).
    pub league_preheal: f64,
    /// Lookahead rollouts per candidate action in contested League fights (0 = the greedy
    /// human); see `battle::fight`.
    pub league_rollouts: u32,
    /// An X item is only used against a foe the fighter's best attack needs this many hits to drop
    /// (1 = the old "always set up"); a runner does not spend a turn on an X item before a KO.
    pub setup_min_turns: u32,
    /// Before a fight, heal with items when the carry is below this HP fraction (up to 0.85). The WR
    /// run walks into most fights at 1/3 HP or less to keep Torrent active
    /// (`human-run/video-analysis/data/calib/v2-mid-fights.md`: Koga 33/120, Giovanni 35/143, ...).
    pub preheal_below: f64,
    /// Moves the carry never forgets when a new one needs a slot.
    pub keep_moves: Vec<String>,
    /// Cure a status that outlives a battle with the matching item (frames + money).
    pub cure_status: bool,
    /// Fetch TM28 Dig on the first Cerulean visit, before the rival (the Rocket is
    /// there from the start: `data/maps/CeruleanCity/map.json:67`'s hide flag is never
    /// set anywhere in the decomp).
    pub early_dig: bool,
    /// Skip the Cerulean Rocket and TM28 Dig altogether (the WR run does not use Dig).
    pub no_dig: bool,
    /// Fight Erika after the Pokémon Tower leg (fly-celadon lands beside the gym): the
    /// carry is ~2 levels higher and usually evolved.
    pub erika_after_tower: bool,
    /// Structural route-order variants of the WR order (`route_wr.rs`), by name: `warden_early`,
    /// `erika_cycling`, `erika_pre_silph`, `surge_early`, `safari_after_koga`, ... Empty = the WR
    /// run's own order.
    pub route_variants: Vec<String>,
    /// Reset unless the carry's stats at its first evolution reach these minima
    /// (HP, Atk, Def, Spe, SpA, SpD; 0 = no check). A later check resolves IVs far finer
    /// than level 5 can: Wartortle at 16 has SpA 25-30 across IVs, so "SpA >= 30" is a
    /// top-decile gate — the WR's reported rule.
    pub evo_min_stats: [u32; 6],
    /// Pokémon Center stops inserted after the named segments (restore HP, PP, status
    /// and fodder). Each costs `extra_heal_frames` (walking in, the nurse's text,
    /// walking out; guess) before `text_factor`.
    pub extra_heals: Vec<String>,
    pub extra_heal_frames: u32,
    /// Escape-Rope out of dungeon fill sites instead of walking back (rope 550,
    /// `items.json`; one use per entry).
    pub rope_fills: bool,
    /// Escape-Rope out of Cerulean Cave after the Mewtwo catch (the walk segment
    /// otherwise pays the way back).
    pub rope_cerulean_cave: bool,
    /// Fallback hunts, tried in order when Oak's check finds fewer than 60 caught.
    pub fill: Vec<Hunt>,
    pub stones: Vec<String>,
    /// Clemi skip (docs/tricks/clemi-skip.md): an L press during the caught jingle opens the
    /// Help menu, which kills the SE the "Gotcha!" text waits on. Measured on tier 1
    /// (frlg-calib --clemi, 2026-11-13): good presses save 170-230 floor frames per successful
    /// catch; the model credits a conservative 140 (late presses save less, misses save none).
    pub clemi_skip: bool,
    /// Trainer groups the human walks around (by segment name).
    pub skip_groups: Vec<String>,
    /// Trainer count override per aggregated group (the others in the group are walked around).
    pub group_n: std::collections::HashMap<String, u32>,
    pub mewtwo: bool,
    /// Extra wild battles for exp before the first Elite Four / before the rematch.
    pub grind_pre_e4: u32,
    pub grind_pre_rematch: u32,
    /// Level and base exp yield of what the human grinds on.
    pub grind_level: u32,
    pub grind_yield: u32,
    /// Effective-level multiplier for a caught Mewtwo (guess).
    pub power_bonus: f64,
    /// Scales battle turn and per-mon overhead frames (sensitivity knob; 1.0 is the guess).
    pub battle_scale: f64,
    /// Human mistakes (wrong menu slot, wall bump, missed turn): events per 100 tiles walked
    /// and per 100 menu inputs, each costing U{60..300} frames. 0 = the v1 model without them.
    pub mistake_rate: f64,
    /// Weight of the type-matchup model on trainer battles (0 = off: every trainer is type
    /// neutral except the hand-set Brock factor; 1 = full). guess.
    pub type_strength: f64,
    /// Torrent/Blaze/Overgrow: effective-level bonus in close fights while the fighter is the
    /// starter line (the 1.5x STAB boost at <= 1/3 HP). guess.
    pub ability_bonus: f64,
    /// Lowest matchup multiplier: the human swaps in a covering team mon rather than fight a
    /// 4x-resisted matchup with the carry. guess.
    pub type_floor: f64,
    /// Use the sight-cone audit (`human-run/tools/forced.py`) for which trainers the route cannot
    /// walk around, instead of the earlier guessed groups.
    pub audited_forced: bool,
    /// Second audit (`human-run/tools/silph_audit.py`, `forced-audit.md` round 2): Silph Co. card-key
    /// doors and the Vermilion gym barrier modelled. 0 = off (earlier plans: `skip_groups` decides
    /// the four optional groups). 1 = the cone-forced trainers are fought whatever `skip_groups`
    /// says (Silph 11F Grunt2, Vermilion gym Tucker) and the door / quiz texts are charged.
    /// 2 = pessimistic: rotating trainers on the path count too (Silph 5F Grunt1, one of the
    /// Vermilion gym's two LOOK_AROUND trainers).
    pub audit2: u32,
    /// `random` (a random starter per attempt, gated by `starter_min_*`) or `manip` (frame-exact
    /// presses with a table, `manip.rs`; parameters in `manip`).
    pub starter_model: String,
    pub manip: crate::manip::ManipParams,
    /// Torrent phase, up to and including this segment (empty = off): Pokémon Center stops
    /// (`extra_heals`) are skipped, no pre-fight potions, poison is not cured, the fights use the
    /// Torrent tactic and heal only under a lethal threat (the WR runner skips the PC potion, the
    /// Center and the Mom heal and keeps the starter at <= 1/3 HP through ~Lv17,
    /// `STRATEGY.md` 4).
    pub torrent_until: String,
    /// A whiteout in a fight up to and including this segment ends the attempt (the runner does not
    /// salvage early deaths: `STRATEGY.md` 7). Empty = the old retry-from-the-Center model.
    pub no_salvage_until: String,
    /// Per-segment salvage rule: after a whiteout in a fight of this segment (and its cost) the
    /// human resets when `clock + floor + a * (mean - floor)` still exceeds T, with `a` the value
    /// here (the `reset_aggr` formula); otherwise he plays on. Overrides `no_salvage_until` for
    /// that segment. `salvage_default` (>= 0) applies the same check to every other segment.
    pub salvage_at: std::collections::HashMap<String, f64>,
    pub salvage_default: f64,
    /// A whiteout in the League sends the human back to the first room: > 0 = he walks the rooms
    /// again (`league_redo` times the League clock spent so far, plus the whiteout cost) instead of
    /// resetting. 0 = reset (the old model).
    pub league_redo: f64,
    /// Save-and-reload discipline (what-if; the category rules say only that the timer starts at New Game, so a
    /// reload from an in-game save keeps the timer running): the human saves at the start of each listed
    /// segment (`reload_save_frames`: Start menu, overwrite prompt, "Saving..."), and when an attempt would die
    /// at or after it (a lost League fight, or a flat `fatal` hit, which costs `reload_fatal_frames` of the
    /// dying fight) he soft-resets and loads instead: the clock keeps what has passed plus `reload_frames`
    /// (reset, logos, Continue), the whole game state is the saved one (heal items, X items, money, HP, PP),
    /// the segments replay with fresh draws. At most `reload_max` reloads per attempt, and only while
    /// `clock + floor + reload_aggr * (mean - floor)` of the replay still fits the target.
    pub reload_save_at: Vec<String>,
    pub reload_save_frames: u32,
    pub reload_frames: u32,
    pub reload_fatal_frames: u32,
    pub reload_max: u32,
    pub reload_aggr: f64,
    /// Poison outside battle costs 1 HP per this many frames of walking/text (decomp: 1 HP per 5
    /// steps, `field_control_avatar.c:710-733`; a step is 8-16 frames; guess for the blend). 0 = no
    /// field poison. The human drinks an Antidote when the HP reaches `poison_floor`.
    pub poison_frames_per_hp: f64,
    pub poison_floor: i32,
    /// In the Torrent phase, fights without `entry` start only with HP for this many max-roll hits of
    /// the foes' best move (a potion otherwise); fights with `entry` (the YOLO fights) with one.
    pub torrent_hits: f64,
    /// Stall turns in the Torrent tactic (Withdraw is always allowed).
    pub torrent_stall: bool,
    /// Withdraw against a foe that moves first too (`Tactics::torrent_early`).
    pub torrent_early: bool,
    /// In-battle heal threshold of the Torrent phase (0 = only when a hit could KO).
    pub torrent_heal_below: f64,
    /// `carry_power` up to and including the segment `power_early_until`: the early fights (Brock,
    /// Misty, ...) use the engine's own Torrent and the real manip IVs, so the fitted stand-in is not
    /// needed there (empty = `carry_power` throughout).
    pub carry_power_early: f64,
    pub power_early_until: String,
    pub extra_fights: Vec<ExtraFight>,
    /// Risk lane (plan 930; empty = off): in every fight up to and including this segment a second,
    /// *honest* copy of the fight is played from the lane's own HP (the real HP plus what the
    /// calibrated `carry_power` hid), and a loss there is an unsalvaged death. The timing lane is
    /// untouched. Replaces the flat `fatal` of the segments it covers (remove those entries).
    pub risk_until: String,
    /// The carry's Atk/SpA multiplier in the risk lane (1.0 = the formula's own damage; the timing
    /// lane's `carry_power` 2.0 is a fitted stand-in for the human's play, not a damage claim).
    pub risk_power: f64,
    /// The foes' Atk/SpA multiplier in the risk lane: the one fitted noise knob (crits beyond the
    /// engine's, Rock Tomb rolls, speed ties, misclicks, wild Pikachu: everything the runner's
    /// 40/64/71% pass rates contain that a single fight sim does not).
    pub risk_foe: f64,
    /// Potions per fight (entry heal + in-fight) in the risk lane unless the fight's tactic says
    /// otherwise (`FightTactic::potions`).
    pub risk_potions: u32,
    /// Extra per-fight loss probability in the risk lane, by segment name (documented guesses).
    pub risk_noise: std::collections::HashMap<String, f64>,
    /// Risk lane potions are paid for out of the purse (money model; Potion 300, Super 700, Hyper 1200) and
    /// each use costs a bag turn; a fight can still spend at most its own cap. false = free and instant.
    pub risk_pay: bool,
    /// The carry leaves a risk-lane fight with the lane's HP when the lane drank potions in it
    /// (the heal was real: the timing lane loses the Torrent window it would have kept).
    pub risk_adopt: bool,
    /// Diagnostic: the timing lane's carry has no Torrent (no x1.5, no Torrent tactic); the risk lane keeps it.
    /// Measures what the Torrent state is worth downstream in the model.
    pub ablate_torrent: bool,
    /// Lane windows past `no_salvage_until` (plan 940+), in route order; see [`RiskWindow`].
    pub risk_windows: Vec<RiskWindow>,
    /// Key the lane windows by the fight, not by position: each segment takes the window it has in the
    /// order built with these `route_variants` (the order the windows were fitted on), so a reordering
    /// variant carries every fight's fitted strength with it (None = windows by position).
    pub risk_canon: Option<Vec<String>>,
    /// Do the dex fill after the first Hall of Fame, just before Oak's check, instead of before the
    /// Indigo door (the 60 caught count is only read there: `PalletTown_ProfessorOaksLab/scripts.inc`
    /// `TryStartNationalDexScene`, reached only with `FLAG_SYS_GAME_CLEAR`, i.e. after the Hall of
    /// Fame). Attempts that die in round 1 then never pay for the fill.
    pub fill_after_hof: bool,
    /// Rare Candies held back at the Indigo heal for Mewtwo (a level each, `mewtwo_levels`; 400 frames
    /// per use) instead of going into the carry for round 1.
    pub mewtwo_candies: u32,
    /// Segments skipped at run time when the attempt is behind the WR's pace; see [`CondSkip`].
    pub cond_skips: Vec<CondSkip>,
    /// A lane loss ends the attempt wherever it happens (the flat `fatal` hazards it replaces were
    /// unsalvaged deaths); without it a loss after `no_salvage_until` is an ordinary whiteout.
    pub risk_abandon: bool,
    /// With `risk_adopt`: a lane heal that leaves the carry above 1/3 HP also pauses the Torrent phase
    /// (no Center skips, no Torrent tactic) until damage brings it back inside the window.
    pub risk_torrent_lost: bool,
    /// The mons caught so far are the early fights' sacrifices (`FightTactic::sacs`).
    pub early_helpers: bool,
    /// Outside the League a sacrifice turn is the hit absorbed and nothing else (`Tactics::sac_free`
    /// false): no attack and no X item from behind the helper. Default off keeps older plans' numbers.
    pub sac_honest: bool,
    /// With `sac_honest`, the League's sacrifices are honest too (default off keeps plan 372's calibration
    /// rule there). The WR runner carries four low-level mons through both rounds and never switches
    /// (`data/calib/v3-e4r1.md`, `v4-e4r2.md`: no sacrifices seen).
    pub sac_honest_league: bool,
    /// HP of the typical sacrificial helper (a Lv5-9 mon: ~20-30 HP; a guess). A sacrifice against a foe whose
    /// damage stays under it does not faint the helper and the fighter takes the hit on the switch back
    /// (`Tactics::helper_hp`). 0 = every helper faints.
    pub helper_hp: i32,
    /// MODEL CHANGE (battle bridge, journal 2026-11-22 model-honesty): the policy drinks no potion while
    /// the fighter is asleep unless it outheals the foe's best hit (`Tactics::sleep_no_potion`). Off = the
    /// old policy.
    pub sleep_no_potion: bool,
    /// MODEL CHANGE (same journal): the potion tier the bag really holds. Non-empty: the heal per potion use
    /// outside the League's items is the level tier of `battle::heal_amount` capped by the best tier bought so
    /// far: a Potion (20 HP) from the start, each entry's `hp` from the segment after its `after` mart stop on
    /// (Super Potion 50 first sold at Cerulean, `CeruleanCity_Mart/scripts.inc:26`; Hyper Potion 200 first at
    /// Cinnabar on this route, `CinnabarIsland_Mart/scripts.inc`, or Saffron). Empty = the level tiers alone
    /// (Super Potion from L15 and Hyper from L30 wherever the carry is: the old model).
    pub potion_tiers: Vec<PotionTier>,
    /// MODEL CHANGE (same journal): balls cannot be bought on demand before the segment `ball_shop_from`
    /// (inside Viridian Forest, Route 3, Mt. Moon, Route 4 no mart is on the way). The bag then holds the
    /// `early_balls` Poke Balls bought at the Viridian mart after the parcel (`ViridianCity_Mart/scripts.inc`),
    /// paid out of the purse there. Empty = the old on-demand buying everywhere.
    pub ball_shop_from: String,
    pub early_balls: u32,
    /// PLAN (with the `sleep_no_potion` model): Awakenings bought at the Cerulean mart stop
    /// (`persim-and-potions`), 250 each; the policy wakes a sleeping fighter with one (`Action::Awaken`).
    pub awakenings: u32,
    /// MODEL CHANGE (controller run Giovanni 1 -> Koga, journal 2026-11-22-12-30-controller): Fly needs the
    /// Thunder Badge (`decompiled/src/party_menu.c:3926`, `FlagGet(FLAG_BADGE01_GET + FIELD_MOVE_FLY)`,
    /// `FIELD_MOVE_FLY` 2 in `include/constants/party_menu.h:36`). In the `wr` order without `surge_early`
    /// the Flys to Lavender (Route 16 house -> tower) and to Vermilion (tower -> Surge) come before the
    /// badge: they become bike walks through Saffron (the Tea opens the gates,
    /// `Route7_EastEntrance/scripts.inc:30`): 321 and 242 tiles (`bin/frlg-mapgraph path`).
    pub fly_badge_gate: bool,
    /// MODEL CHANGE (same run): a Game Corner prize (`buy` special, the Abra) needs the Coin Case
    /// (`CeladonCity_GameCorner_PrizeRoom/scripts.inc:15`), a gift in the Celadon restaurant
    /// (`CeladonCity_Restaurant/scripts.inc:19`), and the coins are sold 50 for 1,000 (`strings.c:425`):
    /// the prize's coins are rounded up to whole purchases and these frames are added once (restaurant
    /// detour + clerk menus; guess). 0 = the old model (no Coin Case, coins at exactly 20 each).
    pub coin_case_frames: u32,
    /// MODEL CHANGE (same run): the Vermilion Gym trash-can switches (`SetVermilionTrashCans`,
    /// `decompiled/src/field_specials.c:552`; `VermilionCity_Gym/scripts.inc`: a wrong second can resets
    /// both). The human checks the 15 cans in order until the first switch, then one random neighbour
    /// (+-1 in the row, +-5 in the column); a miss re-rolls. Cost `cans_fixed + checks * can_frames`
    /// after `surge-walk`: 170 frames a check (walk to the next can + "Nope, there's only trash"), the
    /// 53-can played branch's ~9000 frames / 53; fixed 0 (the 13-can branch's 117 s holds controller
    /// overhead a line fit would turn into a 90 s constant, more than the WR's whole window allows).
    pub trash_cans: bool,
    pub can_frames: u32,
    pub cans_fixed: u32,
    /// MODEL CHANGE (same run): frames added at the end of the named segment, measured by the playing
    /// controller against the model (one sample each; `calibration.md`).
    pub played_extra: std::collections::HashMap<String, f64>,
    /// MODEL CHANGE (same run): the Pokemon Tower 6F Marowak ghost is a forced wild battle (Marowak L30,
    /// `PokemonTower_6F/scripts.inc:9`), fought by the sim's engine after the channelers. Off = not modelled.
    pub marowak_fight: bool,
}

/// One step of `Params::potion_tiers`.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PotionTier {
    pub after: String,
    pub hp: i32,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            name: "unnamed".into(),
            starter: "squirtle".into(),
            route_style: "old".into(),
            version: "FR".into(),
            starter_min_total: 0,
            starter_save_reroll: false,
            reroll_frames: 1950,
            starter_min_stats: [0; 6],
            jitter_n: 4,
            react_lo: 15,
            react_hi: 30,
            text_factor: 1.25,
            path_overhead: 1.12,
            bike: true,
            teleport_abra: false,
            reset_aggr: 0.0,
            reset_aggr_at: Default::default(),
            reset_table: Default::default(),
            reset_adj: Default::default(),
            extra_splits: vec![],
            dex_resets: false,
            battle_sim: false,
            tune: Default::default(),
            ball: "poke".into(),
            weaken: false,
            catch_on_pass: true,
            pass_min_catch: 0,
            hunt_min_catch: 0,
            hunt_bike: false,
            safari_model: false,
            safari_rocks: 0,
            money_model: false,
            marts: vec![],
            calib_enc: false,
            enc_path: false,
            detour_map: false,
            detour_bike: false,
            avoid_grass: false,
            calib_battle: false,
            calib_wr: false,
            league_transit: false,
            transit_outro: 0,
            hazard: Default::default(),
            trainer_fit: 0.0,
            postgame_calib: false,
            postgame_events: false,
            ruby_text: 4800,
            resort_text: 6000,
            item_swaps: false,
            persim: true,
            persim_frames: 300,
            menu_slip: 0.0,
            scarce_factor: 0.6,
            fatal: Default::default(),
            slow: vec![],
            slow_scale: 1.0,
            win_cv: 0.0,
            mewtwo_hp_bar: 0.0,
            mewtwo_heal_extra: 0.0,
            mewtwo_levels: false,
            mewtwo_pp: false,
            mewtwo_pp_floor: 0,
            ev_cap: false,
            ev_lag: false,
            mewtwo_evs: false,
            black_glasses: false,
            league_torrent: false,
            mewtwo_keep: vec![],
            mystic_after: String::new(),
            mystic_cost: 20_000,
            mystic_frames: 1800,
            rod: "mixed".into(),
            repel_on: vec![],
            ball_batch: 10,
            safari_bait_from: 0,
            fight_wild: false,
            fight_until_level: 30,
            team_bonus: 1.08,
            hunts: vec![],
            specials: vec![],
            evolutions: vec![],
            safari_stop: 0,
            poke_from: 0,
            ultra_below: 0,
            skip_powerplant: false,
            pc_frames: 0,
            pass_off: vec![],
            level_gap: 12,
            grinds: vec![],
            tms: vec![],
            xbuys: vec![],
            elixirs: vec![],
            fight_tactics: Default::default(),
            heal_below: 0.4,
            preheal_below: 0.5,
            candy_early: false,
            candy_early_n: 99,
            carry_power: 1.0,
            mewtwo_power: 1.0,
            league_power: 0.0,
            league_max_reserve: 2,
            league_elixir_reserve: 0,
            league_heals: 6,
            league_heal_price: 2500,
            league_preheal: 0.7,
            league_rollouts: 0,
            setup_min_turns: 1,
            keep_moves: vec![],
            cure_status: true,
            early_dig: false,
            no_dig: false,
            erika_after_tower: false,
            route_variants: vec![],
            evo_min_stats: [0; 6],
            extra_heals: vec![],
            extra_heal_frames: 700,
            rope_fills: false,
            rope_cerulean_cave: false,
            fill: vec![],
            stones: vec![],
            clemi_skip: false,
            skip_groups: vec![],
            group_n: Default::default(),
            mewtwo: false,
            grind_pre_e4: 0,
            grind_pre_rematch: 0,
            grind_level: 45,
            grind_yield: 90,
            power_bonus: 1.12,
            battle_scale: 1.0,
            mistake_rate: 0.0,
            type_strength: 0.0,
            ability_bonus: 0.0,
            type_floor: 0.0,
            audited_forced: false,
            audit2: 0,
            starter_model: "random".into(),
            manip: Default::default(),
            torrent_until: String::new(),
            no_salvage_until: String::new(),
            salvage_at: Default::default(),
            salvage_default: -1.0,
            league_redo: 0.0,
            reload_save_at: vec![],
            reload_save_frames: 800,
            reload_frames: 1500,
            reload_fatal_frames: 3900,
            reload_max: 3,
            reload_aggr: 0.8,
            poison_frames_per_hp: 0.0,
            poison_floor: 3,
            torrent_hits: 2.0,
            torrent_stall: true,
            torrent_early: false,
            torrent_heal_below: 0.0,
            carry_power_early: 1.0,
            power_early_until: String::new(),
            extra_fights: vec![],
            risk_until: String::new(),
            risk_power: 1.0,
            risk_foe: 1.0,
            risk_potions: 3,
            risk_noise: Default::default(),
            risk_pay: false,
            risk_adopt: false,
            ablate_torrent: false,
            risk_windows: vec![],
            risk_canon: None,
            fill_after_hof: false,
            mewtwo_candies: 0,
            cond_skips: vec![],
            risk_abandon: false,
            risk_torrent_lost: false,
            early_helpers: false,
            sac_honest: false,
            sac_honest_league: false,
            helper_hp: 0,
            sleep_no_potion: false,
            potion_tiers: vec![],
            ball_shop_from: String::new(),
            early_balls: 0,
            awakenings: 0,
            fly_badge_gate: false,
            coin_case_frames: 0,
            trash_cans: false,
            can_frames: 170,
            cans_fixed: 0,
            played_extra: Default::default(),
            marowak_fight: false,
        }
    }
}

impl Params {
    /// Is the structural route variant `name` on?
    pub fn variant(&self, name: &str) -> bool {
        self.route_variants.iter().any(|v| v == name)
    }

    /// Reads the first ```json fenced block of a plan file.
    pub fn from_plan_text(text: &str) -> Result<Params, String> {
        let start = text.find("```json").ok_or("plan has no ```json block")? + "```json".len();
        let rest = &text[start..];
        let end = rest.find("```").ok_or("unterminated ```json block")?;
        let p: Params =
            serde_json::from_str(&rest[..end]).map_err(|e| format!("plan-params: {e}"))?;
        if !matches!(p.starter_model.as_str(), "random" | "manip") {
            return Err(format!("unknown starter_model {}", p.starter_model));
        }
        p.manip.validate()?;
        Ok(p)
    }
}
