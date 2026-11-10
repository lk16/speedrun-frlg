//! Plan parameters: the machine-readable block of a `human-run/plans/NNN-*.md` file.

use serde::Deserialize;

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
    pub grinds: Vec<Grind>,
    /// Fallback hunts, tried in order when Oak's check finds fewer than 60 caught.
    pub fill: Vec<Hunt>,
    pub stones: Vec<String>,
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
}

impl Default for Params {
    fn default() -> Self {
        Params {
            name: "unnamed".into(),
            starter: "squirtle".into(),
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
            rod: "mixed".into(),
            repel_on: vec![],
            ball_batch: 10,
            safari_bait_from: 0,
            fight_wild: false,
            fight_until_level: 30,
            team_bonus: 1.08,
            hunts: vec![],
            specials: vec![],
            grinds: vec![],
            fill: vec![],
            stones: vec![],
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
        }
    }
}

impl Params {
    /// Reads the first ```json fenced block of a plan file.
    pub fn from_plan_text(text: &str) -> Result<Params, String> {
        let start = text.find("```json").ok_or("plan has no ```json block")? + "```json".len();
        let rest = &text[start..];
        let end = rest.find("```").ok_or("unterminated ```json block")?;
        serde_json::from_str(&rest[..end]).map_err(|e| format!("plan-params: {e}"))
    }
}
