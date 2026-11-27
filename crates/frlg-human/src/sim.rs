//! One attempt of a plan: run the segments, sampling the human's jitter and the game's
//! randomness, applying the reset rules, and report what it cost.
//!
//! Model constants marked `guess` are uncalibrated (see `human-run/plans/*.md`).

use crate::battle::{Status1, XItem};
use crate::data::{Area, Caught, Dex, Slot};
use crate::params::{Evolve, Hunt, Params};
use crate::rng::{hash_str, GameRng, Rng};
use crate::route::{Era, Kind, Seg};
use frlg_mon::stats::{calc_stats, exp_for_level, exp_gain, level_from_exp, Growth};

/// Frames of a wild encounter before the first action is possible (transition, text). guess.
const ENC_INTRO: u32 = 220;
/// Running from a wild battle once it is decided. guess.
const FLEE: u32 = 150;
/// One turn of a trainer or wild battle: move menu, animations off, enemy action, text. guess.
const TURN: u32 = 330;
/// Sending out / defeating one enemy mon: faint, exp text, next send-out. guess.
const MON_OVERHEAD: u32 = 450;
/// Trainer battle outro: money text and walking on. guess.
const TRAINER_OUTRO: u32 = 300;
/// Stat screen and move prompts per level gained. guess.
const LEVEL_UP: u32 = 300;
/// Paying at the Safari gate, the Warden's text and walking in. guess.
const SAFARI_ENTRY: u32 = 1500;
/// The Warden calling time and walking out. guess.
const SAFARI_EXIT: u32 = 900;
/// A Poké Ball throw with its shakes. guess.
const THROW: u32 = 420;
/// Dex entry and nickname prompt after a catch. guess.
const CATCH_END: u32 = 380;
/// Whiting out: text, Pokémon Center, walking back. guess.
const WHITEOUT: u32 = 2500;
/// Chance per turn the enemy crits or our move misses, each forcing a decision. guess.
const P_EVENT: f64 = 0.10;

pub const FPS: f64 = 59.7275;

/// A fight's tactics, resolved from `Params::fight_tactics`.
#[derive(Clone, Debug)]
pub struct TacCfg {
    pub x: Vec<XItem>,
    pub heal_below: f64,
    pub sacs: u32,
    pub torrent: bool,
    pub entry: bool,
    pub power: f64,
    pub potions: i32,
    pub risk_power: f64,
    pub risk_foe: f64,
    pub free_torrent: bool,
}

impl Default for TacCfg {
    fn default() -> Self {
        TacCfg {
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

pub struct Ctx<'a> {
    pub dex: &'a Dex,
    pub p: &'a Params,
    pub segs: &'a [Seg],
    /// Best-case frames to finish from just after segment i (no jitter, no events).
    pub floor_after: Vec<u64>,
    /// Mean frames remaining after segment i, from a no-reset pilot (for `reset_aggr`).
    pub mean_after: Vec<f64>,
    /// Optimal-stopping cutoffs: for a target in frames, the elapsed frames per segment above which
    /// the human resets at that split (NaN = no table entry, use the `reset_aggr` formula).
    pub cut_tables: Vec<(u64, Vec<f64>)>,
    /// Second-variable adjustment of the cutoff per split: (kind 0 none / 1 dex count / 2 lead level,
    /// reference value, frames of cutoff per unit above the reference): reset if
    /// `clock > cut + slope * (x - ref)`. Same indexing as `cut_tables`.
    pub cut_adj: Vec<(u64, Vec<(u8, f64, f64)>)>,
    /// The starter and its two evolutions.
    pub starter_line: [usize; 3],
    /// Levels at which the starter evolves (`decompiled/src/data/pokemon/evolution.h:3-8`).
    pub evo_at: [u32; 2],
    /// First level with a damaging STAB move (`level_up_learnsets.h`: Bubble/Ember 7, Vine Whip 10).
    pub stab_from: u32,
    /// Reset aggressiveness at each segment (the global value unless overridden).
    pub aggr: Vec<f64>,
    /// Moves the carry never forgets (battle_sim), resolved from `Params::keep_moves`.
    pub keep_ids: Vec<usize>,
    /// Per-segment tactics (battle_sim), resolved from `Params::fight_tactics`:
    /// (X items, heal threshold, sacrificial teammates).
    pub tactics: std::collections::HashMap<String, TacCfg>,
    /// Last segment of the Torrent phase / of the no-salvage phase (`Params::torrent_until`,
    /// `Params::no_salvage_until`).
    pub torrent_end: Option<usize>,
    pub power_early_end: Option<usize>,
    /// Last segment of the risk lane (`Params::risk_until`).
    pub risk_end: Option<usize>,
    /// Per segment: the extended lane's (power, foe) factors (0 = the global ones), `Params::risk_windows`.
    pub risk_win: Vec<(f64, f64, Option<u32>)>,
    pub no_salvage_end: Option<usize>,
    /// Salvage check aggressiveness per segment (negative = none), `Params::salvage_at`.
    pub salvage: Vec<f64>,
    /// Index of Oak's check, and (from the no-reset pilot) the fill time as `base + slope * missing
    /// species`, the mean species still to be caught before Oak after segment i, and the mean remaining
    /// frames after segment i without the fill (for `dex_resets`).
    pub oak_idx: usize,
    pub fill_base: f64,
    pub fill_slope: f64,
    pub gain_after: Vec<f64>,
    pub mean_nofill_after: Vec<f64>,
    /// The WR's cumulative seconds at the segment when it ends a WR split (`wrsplits::WR_SPLITS`), else -1.
    pub wr_at: Vec<f64>,
    /// Per segment: skipped when the attempt is more than this many seconds behind the WR at the last
    /// split it passed (`Params::cond_skips`); infinity = never.
    pub cond_skip: Vec<f64>,
    /// MODEL CHANGE `Params::potion_tiers`: HP of one potion use per segment (empty = level tiers).
    pub potion_hp: Vec<i32>,
    /// MODEL CHANGE `Params::ball_shop_from`: first segment where balls can be bought on demand.
    pub ball_shop_from: Option<usize>,
    /// MODEL CHANGE `Params::potion_bag_from`: first segment of the finite potion bag.
    pub potion_bag_from: Option<usize>,
    /// MODEL CHANGE `Params::stone_shops`: first segment in the Celadon Department Store.
    pub stone_shop_from: Option<usize>,
    /// MODEL CHANGE `Params::early_bag`: last segment (`brock`) whose risk-lane fights spend only the
    /// picked-up early Potions.
    pub early_bag_end: Option<usize>,
}

/// HP one potion use restores at segment `seg_i`: the level tier (`battle::heal_amount`: the human buys the tier the
/// level needs) capped by the best tier the plan has bought so far (`Params::potion_tiers`). Full Restores unchanged.
pub fn potion_hp(ctx: &Ctx, seg_i: usize, level: u32, league_items: bool) -> i32 {
    match ctx.potion_hp.get(seg_i) {
        Some(&hp) if !league_items => hp.min(crate::battle::heal_amount(level, false)),
        _ => crate::battle::heal_amount(level, league_items),
    }
}

#[derive(Default, Clone)]
pub struct SegStats {
    /// Attempts that skipped the segment (`Params::cond_skips`).
    pub skips: u64,
    pub n: u64,
    pub sum: f64,
    pub sumsq: f64,
    pub resets: u64,
    pub whiteouts: u64,
    pub abandons: u64,
    pub level_sum: f64,
    pub caught_sum: f64,
    /// Money model: prize money and balls in the bag after the segment (summed).
    pub money_sum: f64,
    pub balls_sum: f64,
}

/// Optional extra measurements from an attempt.
#[derive(Default)]
pub struct Probe {
    /// Kanto species caught when Oak's check starts.
    pub oak_count: u32,
    /// Per fill hunt: (frames spent, species gained, times run).
    pub fill: Vec<(f64, f64, u64)>,
    /// Kanto species caught after each segment (filled when a probe is passed).
    pub caught_at: Vec<u32>,
    /// The starter phase of a `manip` attempt (`manip.rs`).
    pub phase: Option<crate::manip::PhaseSample>,
    /// The carry's level after each segment (filled when a probe is passed).
    pub level_at: Vec<u32>,
    /// Trainer fights of the attempt: (segment name, player actions, whether it was won, frames from
    /// the intro to the last faint without outro and level-up text, the carry's level-ups in it). Only
    /// recorded when a probe is passed.
    pub fights: Vec<(String, u32, bool, f64, u32)>,
    /// National numbers still missing when the fill gave up (an abandoned attempt).
    pub missing: Vec<u32>,
    /// National numbers missing when the fill (or Oak's check) starts.
    pub oak_missing: Vec<u32>,
    /// A snapshot of the bag after segment `snap_at` (set by the caller; `frlg-human bag`): species
    /// caught with the level each was caught at, the Rare Candies in stock, the money and the clock.
    pub snap_at: Option<usize>,
    pub snap: Option<Snapshot>,
    /// Every split reached: (segment index, run clock, wall clock) just before the reset check
    /// (`optstop.rs` trains on these).
    pub split_log: Vec<(u32, u64, u64)>,
    /// Alongside `split_log`: (Kanto species caught, lead level) at each split.
    pub split_x: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// (national number, catch level, order of first catch).
    pub mons: Vec<(u32, u32, u32, u8)>,
    pub candies: u32,
    pub money: u32,
    pub clock: u64,
    pub carry: u32,
    /// The carry's battle-relevant state (`frlg-human lstate`): IVs, EVs, nature, moves, X stock, held
    /// boosters, Mewtwo's pair.
    pub carry_state: CarryState,
}

#[derive(Clone, Debug, Default)]
pub struct CarryState {
    pub ivs: [u32; 6],
    pub evs: [u32; 6],
    pub evs_real: [u32; 6],
    pub nature: u8,
    pub slots: Vec<usize>,
    pub x_stock: [u32; 7],
    pub glasses: bool,
    pub mystic: bool,
    pub ice: bool,
    pub leftovers: bool,
    pub heal_items: u32,
    pub candies: u32,
    pub mewtwo: Option<([u32; 6], u8)>,
    pub mewtwo_evs: [u32; 6],
    pub mewtwo_level: u32,
    pub mewtwo_moves: Vec<usize>,
    pub elixirs: u32,
    pub max_elixirs: u32,
    pub revives: u32,
    pub badges: u8,
}

pub struct Outcome {
    pub wall: u64,
    pub run: u64,
    pub finished: bool,
    pub caught: u32,
    pub whiteouts: u32,
    /// Wall frames spent off the timer (starter resets, boot).
    pub off: u64,
    /// Segment index the attempt ended in (reset, death or the last one).
    pub end_seg: u32,
    /// 0 finished, 1 reset at a split/dex check, 2 starter-phase reject, 3 death/abandon, 4 evo-stat reject.
    pub kind: u8,
}

#[derive(Clone)]
struct State {
    /// The active target in frames (None = no resets) and the clock when the League began.
    target: Option<u64>,
    league_t0: u64,
    clock: u64,
    off: u64,
    exp: u32,
    carry: u32,
    strength: f64,
    power: f64,
    caught: Caught,
    abandoned: bool,
    whiteouts: u32,
    /// The game's RNG stream (re-keyed per segment, advanced per frame by `bump`).
    game: GameRng,
    /// The carry's four move slots, their PP left, and the level up to which level-up
    /// moves were offered.
    slots: Vec<usize>,
    slot_pp: Vec<i32>,
    learned_to: u32,
    /// Status that outlived the last battle (cured by item or Pokémon Center).
    carry_status: Status1,
    /// X items in the bag: Attack, Defend, Speed, Special, Dire Hit.
    x_stock: [u32; 7],
    /// Mewtwo once caught (battle_sim): its rolled IVs and nature, and its move slots.
    mewtwo: Option<([u32; 6], u8)>,
    mewtwo_moves: Vec<usize>,
    /// PP left per Mewtwo slot (parallel to `mewtwo_moves`; `Params::mewtwo_pp`).
    mewtwo_pp: Vec<i32>,
    /// PP Ups applied per Mewtwo slot (0..=3; each is +20% of base PP, `CalculatePPWithBonus`,
    /// `decompiled/src/pokemon.c:3898`), PP Ups held in the bag, and Ether / Max Ether in the bag.
    mewtwo_bonus: Vec<i32>,
    pp_ups: u32,
    ethers: u32,
    max_ethers: u32,
    /// The first-evolution stat gate was already checked.
    evo_checked: bool,
    /// Living expendable teammates (sacrifices burn them; a Centre revives them).
    fodder: u32,
    /// TM and tutor moves already taught: Gen 3 TMs are single-use and each tutor
    /// teaches once (`FLAG_TUTOR_*`, `data/scripts/move_tutors.inc`); HMs are reusable.
    taught_once: Vec<usize>,
    /// Elixirs and Max Elixirs in the bag.
    elixirs: u32,
    max_elixirs: u32,
    revives: u32,
    candies: u32,
    /// Inside the Safari Zone on the current entry (steps and balls left).
    safari_open: bool,
    safari_steps: u32,
    safari_balls: u32,
    /// Money model: prize money, ball stock (Poké, Great, Ultra) and the best ball a mart on the
    /// route so far sells.
    money: u32,
    balls: [u32; 3],
    max_tier: usize,
    /// Repel steps left, and whether a mart on the route has sold them yet.
    repel_left: u32,
    repel_ok: bool,
    /// MODEL CHANGE `Params::awakenings`: Awakenings in the bag.
    awakenings: u32,
    /// PLAN `Params::full_heals`: poison cures in the bag.
    cures: u32,
    /// MODEL CHANGE `Params::coin_case_frames`: the Coin Case has been fetched.
    coin_case: bool,
    /// Battle simulation: the carry's IVs and nature (rolled at the starter), effort values earned,
    /// HP as a fraction, badges (bit 0 Boulder, 4 Soul, 6 Volcano) and whether Surf is known.
    ivs: [u32; 6],
    nature: u8,
    /// Quiet wild seed (`manip.rs`): encounters still allowed in the first `quiet_window` rolls
    /// (`u32::MAX` = no table) and rolls used so far.
    quiet_hits_left: u32,
    quiet_rolls: f64,
    quiet_window: f64,
    evs: [u32; 6],
    /// Effort values actually earned (`Params::ev_lag`: `evs` is what the cached stats were last
    /// computed from), and Mewtwo's pair.
    evs_real: [u32; 6],
    mewtwo_evs: [u32; 6],
    mewtwo_evs_real: [u32; 6],
    /// Black Glasses equipped (`Params::black_glasses`).
    glasses: bool,
    /// MODEL CHANGE `persim_held`: the carry holds the Persim Berry (from `persim-and-potions` until it is
    /// eaten or swapped for the glasses at `black-glasses-swap`).
    persim: bool,
    /// Never-Melt Ice held (pickup kind `neverice`): Ice moves x1.1, instead of the glasses.
    ice: bool,
    /// Leftovers held (pickup kind `leftovers`): 1/16 HP per turn, instead of the glasses.
    leftovers: bool,
    /// Itemfinder in the bag (pickup kind `itemfinder`: the Route 11 east-gate aide, which hands it over only
    /// at 30 Kanto species caught). The six `underfoot` hidden items (`HIDDEN_ITEM_UNDERFOOT`,
    /// `field_control_avatar.c:497-500`: pressing A skips them) can only be dug up with it
    /// (`itemfinder.c:241-246`); the pickup kinds `leftovers` and `ppmax` need it.
    itemfinder: bool,
    /// Mystic Water bought and held (`Params::mystic_after`).
    mystic: bool,
    hp_frac: f64,
    /// Risk lane (`Params::risk_until`): the honest copy's HP fraction, and the timing lane's HP at
    /// the end of the last fight (a rise since then, a heal or a rest, is applied to the risk lane).
    risk_hp: f64,
    risk_real: f64,
    /// MODEL CHANGE `risk_entry_paid`: this fight's timing lane drank free pre-fight potions.
    free_heal: bool,
    /// A risk-lane heal took the carry out of the Torrent window (`Params::risk_adopt`): the Torrent
    /// phase (Center skips, the Torrent tactic) pauses until damage brings it back to <= 1/3 HP.
    torrent_lost: bool,
    /// Early helpers that fainted (`Params::early_helpers`).
    helpers_dead: u32,
    badges: u8,
    league_items: bool,
    /// Heal items left for the League round in progress (bought at its first fight).
    heal_items: u32,
    /// MODEL CHANGE `league_bag_honest`: Full Restores and Max Potions among `heal_items` (the rest are
    /// Hyper Potions), and the League bag's Full Heals.
    lg_fr: u32,
    lg_max: u32,
    lg_fh: u32,
    /// Mewtwo's HP fraction carried between R2 fights.
    mewtwo_hp: f64,
    /// Mewtwo's exp (Slow growth); its level follows (`Params::mewtwo_levels`).
    mewtwo_exp: u32,
    /// Record each trainer fight (see `Probe::fights`).
    log_fights: bool,
    /// Rare Candies eaten on the spot so far (`candy_early_n` caps it).
    candies_eaten: u32,
    fight_log: Vec<(String, u32, bool, f64, u32)>,
    /// Index of the segment being played, and the poison HP not yet charged (frames).
    seg_i: usize,
    poison_acc: f64,
    /// Level each Kanto species was caught at (0 = unknown / not a catch) and the order of first
    /// catches (1 = the starter): a mon caught after the party of six filled up sits in the PC.
    catch_lv: [u8; 152],
    catch_seq: [u16; 152],
    /// Living bodies of each species in the party or box: a catch adds one, an evolution moves one
    /// from the pre-evolution to the target, a trade gives one away. The dex flag stays set.
    bodies: [u8; 152],
    /// How each species was obtained (`SRC_NAMES`) and the source the next `got` records.
    src: [u8; 152],
    cur_src: u8,
    /// Evolution stones in the bag: Moon, Leaf, Thunder, Fire, Water (`stone_ix`).
    stones: [u8; 5],
    /// The tier the current throw sequence wants (`ball_tier`), `None` = the plan's single ball.
    ball_tier: Option<usize>,
    catch_n: u16,
    /// The anchor whose PC visit was already paid (hash), new species caught in the current Safari
    /// entry, and whether the plan's `safari_stop` was reached (the zone is left for good).
    pc_paid: u64,
    safari_new: u32,
    safari_done: bool,
    /// MODEL CHANGE `Params::potion_bag_from`: potions in the bag.
    potions: u32,
    /// PLAN `Params::early_potions`: free Potions picked up and not yet drunk (the risk lane spends them
    /// before buying one).
    free_potions: u32,
    /// PLAN `Params::early_torrent`: the HP fraction the human walks a poisoned carry down to before the
    /// steered fight (set in `sim_battle`, applied to the risk lane in `risk_fight`), and the HP the timing
    /// lane lost doing it (the walk is charged once, for the larger drop).
    steer_to: Option<f64>,
    steer_hp: i32,
    /// The segment of the last PC visit paid (`pc_visit`) and `catch_n` then (`strength_carrier`).
    pc_seg: Option<usize>,
    pc_catch_n: u16,
    /// MODEL CHANGE `Params::carry_pp_ups`: PP Ups applied per carry slot (parallel to `slots`).
    slot_bonus: Vec<i32>,
}

impl Ctx<'_> {
    /// A model constant, overridable through `Params::tune` for sensitivity studies.
    fn k(&self, name: &str, default: f64) -> f64 {
        if self.p.tune.is_empty() {
            default
        } else {
            self.p.tune.get(name).copied().unwrap_or(default)
        }
    }

    /// Frames from the battle starting to the first action menu. Measured on libmgba with A
    /// mashed: 358 (`human-run/calibration.md`), times 1.25 for a human; the older guess was 220.
    fn enc_intro(&self) -> f64 {
        if self.p.calib_wr {
            // The libmgba floor 358 (`calibration.md`) x ~1.06, the WR-level human factor.
            380.0
        } else if self.p.calib_enc {
            450.0
        } else {
            ENC_INTRO as f64
        }
    }

    /// One ball thrown and its result back at the menu: 397-416 measured, x1.2.
    fn throw_cost(&self) -> f64 {
        if self.p.throw_frames > 0 {
            // MODEL CHANGE `throw_frames` (played misses, calibration 2026-11-22).
            return self.p.throw_frames as f64;
        }
        if self.p.calib_wr {
            // Measured floor 397-416 per ball (`calibration.md`) x ~1.05.
            425.0
        } else if self.p.calib_enc {
            480.0
        } else {
            THROW as f64
        }
    }

    /// After the last shake: Gotcha text, dex page, nickname prompt. The measured throw-to-overworld
    /// total is 409-549, so what comes after the throw cycle is small.
    fn catch_end(&self) -> f64 {
        let base = if self.p.catch_end_new > 0 {
            // MODEL CHANGE `catch_end_new`: a new species' dex page (`battle_scripts_2.s:73-91`).
            self.p.catch_end_new as f64
        } else if self.p.calib_wr {
            130.0
        } else if self.p.calib_enc {
            150.0
        } else {
            CATCH_END as f64 * self.p.text_factor
        };
        // The caught-jingle wait spans catch_end and the tail of the throw animation; the
        // measured whole-catch saving is credited here (clamped so the segment stays positive).
        if self.p.clemi_skip {
            (base - 140.0).max(10.0)
        } else {
            base
        }
    }

    /// After a trainer's last mon faints: the defeat text, prize money, walking on. `calib_wr`: 240
    /// frames (guess; the WR video's fight durations end at the "defeated" box, so this part of a
    /// trainer encounter is not measured).
    fn outro(&self) -> f64 {
        if self.p.calib_wr {
            self.k("trainer_outro", 240.0)
        } else {
            self.k("trainer_outro", TRAINER_OUTRO as f64)
        }
    }

    /// A level-up: the "grew to" box and the stat screen. `calib_wr`: 240 frames (Brock in the WR
    /// video: victory text +28 s, "grew to LV 13" +32 s, next prompt +44 s with a learn prompt between).
    fn level_up(&self) -> f64 {
        if self.p.calib_wr {
            self.k("level_up", 240.0)
        } else {
            self.k("level_up", LEVEL_UP as f64)
        }
    }

    /// One battle turn. `calib_battle`: 385 = the 280-406 frame floor measured on libmgba
    /// (mean ~320 with A mashed) times 1.2 (`human-run/calibration.md`).
    fn turn(&self) -> f64 {
        if self.p.calib_wr {
            // 345 = 5.74 s per player action, fitted with `mon_overhead` to the 16 fight durations
            // read off the WR video (`human-run/calibration.md`, "WR fight fit").
            345.0
        } else if self.p.calib_battle {
            385.0
        } else {
            TURN as f64 * self.p.battle_scale
        }
    }

    /// Faint, exp text and the next send-out per enemy mon beyond the turn itself. `calib_battle`:
    /// 810 = (975 measured wild kill cycle - 320 turn) x 1.2.
    fn mon_overhead(&self) -> f64 {
        if self.p.calib_wr {
            // 10.5 s per enemy mon (send-out, faint, exp text), same fit as `turn`.
            630.0
        } else if self.p.calib_battle {
            810.0
        } else {
            MON_OVERHEAD as f64 * self.p.battle_scale
        }
    }

    /// Frames from a trainer battle starting to its first menu. `calib_battle`: at least
    /// 1250 = the 1040 frames measured for the lab rival x 1.2.
    fn trainer_intro(&self, intro: u32) -> f64 {
        if self.p.calib_wr {
            // Swirl, the trainer's text and the send-out: ~10-14 s observed for E4 rounds (intro
            // 900 -> 720 frames); the fit is insensitive to it (video fight durations include it).
            return intro as f64 * 0.8;
        }
        let t = intro as f64 * self.p.text_factor;
        if self.p.calib_battle {
            t.max(1250.0)
        } else {
            t
        }
    }

    pub fn floor_frames(&self, seg: &Seg) -> f64 {
        let p = self.p;
        // MODEL CHANGE `vod_frames`: a WR-VOD-measured lump replaces the guessed text / walk frames.
        if matches!(seg.kind, Kind::Text { .. } | Kind::Walk { .. }) {
            if let Some(&f) = p.vod_frames.get(&seg.name) {
                return f;
            }
        }
        match &seg.kind {
            // MODEL CHANGE `fly_frames`: a played Fly, menu to control on arrival (calibration 2026-11-22).
            Kind::Text { .. } if p.fly_frames > 0 && seg.name.starts_with("fly-") => {
                p.fly_frames as f64
            }
            Kind::Text { frames } => *frames as f64 * p.text_factor,
            Kind::Walk { tiles, era, .. } => {
                *tiles as f64 * era.frames_per_tile() * p.path_overhead
            }
            Kind::Trainer { party, intro, .. } => {
                self.trainer_intro(*intro)
                    + party.len() as f64 * (self.turn() + self.mon_overhead())
                    + self.outro()
            }
            Kind::Parties { parties, .. } => parties
                .iter()
                .map(|p| {
                    self.trainer_intro(600)
                        + p.len() as f64 * (self.turn() + self.mon_overhead())
                        + self.outro()
                })
                .sum(),
            Kind::Group { n, mons, .. } => {
                *n as f64
                    * (self.trainer_intro(600)
                        + *mons as f64 * (self.turn() + self.mon_overhead())
                        + self.outro())
            }
            Kind::Wander { .. }
            | Kind::Hunt { .. }
            | Kind::OakCheck
            | Kind::Stones
            | Kind::Mewtwo => 0.0,
            Kind::Special { .. } | Kind::Mart { .. } | Kind::Evolve { .. } => 0.0,
            Kind::Teach { teach } => teach.frames as f64 * self.p.text_factor,
            Kind::XBuy { xbuy } => xbuy.frames as f64 * self.p.text_factor,
            Kind::Elixir { pickup } => pickup.frames as f64 * self.p.text_factor,
            Kind::TeleportOrWalk { frames, .. } => *frames as f64 * self.p.text_factor,
            Kind::Grind { .. } => 0.0,
        }
    }
}

/// Where a dex entry came from, for `frlg-human bag`: caught, gift, trade, bought, evolved by stone,
/// by candy, by a level-up (lead swap).
pub const SRC_NAMES: [&str; 7] = ["catch", "gift", "trade", "buy", "stone", "candy", "level"];

/// Index of an evolution stone in `State::stones` by the plan's name for it.
fn stone_ix(name: &str) -> Option<usize> {
    ["MOON", "LEAF", "THUNDER", "FIRE", "WATER"]
        .iter()
        .position(|&n| name.eq_ignore_ascii_case(n))
}

/// A Kanto species obtained for the first time (caught, gifted, traded, evolved): remembers the level
/// and the order, for the candy arithmetic and the PC logistics of `Evolve` and trades.
fn got(st: &mut State, dex: u32, level: u32) -> bool {
    let new = st.caught.insert(dex);
    if new {
        st.catch_n += 1;
        st.catch_lv[dex as usize] = level.min(255) as u8;
        st.catch_seq[dex as usize] = st.catch_n;
        st.bodies[dex as usize] = st.bodies[dex as usize].saturating_add(1);
        st.src[dex as usize] = st.cur_src;
    }
    new
}

/// The party holds six: whatever was obtained after that is in the box and has to be withdrawn before
/// it can evolve or be traded (`Params::pc_frames`, charged once per anchor).
fn pc_visit(ctx: &Ctx, st: &mut State, dexes: &[u32], anchor: &str) {
    if ctx.p.pc_frames == 0 || !dexes.iter().any(|&d| st.catch_seq[d as usize] > 6) {
        return;
    }
    let h = hash_str(anchor);
    if st.pc_paid != h {
        st.pc_paid = h;
        bump(st, ctx.p.pc_frames as f64 * ctx.p.text_factor);
    }
    st.pc_seg = Some(st.seg_i);
    st.pc_catch_n = st.catch_n;
}

/// MODEL CHANGE `Params::strength_carrier`: before Victory Road's boulders a party mon must know
/// Strength (`decompiled/data/maps/VictoryRoad_1F/map.json` `EventScript_StrengthBoulder`; HM04 from the
/// Warden). The carrier is the earliest-obtained living species outside the carry's line that learns HM04
/// (`src/data/pokemon/tmhm_learnsets.h`): in the party (first six obtained) it costs nothing beyond the
/// teach in `warden-strength`; in the box it is withdrawn at a Pokemon Center (`pc_frames`), unless a PC
/// visit the plan already paid came after the Warden and after the carrier was obtained. With no carrier
/// the carry itself learns it (a forced slot). Returns what was done, for traces and the controller.
fn strength_carrier(ctx: &Ctx, st: &mut State) -> &'static str {
    let mv = ctx.dex.move_id("MOVE_STRENGTH");
    if st.slots.contains(&mv) {
        return "carry";
    }
    let line: Vec<u32> = (1..=100)
        .map(|l| ctx.dex.species[carry_species(ctx, l)].dex)
        .collect();
    let best = (1..=151u32)
        .filter(|&d| st.bodies[d as usize] > 0 && !line.contains(&d))
        .filter(|&d| {
            ctx.dex
                .species
                .iter()
                .position(|sp| sp.dex == d)
                .is_some_and(|id| can_learn(ctx, id, mv))
        })
        .min_by_key(|&d| st.catch_seq[d as usize]);
    let Some(d) = best else {
        offer_move(ctx, st, mv, true);
        return "carry-forced";
    };
    let seq = st.catch_seq[d as usize];
    if std::env::var_os("FRLG_SIM_TRACE").is_some() {
        let pc = st
            .pc_seg
            .and_then(|i| ctx.segs.get(i))
            .map(|s| s.name.as_str());
        eprintln!("strength-carrier dex {d} seq {seq} last-pc {pc:?}");
    }
    if seq <= 6 {
        return "party";
    }
    let warden = ctx.segs.iter().position(|s| s.name == "warden-strength");
    let rode = st.pc_seg.is_some_and(|p| warden.is_some_and(|w| p > w)) && st.pc_catch_n >= seq;
    if rode {
        return "box-rode";
    }
    bump(st, ctx.p.pc_frames as f64 * ctx.p.text_factor);
    st.pc_seg = Some(st.seg_i);
    st.pc_catch_n = st.catch_n;
    "box"
}

fn bump(st: &mut State, f: f64) {
    let n = f.round() as u64;
    st.clock += n;
    // The VBlank advance (`decompiled/src/main.c:412`): time moves the game's stream.
    st.game.frames(n.min(u32::MAX as u64) as u32);
}

fn jitter(ctx: &Ctx, rng: &mut Rng, inputs: u32) -> u32 {
    let n = ctx.p.jitter_n;
    if n == 0 {
        return 0;
    }
    (0..inputs).map(|_| rng.below(n + 1)).sum()
}

/// Mistakes over `units` hundreds of tiles or inputs.
fn mistakes(ctx: &Ctx, rng: &mut Rng, units: f64) -> f64 {
    let rate = ctx.p.mistake_rate;
    if rate <= 0.0 {
        return 0.0;
    }
    let p = (rate / 100.0).min(0.5);
    let n = (units * 100.0) as u64;
    let mut total = 0.0;
    let mut left = n;
    loop {
        let gap = rng.geometric(p);
        if gap >= left {
            break;
        }
        left -= gap + 1;
        total += rng.range(60, 300) as f64 + react(ctx, rng) as f64;
    }
    total
}

fn react(ctx: &Ctx, rng: &mut Rng) -> u32 {
    rng.range(ctx.p.react_lo, ctx.p.react_hi)
}

fn effective_level(st: &State, p: &Params) -> f64 {
    (st.carry as f64 * p.team_bonus).max(st.power)
}

/// A move slot's worth to a mon of `species` when something has to go: damage against a neutral
/// dummy with STAB; keep-listed moves are effectively permanent.
pub fn move_utility(ctx: &Ctx, species: usize, mv: usize) -> f64 {
    if let Some(pos) = ctx.keep_ids.iter().position(|&k| k == mv) {
        return 1_000_000.0 - pos as f64;
    }
    let m = &ctx.dex.moves[mv];
    if m.power == 0 {
        return 0.0;
    }
    let types = ctx.dex.species[species].types;
    let stab = if types.0 == m.mtype || types.1 == m.mtype {
        1.5
    } else {
        1.0
    };
    let acc = if m.acc == 0 { 100 } else { m.acc } as f64 / 100.0;
    m.power as f64 * stab * acc
}

/// Learn a move into the four slots: fill a free slot, else replace the lowest-utility
/// slot if the newcomer is worth more (`forced` teaches always find a victim).
fn offer_move(ctx: &Ctx, st: &mut State, mv: usize, forced: bool) {
    if st.slots.contains(&mv) {
        return;
    }
    st.slot_bonus.resize(st.slots.len(), 0);
    if st.slots.len() < 4 {
        st.slots.push(mv);
        st.slot_pp.push(ctx.dex.moves[mv].pp as i32);
        st.slot_bonus.push(0);
        apply_carry_pp_ups(ctx, st);
        return;
    }
    if let Some(worst_i) = replace_slot(ctx, carry_species(ctx, st.carry), &st.slots, mv, forced) {
        st.slots[worst_i] = mv;
        st.slot_pp[worst_i] = ctx.dex.moves[mv].pp as i32;
        st.slot_bonus[worst_i] = 0;
        apply_carry_pp_ups(ctx, st);
    }
}

/// Max PP of carry slot `k` with its PP Ups (`decompiled/src/pokemon.c` `CalculatePPWithBonus`:
/// base + base * 20 * ups / 100).
fn carry_full_pp(ctx: &Ctx, st: &State, k: usize) -> i32 {
    let base = ctx.dex.moves[st.slots[k]].pp as i32;
    let ups = st.slot_bonus.get(k).copied().unwrap_or(0);
    base + base * 20 * ups / 100
}

/// MODEL CHANGE `Params::carry_pp_ups`: the PP Ups in the bag go to the carry's listed moves, in the
/// plan's order, at most 3 per move (`pokemon.c` `PP_UP_SHIFTS`); a PP Up also adds the PP it grants.
/// Those not usable yet (the move not known) stay in the bag. 350 frames each (bag, party, move).
fn apply_carry_pp_ups(ctx: &Ctx, st: &mut State) {
    if ctx.p.carry_pp_ups.is_empty() {
        return;
    }
    st.slot_bonus.resize(st.slots.len(), 0);
    while st.pp_ups > 0 {
        let slot = ctx.p.carry_pp_ups.iter().find_map(|n| {
            let m = ctx.dex.move_id(n);
            st.slots
                .iter()
                .position(|&x| x == m)
                .filter(|&k| st.slot_bonus[k] < 3)
        });
        let Some(k) = slot else { break };
        let before = carry_full_pp(ctx, st, k);
        st.slot_bonus[k] += 1;
        st.slot_pp[k] += carry_full_pp(ctx, st, k) - before;
        st.pp_ups -= 1;
        bump(st, 350.0 * ctx.p.text_factor);
    }
}

/// The slot a full set of four gives up for `mv` ([`move_utility`]: the lowest-worth slot, if the
/// newcomer is worth more or the teach is `forced`), `None` = keep the four. The simulator's
/// [`offer_move`] and the playing controller's "delete a move?" answer.
pub fn replace_slot(
    ctx: &Ctx,
    species: usize,
    slots: &[usize],
    mv: usize,
    forced: bool,
) -> Option<usize> {
    let (worst_i, worst_u) = slots
        .iter()
        .enumerate()
        .map(|(i, &s)| (i, move_utility(ctx, species, s)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())?;
    let new_u = move_utility(ctx, species, mv);
    (forced || new_u > worst_u).then_some(worst_i)
}

/// Offer every level-up move between the last processed level and the current one,
/// using the learnset of the stage the carry had at each level.
fn learn_moves(ctx: &Ctx, st: &mut State) {
    while st.learned_to < st.carry {
        st.learned_to += 1;
        let l = st.learned_to;
        let sp = carry_species(ctx, l);
        let learn: Vec<usize> = ctx.dex.species[sp]
            .learnset
            .iter()
            .filter(|&&(ll, _)| ll == l)
            .map(|&(_, m)| m)
            .collect();
        for m in learn {
            offer_move(ctx, st, m, false);
        }
    }
}

/// Mewtwo's PP back to full for every slot (Pokémon Centre, whiteout, the catch itself).
fn restore_mewtwo_pp(ctx: &Ctx, st: &mut State) {
    st.mewtwo_bonus.resize(st.mewtwo_moves.len(), 0);
    st.mewtwo_pp = (0..st.mewtwo_moves.len())
        .map(|k| mewtwo_full_pp(ctx, st, k))
        .collect();
}

/// PP of Mewtwo's slot `k` when full: base PP plus 20% per PP Up (`CalculatePPWithBonus`).
fn mewtwo_full_pp(ctx: &Ctx, st: &State, k: usize) -> i32 {
    let base = ctx.dex.moves[st.mewtwo_moves[k]].pp as i32;
    base + base * 20 * st.mewtwo_bonus.get(k).copied().unwrap_or(0) / 100
}

/// Drink an Ether / Max Ether on Mewtwo's emptiest slot (relative to its full PP): Ether +10
/// (`item_effects.h:116`), Max Ether to full (`:121`). Max Ether goes first when the slot is
/// more than 10 short. Returns whether one was used.
fn use_ether(ctx: &Ctx, st: &mut State) -> bool {
    if st.mewtwo_pp.is_empty() || (st.max_ethers == 0 && st.ethers == 0) {
        return false;
    }
    let k = (0..st.mewtwo_pp.len())
        .min_by(|&a, &b| {
            let ra = st.mewtwo_pp[a] as f64 / mewtwo_full_pp(ctx, st, a) as f64;
            let rb = st.mewtwo_pp[b] as f64 / mewtwo_full_pp(ctx, st, b) as f64;
            ra.total_cmp(&rb)
        })
        .unwrap_or(0);
    let full = mewtwo_full_pp(ctx, st, k);
    if st.max_ethers > 0 && (st.ethers == 0 || full - st.mewtwo_pp[k] > 10) {
        st.max_ethers -= 1;
        st.mewtwo_pp[k] = full;
    } else {
        st.ethers -= 1;
        st.mewtwo_pp[k] = (st.mewtwo_pp[k] + 10).min(full);
    }
    bump(st, 450.0 * ctx.p.text_factor);
    true
}

/// Use the PP Ups in the bag on Mewtwo: Psychic first, then Fire Blast, up to 3 each
/// (`decompiled/src/pokemon.c:4207`: at most 3 per move); a PP Up also adds the PP it grants.
fn apply_pp_ups(ctx: &Ctx, st: &mut State) -> f64 {
    let mut frames = 0.0;
    if !ctx.p.mewtwo_pp || st.mewtwo.is_none() {
        return frames;
    }
    st.mewtwo_bonus.resize(st.mewtwo_moves.len(), 0);
    let order = [
        ctx.dex.move_id("MOVE_PSYCHIC"),
        ctx.dex.move_id("MOVE_FIRE_BLAST"),
        ctx.dex.move_id("MOVE_EARTHQUAKE"),
    ];
    while st.pp_ups > 0 {
        let slot = order.iter().find_map(|&m| {
            st.mewtwo_moves
                .iter()
                .position(|&x| x == m)
                .filter(|&k| st.mewtwo_bonus[k] < 3)
        });
        let Some(k) = slot else { break };
        let before = mewtwo_full_pp(ctx, st, k);
        st.mewtwo_bonus[k] += 1;
        let after = mewtwo_full_pp(ctx, st, k);
        st.mewtwo_pp[k] += after - before;
        st.pp_ups -= 1;
        frames += 350.0 * ctx.p.text_factor;
    }
    frames
}

/// `FRLG_EXP_TRACE`: print the carry's level and exp at the start of every segment.
fn exp_trace() -> bool {
    static T: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *T.get_or_init(|| std::env::var_os("FRLG_EXP_TRACE").is_some())
}

/// `EXP_SLOW(n)` (`decompiled/src/data/pokemon/experience_tables.h:4`), Mewtwo's growth rate.
fn slow_exp(n: u32) -> u32 {
    5 * n * n * n / 4
}

/// Mewtwo's level: 70 as caught, then from its exp when `Params::mewtwo_levels` is on.
fn mewtwo_level(ctx: &Ctx, st: &State) -> u32 {
    if !ctx.p.mewtwo_levels || st.mewtwo_exp == 0 {
        return 70;
    }
    let mut l = 70;
    while l < 100 && st.mewtwo_exp >= slow_exp(l + 1) {
        l += 1;
    }
    l
}

/// `MonGainEVs` (`decompiled/src/pokemon.c:5512-5595`): stats in the order HP, Atk, Def, Spe, SpA, SpD;
/// 255 per stat, and with `Params::ev_cap` 510 in total.
fn add_evs(ctx: &Ctx, real: &mut [u32; 6], y: [u32; 6]) {
    let mut total: u32 = real.iter().sum();
    for (e, y) in real.iter_mut().zip(y) {
        let mut inc = y;
        if ctx.p.ev_cap {
            if total >= 510 {
                break;
            }
            inc = inc.min(510 - total);
        }
        inc = inc.min(255u32.saturating_sub(*e));
        *e += inc;
        total += inc;
    }
}

fn add_exp(ctx: &Ctx, st: &mut State, e: u32) -> u32 {
    st.exp = st.exp.saturating_add(e);
    let new = level_from_exp(Growth::MediumSlow, st.exp) as u32;
    let gained = new.saturating_sub(st.carry);
    st.carry = new.max(st.carry);
    let [_sq, war, bla] = ctx.starter_line;
    if st.carry >= ctx.evo_at[0] {
        st.caught.insert(ctx.dex.species[war].dex);
    }
    if st.carry >= ctx.evo_at[1] {
        st.caught.insert(ctx.dex.species[bla].dex);
    }
    if gained > 0 {
        learn_moves(ctx, st);
        // `CalculateMonStats` runs on a level-up: the cached stats pick up the EVs.
        st.evs = st.evs_real;
    }
    gained
}

/// Weighted slot pick (weights sum to 100).
fn pick_slot(rng: &mut Rng, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();
    let mut r = rng.below(total);
    for (i, w) in weights.iter().enumerate() {
        if r < *w {
            return i;
        }
        r -= w;
    }
    weights.len() - 1
}

fn isqrt(n: u32) -> u32 {
    (n as f64).sqrt() as u32
}

/// The four shake checks of `Cmd_handleballthrow`
/// (`decompiled/src/battle_script_commands.c:9549-9640`), on the game's own stream.
fn shakes_catch(g: &mut GameRng, odds: u32) -> bool {
    if odds > 254 {
        return true;
    }
    let o = odds.max(1);
    let s = isqrt(isqrt(16_711_680 / o)).max(1);
    let thr = 1_048_560 / s;
    (0..4).all(|_| (g.roll() as u32) < thr)
}

/// The status bonus of the same function: sleep/freeze double the odds, any other
/// status is x1.5 (`battle_script_commands.c:9553-9556`).
fn status_odds(odds: u32, status: Status1) -> u32 {
    match status {
        Status1::Sleep(_) | Status1::Freeze => odds * 2,
        Status1::None => odds,
        _ => odds * 15 / 10,
    }
}

/// Safari Zone encounter: `safari_rocks` rocks first, then Safari Balls until caught, out of
/// balls or the mon flees. A rock doubles the catch factor and the flee factor for 2-6 turns (the
/// counter runs down once per enemy turn, then the catch factor resets), a ball uses the current
/// factor (`decompiled/src/battle_main.c:4339-4408`, `battle_ai_script_commands.c:1715-1732`,
/// `battle_script_commands.c:9497`, ball multiplier 15 at `:808-814`). The throw order is
/// [`SafariMon::next_act`], shared with the playing controller (`play/dex.rs`).
fn safari_catch(ctx: &Ctx, st: &mut State, rng: &mut Rng, sp: &crate::data::Species) -> bool {
    let mut m = SafariMon::new(ctx, sp);
    loop {
        match m.next_act() {
            SafariAct::Bait => {
                m.bait = (m.bait + rng.range(2, 6)).min(6);
                m.counter = 0;
                bump(st, THROW as f64 * 0.6);
                m.factor = (m.factor >> 1).max(3);
            }
            SafariAct::Rock => {
                bump(st, THROW as f64 * 0.6);
                m.counter = (m.counter + rng.range(2, 6)).min(6);
                m.factor = (m.factor * 2).min(20);
            }
            SafariAct::Ball => {
                if st.safari_balls == 0 {
                    return false;
                }
                st.safari_balls -= 1;
                bump(st, ctx.throw_cost());
                let odds = safari_ball_odds(m.factor);
                let mut g = st.game;
                let caught = shakes_catch(&mut g, odds);
                st.game = g;
                if caught {
                    bump(st, ctx.catch_end());
                    return true;
                }
                bump(st, react(ctx, rng) as f64);
            }
        }
        if rng.below(100) < m.flee_factor() * 5 {
            bump(st, FLEE as f64);
            return false;
        }
        if m.counter > 0 {
            m.counter -= 1;
            if m.counter == 0 {
                m.factor = m.base;
            }
        } else {
            m.bait = m.bait.saturating_sub(1);
        }
    }
}

/// What the human throws next in a Safari battle (the menu is BALL, BAIT, ROCK, RUN:
/// `decompiled/src/battle_controller_safari.c:168-182`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafariAct {
    Bait,
    Rock,
    Ball,
}

/// One Safari encounter as the plan plays it: the catch and escape factors the game sets when the
/// battle starts (`decompiled/src/battle_main.c:2282-2287`), the rock / bait counters, and what is
/// left of the plan's opening (one bait first when `safari_bait_from` says the mon flees easily,
/// `safari_rocks` rocks, then balls).
#[derive(Clone, Copy, Debug)]
pub struct SafariMon {
    pub base: u32,
    pub factor: u32,
    pub esc: u32,
    pub counter: u32,
    pub bait: u32,
    pub rocks: u32,
    pub bait_pending: bool,
}

impl SafariMon {
    pub fn new(ctx: &Ctx, sp: &crate::data::Species) -> Self {
        let base = sp.catch_rate * 100 / 1275;
        let esc = (sp.safari_flee * 100 / 1275).max(2);
        SafariMon {
            base,
            factor: base,
            esc,
            counter: 0,
            bait: 0,
            rocks: ctx.p.safari_rocks,
            bait_pending: ctx.p.safari_bait_from > 0 && esc >= ctx.p.safari_bait_from,
        }
    }

    /// The plan's next throw: the bait (once), then the rocks, then balls.
    pub fn next_act(&mut self) -> SafariAct {
        if self.bait_pending {
            self.bait_pending = false;
            SafariAct::Bait
        } else if self.rocks > 0 {
            self.rocks -= 1;
            SafariAct::Rock
        } else {
            SafariAct::Ball
        }
    }

    /// Flee check at the end of the turn passes below `flee_factor * 5` in 100: doubled while a
    /// rock counter runs, quartered while eating bait (`battle_ai_script_commands.c:1715-1732`).
    pub fn flee_factor(&self) -> u32 {
        if self.counter > 0 {
            (self.esc * 2).min(20)
        } else if self.bait > 0 {
            (self.esc / 4).max(1)
        } else {
            self.esc
        }
    }
}

/// Money a Game Corner prize really costs (`buy` specials price coins at 20 each): with the Coin Case
/// model (`Params::coin_case_frames`) the coins come in purchases of 50 for 1,000 (`strings.c:425`).
fn buy_cost(ctx: &Ctx, cost: u32) -> u32 {
    if ctx.p.coin_case_frames > 0 {
        (cost / 20).div_ceil(50) * 1000
    } else {
        cost
    }
}

/// The number of trash cans the human checks in the Vermilion Gym (`Params::trash_cans`): the switch
/// layout of `SetVermilionTrashCans` (`field_specials.c:552`: first switch 1..15 uniform, the second a
/// neighbour +-1 in the row or +-5 in the column with equal odds), checked in order 1..15 until the first
/// switch, then one guess among the neighbours; a wrong guess re-rolls both (`VermilionCity_Gym/scripts.inc`
/// `TrySwitchTwo`).
pub fn trash_can_checks(rng: &mut Rng) -> u32 {
    let mut checks = 0;
    loop {
        let first = 1 + (rng.next_u64() % 15) as i32;
        let col = (first - 1) % 5;
        let mut nb: Vec<i32> = vec![];
        if col > 0 {
            nb.push(first - 1);
        }
        if col < 4 {
            nb.push(first + 1);
        }
        if first > 5 {
            nb.push(first - 5);
        }
        if first <= 10 {
            nb.push(first + 5);
        }
        let second = nb[(rng.next_u64() % nb.len() as u64) as usize];
        checks += first as u32;
        // The human guesses a neighbour uniformly (he cannot see which one the game picked).
        let guess = nb[(rng.next_u64() % nb.len() as u64) as usize];
        checks += 1;
        if guess == second {
            return checks;
        }
    }
}

/// The odds of one Safari Ball at catch factor `factor`: the factor back to a catch rate
/// (`* 1275 / 100`), the ball's x1.5 (`battle_script_commands.c:808-814`), a full-HP mon
/// (`:9497,9549-9556`).
pub fn safari_ball_odds(factor: u32) -> u32 {
    let rate = (factor * 1275 / 100).min(255);
    (rate * 15 / 10) * 100 / 300
}

/// Mart prices, `decompiled/src/data/items.json`.
const BALL_PRICE: [u32; 3] = [200, 600, 1200];
pub const BALL_MULT: [u32; 3] = [10, 15, 20];
/// Walking into a mart, buying, walking out. guess.
const MART: u32 = 900;

/// The plan's single ball as a tier (0 Poke, 1 Great, 2 Ultra).
fn plan_tier(p: &Params) -> usize {
    match p.ball.as_str() {
        "great" => 1,
        "ultra" => 2,
        _ => 0,
    }
}

/// The ball tier thrown at a species of this catch rate (`species_info.h` `catchRate`): the plan's
/// ball rule (`poke_from`, `ultra_below`) or its single `ball`. A Poke Ball is as good as a Great Ball
/// on a rate-255 mon and costs a third (`items.json`: 200 / 600 / 1200).
pub fn ball_tier(ctx: &Ctx, catch_rate: u32) -> usize {
    let p = ctx.p;
    if p.poke_from > 0 && catch_rate >= p.poke_from {
        0
    } else if p.ultra_below > 0 && catch_rate < p.ultra_below {
        2
    } else if p.poke_from > 0 || p.ultra_below > 0 {
        1
    } else {
        plan_tier(p)
    }
}

/// The ball for the next throw under the money model: the best type the plan allows and a
/// mart on the route has sold so far, bought in batches of `ball_batch` as money allows, else a
/// cheaper one. `None` when not even a Poké Ball is affordable.
fn take_ball(ctx: &Ctx, st: &mut State) -> Option<u32> {
    let mut t = st
        .ball_tier
        .unwrap_or_else(|| plan_tier(ctx.p))
        .min(st.max_tier);
    if std::env::var_os("FRLG_BALL_TRACE").is_some() {
        eprintln!("ball-take seg {} tier {t}", st.seg_i);
    }
    loop {
        if st.balls[t] > 0 {
            st.balls[t] -= 1;
            return Some(BALL_MULT[t]);
        }
        if !ctx.p.marts.is_empty() || ctx.ball_shop_from.is_some_and(|f| st.seg_i < f) {
            // Scheduled mart stops only (or, MODEL CHANGE `ball_shop_from`, no mart on the way yet):
            // use whatever is in the bag.
            let Some(t) = bag_ball(t, st.balls) else {
                if std::env::var_os("FRLG_BALL_TRACE").is_some() {
                    eprintln!("ball-empty seg {}", st.seg_i);
                }
                return None;
            };
            st.balls[t] -= 1;
            return Some(BALL_MULT[t]);
        }
        let qty = (st.money / BALL_PRICE[t]).min(ctx.p.ball_batch);
        if std::env::var_os("FRLG_BALL_TRACE").is_some() {
            eprintln!("ball-buy seg {} tier {t} qty {qty}", st.seg_i);
        }
        if qty > 0 {
            st.money -= qty * BALL_PRICE[t];
            st.balls[t] += qty;
            bump(st, MART as f64 * ctx.p.text_factor);
            continue;
        }
        if t == 0 {
            return None;
        }
        t -= 1;
    }
}

/// The ball actually thrown when the plan wants tier `want` (0 Poke, 1 Great, 2 Ultra) and the bag
/// holds `have`: that tier, else the next cheaper one in the bag (`take_ball` with scheduled marts;
/// the playing controller, whose bag is the real one). `None` = no ball at all.
pub fn bag_ball(want: usize, have: [u32; 3]) -> Option<usize> {
    (0..=want.min(2)).rev().find(|&t| have[t] > 0)
}

/// Patience: throws at one wild mon before the human gives up on it (`throw_balls`' cap).
pub const MAX_THROWS: u32 = 8;

/// The catch odds of `Cmd_handleballthrow` (`decompiled/src/battle_script_commands.c:9549-9556`):
/// `catchRate * ball / 10 * (3 maxHP - 2 hp) / (3 maxHP)`, then the status bonus.
pub fn catch_odds(catch_rate: u32, ball_mult: u32, hp: u32, max_hp: u32, status: Status1) -> u32 {
    let max_hp = max_hp.max(1);
    status_odds(
        (catch_rate * ball_mult / 10) * (max_hp * 3 - hp.min(max_hp) * 2) / (3 * max_hp),
        status,
    )
}

/// Probability that one ball with these odds catches: four shakes, each passing with
/// `thr / 65536` ([`shakes_catch`]).
pub fn catch_prob(odds: u32) -> f64 {
    if odds > 254 {
        return 1.0;
    }
    let s = isqrt(isqrt(16_711_680 / odds.max(1))).max(1);
    let thr = (1_048_560 / s).min(65536) as f64 / 65536.0;
    thr.powi(4)
}

/// Throws until caught or patience (8 throws) runs out. Formula:
/// `decompiled/src/battle_script_commands.c:9549-9640`, status bonus included.
#[allow(clippy::too_many_arguments)]
fn throw_balls(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    catch_rate: u32,
    hp_frac: f64,
    status: Status1,
    ball: Option<u32>,
    max_throws: u32,
) -> bool {
    let max_hp = 100u32;
    let hp = ((hp_frac * max_hp as f64) as u32).max(1);
    for _ in 0..max_throws {
        let b = match ball {
            Some(b) => b,
            None => match take_ball(ctx, st) {
                Some(b) => b,
                None => return false,
            },
        };
        let odds = catch_odds(catch_rate, b, hp, max_hp, status);
        bump(st, ctx.throw_cost());
        let mut g = st.game;
        let caught = shakes_catch(&mut g, odds);
        st.game = g;
        if caught {
            bump(st, ctx.catch_end());
            return true;
        }
        bump(st, react(ctx, rng) as f64);
    }
    false
}

/// `encounter`'s catch rule: a Kanto species (`dex != 0`) not owned yet, not in the hunt's skip
/// list, on an encounter where the human catches at all (`want_catch`: a hunt, or a passing
/// catch the plan allows). Shared with the playing controller (`play/dex.rs`), which reads
/// `owned` from the Pokédex flags in RAM.
pub fn wants_catch(
    ctx: &Ctx,
    species: usize,
    owned: bool,
    skip: &[usize],
    want_catch: bool,
) -> bool {
    want_catch && ctx.dex.species[species].dex != 0 && !owned && !skip.contains(&species)
}

/// `hunt`'s area by the plan's word (`land`, `water`, `fish`).
pub fn hunt_area(h: &Hunt) -> Area {
    match h.area.as_str() {
        "water" => Area::Water,
        "fish" => Area::Fish,
        _ => Area::Land,
    }
}

/// `hunt`'s skip list: the plan's `skip` plus every species below `hunt_min_catch`.
pub fn hunt_skip(ctx: &Ctx, h: &Hunt) -> Vec<usize> {
    let mut skip: Vec<usize> = h.skip.iter().map(|s| ctx.dex.id(s)).collect();
    if ctx.p.hunt_min_catch > 0 {
        skip.extend(
            (0..ctx.dex.species.len())
                .filter(|&i| ctx.dex.species[i].catch_rate < ctx.p.hunt_min_catch),
        );
    }
    skip
}

/// A Safari Zone hunt under the plan's Safari model (`hunt`, `run_encounters`).
pub fn is_safari_hunt(ctx: &Ctx, h: &Hunt) -> bool {
    ctx.p.safari_model && h.site.starts_with("MAP_SAFARI_ZONE")
}

/// `hunt`'s cap on new species this visit: `stop_after`, tightened by what is left of
/// `safari_stop` in the zone (`safari_new` = new species caught there so far). 0 = no cap.
pub fn hunt_stop_new(ctx: &Ctx, h: &Hunt, safari_new: u32) -> u32 {
    let mut stop_new = h.stop_after;
    if is_safari_hunt(ctx, h) && ctx.p.safari_stop > 0 {
        let left = ctx.p.safari_stop.saturating_sub(safari_new);
        stop_new = if stop_new == 0 {
            left
        } else {
            stop_new.min(left)
        };
    }
    stop_new
}

/// `run_encounters`' stop rule (`stop_when_done`): every slot of the site is owned or skipped.
/// The pre-fight PP rule (shared with the playing controller): low on attacking PP before a
/// trainer fight means fewer than 12 PP in all; before a League fight also any empty slot, or a
/// scarce slot (base PP <= 5: Blizzard, Fire Blast) below 3. `moves` are dex move indices.
pub fn pp_low_before_fight(dex: &Dex, league: bool, moves: &[usize], pp: &[i32]) -> bool {
    let scarce_low = moves
        .iter()
        .zip(pp)
        .any(|(&m, &p)| dex.moves[m].pp <= 5 && dex.moves[m].power > 0 && p < 3);
    pp.iter().sum::<i32>() < 12 || (league && (pp.contains(&0) || scarce_low))
}

pub fn site_done(ctx: &Ctx, slots: &[Slot], owned: &dyn Fn(u32) -> bool, skip: &[usize]) -> bool {
    slots.iter().all(|s| {
        let sp = &ctx.dex.species[s.species];
        sp.dex == 0 || owned(sp.dex) || skip.contains(&s.species)
    })
}

#[allow(clippy::too_many_arguments)]
/// One wild encounter on arrival: catch if it is a new Kanto species the human wants, else
/// flee or fight. `skip` lists species left alone.
fn encounter(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    slot: &Slot,
    level: u32,
    skip: &[usize],
    want_catch: bool,
    safari: bool,
) {
    let sp = &ctx.dex.species[slot.species];
    bump(st, ctx.enc_intro() + react(ctx, rng) as f64);
    let wanted = wants_catch(ctx, slot.species, st.caught.has(sp.dex), skip, want_catch);
    if wanted && safari {
        if safari_catch(ctx, st, rng, sp) {
            got(st, sp.dex, level);
        }
    } else if wanted {
        let mut hp = 1.0;
        if ctx.p.weaken {
            let turns = 1 + rng.chance(0.5) as u32;
            bump(st, turns as f64 * ctx.turn());
            if rng.chance(ctx.k("weaken_ko", 0.05) * turns as f64) {
                return;
            }
            hp = ctx.k("weaken_hp", 0.3);
        }
        let tier = ball_tier(ctx, sp.catch_rate);
        st.ball_tier = Some(tier);
        let ball = (!ctx.p.money_model).then(|| BALL_MULT[tier]);
        // PLAN `low_chance_throws`: fewer throws at a mon the first ball catches rarely.
        let max_throws = if ctx.p.low_chance_throws > 0
            && catch_prob(catch_odds(
                sp.catch_rate,
                BALL_MULT[tier.min(st.max_tier)],
                (hp * 100.0) as u32,
                100,
                Status1::None,
            )) < ctx.p.low_chance_below
        {
            ctx.p.low_chance_throws
        } else {
            MAX_THROWS
        };
        let ok = throw_balls(
            ctx,
            st,
            rng,
            sp.catch_rate,
            hp,
            Status1::None,
            ball,
            max_throws,
        );
        st.ball_tier = None;
        if std::env::var_os("FRLG_CATCH_TRACE").is_some() {
            let seg = ctx.segs.get(st.seg_i).map_or("?", |s| s.name.as_str());
            let res = if ok { "caught" } else { "missed" };
            eprintln!(
                "catch {seg} {} {res} balls {:?} seq {}",
                sp.name, st.balls, st.catch_n
            );
        }
        if ok {
            got(st, sp.dex, level);
        }
    } else if fights_wild(ctx, st.carry, st.hp_frac) && ctx.p.battle_sim {
        let foe = wild_mon(ctx, st, slot.species, level);
        // A lost wild fight is a whiteout like any other; the human does not give up the run over it.
        sim_battle(
            ctx,
            st,
            rng,
            &BattleArgs {
                name: "wild",
                foes: &[foe],
                trainer: false,
                intro: 0,
                ai: 0,
                heals: 0,
                yield_: None,
                league: false,
                wild: true,
            },
        );
    } else if fights_wild(ctx, st.carry, st.hp_frac) {
        let e = exp_gain(sp.exp_yield as u8, level as u8, false);
        let t = turns_per_mon(level as f64, effective_level(st, ctx.p), st.strength, 1.0);
        bump(st, t * ctx.turn() + ctx.mon_overhead());
        let g = add_exp(ctx, st, e);
        bump(st, g as f64 * ctx.level_up());
    } else if safari {
        // Safari battles have no Run roll and a two-row menu: leaving an unwanted
        // encounter is one press (`decompiled/src/battle_script_commands.c`, Safari
        // flee always succeeds). 200 frames is the fill audit's estimate (guess); MODEL CHANGE
        // `safari_run_frames`: played 100 (calibration 2026-11-22).
        let f = if ctx.p.safari_run_frames > 0 {
            ctx.p.safari_run_frames as f64
        } else {
            200.0
        };
        bump(st, f);
    } else {
        flee(ctx, st, rng);
    }
}

/// Running from a wild battle. Measured on libmgba with a level-6 starter in Viridian Forest:
/// a successful Run costs 400-1013 frames from the menu (mean 598 with retries), a failed one
/// 277-585 (mean 374); 10 of 54 attempts worked. The escape formula guarantees an escape when the
/// player is faster, which a carry past level 15 nearly always is vs a wild mon of the area
/// (`decompiled/src/battle_main.c`, `TryRunFromBattle`). guess for the 15 and the 0.2.
fn flee(ctx: &Ctx, st: &mut State, rng: &mut Rng) {
    if !ctx.p.calib_enc {
        bump(st, FLEE as f64);
        return;
    }
    let p_ok = if st.carry < 15 { 0.2 } else { 1.0 };
    // Measured 400-1025 per successful Run and 277-596 per failure (`calibration.md`); the WR-level
    // factor (~1.06) over the floors, not the 1.25 of the default human.
    let (ok_cost, fail_cost) = if ctx.p.calib_wr {
        (430.0, 400.0)
    } else {
        (500.0, 470.0)
    };
    // MODEL CHANGE `flee_frames` (played Runs at carry L36, calibration 2026-11-22).
    let ok_cost = if ctx.p.flee_frames > 0 {
        ctx.p.flee_frames as f64
    } else {
        ok_cost
    };
    loop {
        if rng.chance(p_ok) {
            bump(st, ok_cost);
            return;
        }
        bump(st, fail_cost);
    }
}

fn turns_per_mon(le: f64, lp: f64, strength: f64, adv: f64) -> f64 {
    ((1.0 + 3.0 * (le / lp - 0.7).max(0.0)) / (strength * adv)).max(1.0)
}

/// A wild mon with the game's generation: nature from the PID (`Random32() % 25`),
/// IVs from two 16-bit rolls in 5-bit fields (`decompiled/src/pokemon.c`, `CreateBoxMon`:
/// personality at `:1793`, IVs at `:1834-1853`), default level-up moves.
fn wild_mon(ctx: &Ctx, st: &mut State, species: usize, level: u32) -> crate::battle::Mon {
    let mut g = st.game;
    let pid = (g.roll() as u32) | ((g.roll() as u32) << 16);
    let iv1 = g.roll() as u32;
    let iv2 = g.roll() as u32;
    st.game = g;
    let ivs = [
        iv1 & 0x1F,
        (iv1 >> 5) & 0x1F,
        (iv1 >> 10) & 0x1F,
        iv2 & 0x1F,
        (iv2 >> 5) & 0x1F,
        (iv2 >> 10) & 0x1F,
    ];
    let nature = (pid % 25) as u8;
    let mut foe = crate::battle::make_mon(ctx.dex, species, level, ivs, [0; 6], nature, 0);
    foe.moves = ctx.dex.default_moves(species, level);
    foe
}

/// Opening the bag and using a Repel. guess.
const REPEL_USE: f64 = 350.0;

/// Tiles of a walk covered by Repels, using new ones as the old run out. Pays for them when the
/// money model is on; stops when it cannot.
fn use_repels(ctx: &Ctx, st: &mut State, seg: &Seg, tiles: u32) -> u32 {
    if !st.repel_ok || !ctx.p.repel_on.contains(&seg.name) {
        return 0;
    }
    let (mut covered, mut left) = (0, tiles);
    while left > 0 {
        if st.repel_left == 0 {
            let (dur, price) = match st.max_tier {
                0 => (100, 350),
                1 => (200, 500),
                _ => (250, 700),
            };
            if ctx.p.money_model {
                if st.money < price {
                    break;
                }
                st.money -= price;
            }
            bump(st, REPEL_USE * ctx.p.text_factor);
            st.repel_left = dur;
            if std::env::var_os("FRLG_REPEL_TRACE").is_some() {
                eprintln!("repel-use {}", seg.name);
            }
        }
        let take = left.min(st.repel_left);
        st.repel_left -= take;
        left -= take;
        covered += take;
    }
    covered
}

struct EncArgs<'a> {
    site: &'a str,
    area: Area,
    steps: u32,
    era: Era,
    skip: &'a [usize],
    stop_when_done: bool,
    /// A Repel is active: encounters below the lead's level never start.
    repel: bool,
    /// Lead level the Repel compares against (0 = the carry's level).
    repel_lvl: u32,
    /// Stop after this many new species were caught in this call (0 = no limit).
    stop_new: u32,
    /// A passing encounter may be caught (`catch_on_pass`); false on segments the plan lists in
    /// `pass_off`.
    pass_catch: bool,
}

fn run_encounters(ctx: &Ctx, st: &mut State, rng: &mut Rng, a: &EncArgs) -> u32 {
    let EncArgs {
        site,
        area,
        steps,
        era,
        skip,
        stop_when_done,
        repel,
        repel_lvl,
        stop_new,
        pass_catch,
    } = *a;
    let c0 = st.caught.count();
    let Some((rate, slots, weights)) = ctx.dex.area(site, area, ctx.p.version == "LG", &ctx.p.rod)
    else {
        return 0;
    };
    let mut p = rate as f64 * 16.0 / 1600.0;
    if era == Era::Bike {
        p *= 0.8;
    }
    // Encounter cooldown (`decompiled/src/wild_encounter.c:673-699,707-755`): after a map
    // entry and after every battle, the first `8 - rate/10` encounter-eligible steps only
    // reach the dice through a 5% gate. Without this the model averaged ~17 encounters
    // before Brock where the WR video shows 2-4 (prebrock audit, 2026-11-13).
    let cd_steps = if rate >= 80 {
        0
    } else if rate < 10 {
        8
    } else {
        8 - (rate as u64) / 10
    };
    let mut cooldown = cd_steps;
    let mut left = steps as u64;
    let mut used = 0u32;
    let safari_site = ctx.p.safari_model && site.starts_with("MAP_SAFARI_ZONE");
    loop {
        if stop_when_done && site_done(ctx, &slots, &|d| st.caught.has(d), skip) {
            break;
        }
        // MODEL CHANGE `hunt_stop_empty`: no ball left and no mart inside a hunt: the human stops hunting.
        if stop_when_done
            && ctx.p.hunt_stop_empty
            && ctx.p.money_model
            && !safari_site
            && st.balls.iter().sum::<u32>() == 0
            && (!ctx.p.marts.is_empty() || ctx.ball_shop_from.is_some_and(|f| st.seg_i < f))
        {
            break;
        }
        // Steps inside the cooldown window roll at 5% of the rate; afterwards at full rate.
        let gap = {
            let g1 = rng.geometric(p * 0.05);
            if g1 < cooldown {
                g1
            } else {
                cooldown + rng.geometric(p)
            }
        };
        if gap >= left {
            quiet_account(st, left, cooldown);
            used += left as u32;
            break;
        }
        let quiet_hit = quiet_account(st, gap + 1, cooldown);
        left -= gap + 1;
        used += gap as u32 + 1;
        cooldown = cd_steps;
        let slot = &slots[pick_slot(rng, &weights)];
        let level = rng.range(slot.min, slot.max);
        if repel && level < if repel_lvl > 0 { repel_lvl } else { st.carry } {
            continue;
        }
        let want_catch = stop_when_done
            || (ctx.p.catch_on_pass
                && ctx.dex.species[slot.species].catch_rate >= ctx.p.pass_min_catch);
        if quiet_hit == Some(true) {
            // A quiet table seed: this roll is a miss (`manip.rs`, doc 3.4). With `quiet_keeps_wanted`
            // the table also picks the species, so an encounter the human wants (a new species he
            // would throw a ball at) still happens; only the unwanted ones are suppressed.
            let sp = &ctx.dex.species[slot.species];
            let wanted = want_catch
                && sp.dex != 0
                && !st.caught.has(sp.dex)
                && !skip.contains(&slot.species);
            if !(ctx.p.manip.quiet_keeps_wanted && wanted) {
                continue;
            }
        }
        let safari = ctx.p.safari_model && site.starts_with("MAP_SAFARI_ZONE");
        let rate_s = ctx.dex.species[slot.species].catch_rate;
        // MODEL CHANGE `pass_reserve`: a hard passing catch leaves the last balls for the next hunt.
        let reserve_ok = stop_when_done
            || safari
            || ctx.p.pass_reserve == 0
            || !ctx.p.money_model
            || rate_s >= ctx.p.pass_reserve_rate
            || st.balls.iter().sum::<u32>() > ctx.p.pass_reserve;
        encounter(
            ctx,
            st,
            rng,
            slot,
            level,
            skip,
            stop_when_done
                || (pass_catch
                    && reserve_ok
                    && ctx.p.catch_on_pass
                    && rate_s >= ctx.p.pass_min_catch),
            safari,
        );
        if stop_new > 0 && st.caught.count() - c0 >= stop_new {
            break;
        }
        if stop_when_done && left == 0 {
            break;
        }
    }
    used
}

/// Type-matchup multiplier on the fighter's effective level against one enemy species: its best
/// damaging type against the enemy over a neutral baseline, divided by the root of the enemy's
/// best STAB against it. Move power, accuracy and AI choices are ignored. guess.
fn matchup(ctx: &Ctx, st: &State, enemy: usize) -> f64 {
    if ctx.p.type_strength == 0.0 {
        return 1.0;
    }
    let e = ctx.dex.species[enemy].types;
    let dex = ctx.dex;
    let (mine, off, neutral) = if st.power > st.carry as f64 * ctx.p.team_bonus {
        // Mewtwo: Psychic STAB plus TM coverage.
        let off = (1.5 * dex.eff(14, e))
            .max(dex.eff(15, e))
            .max(dex.eff(13, e))
            .max(dex.eff(10, e));
        ((14, 14), off, 1.5)
    } else {
        let stage = (st.carry >= ctx.evo_at[0]) as usize + (st.carry >= ctx.evo_at[1]) as usize;
        let mine = dex.species[ctx.starter_line[stage]].types;
        let normal = dex.eff(0, e);
        if st.carry >= ctx.stab_from {
            let mut stab = dex.eff(mine.0, e);
            if stage > 0 {
                stab = stab.max(dex.eff(mine.1, e));
            }
            ((mine.0, mine.1), (1.5 * stab).max(normal), 1.5)
        } else {
            (mine, normal, 1.0)
        }
    };
    let inc = dex.eff(e.0, mine).max(dex.eff(e.1, mine)).clamp(0.5, 2.0);
    ((off / neutral).clamp(0.5, 2.0) / inc.sqrt())
        .clamp(0.5, 2.0)
        .powf(ctx.p.type_strength)
        .max(ctx.p.type_floor)
}

fn enemy_list(ctx: &Ctx, st: &State, party: &[(u32, usize)]) -> Vec<(u32, u32, f64)> {
    party
        .iter()
        .map(|&(l, s)| (l, ctx.dex.species[s].exp_yield, matchup(ctx, st, s)))
        .collect()
}

fn p_lose(r: f64, center: f64, slope: f64) -> f64 {
    1.0 / (1.0 + (-(r - center) / slope).exp())
}

/// A set of enemy mons fought in sequence by the best fighter. Returns false if the human
/// gives up (whited out too often).
fn battle(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    enemies: &[(u32, u32, f64)],
    trainer: bool,
    intro: u32,
    adv: f64,
) -> bool {
    let lp0 = effective_level(st, ctx.p);
    let lp_of = |a: f64| lp0 * (a * adv).sqrt();
    let mut r = enemies
        .iter()
        .map(|e| e.0 as f64 / lp_of(e.2))
        .fold(0.0, f64::max);
    if r > 0.9 && st.power <= st.carry as f64 * ctx.p.team_bonus && st.carry >= ctx.stab_from {
        r /= 1.0 + ctx.p.ability_bonus;
    }
    let mut tries = 0;
    loop {
        let start = st.clock;
        bump(st, ctx.trainer_intro(intro));
        bump(st, jitter(ctx, rng, 8) as f64);
        let mut total_exp = 0u32;
        for &(lvl, yld, a) in enemies {
            let mean = turns_per_mon(lvl as f64, lp_of(a), st.strength, 1.0);
            let mut turns = mean.floor() + rng.chance(mean - mean.floor()) as u32 as f64;
            for _ in 0..turns as u32 {
                if rng.chance(ctx.k("p_event", P_EVENT)) {
                    bump(st, react(ctx, rng) as f64);
                    if rng.chance(0.5) {
                        turns += 1.0;
                    }
                }
            }
            bump(st, turns * ctx.turn() + ctx.mon_overhead());
            bump(st, jitter(ctx, rng, turns as u32 * 2 + 3) as f64);
            total_exp += exp_gain(yld as u8, lvl as u8, trainer);
        }
        let heal_turns = ((r - 0.95).max(0.0) * enemies.len() as f64 * 2.0).round();
        bump(st, heal_turns * ctx.turn());
        if trainer {
            bump(st, ctx.outro());
        }
        let danger = p_lose(r, ctx.k("p_lose_center", 1.25), ctx.k("p_lose_slope", 0.05))
            * 0.5f64.powi(tries);
        if rng.chance(danger) {
            let spent = st.clock - start;
            st.whiteouts += 1;
            bump(st, ctx.k("whiteout", WHITEOUT as f64) + spent as f64 * 0.0);
            tries += 1;
            if tries >= 3 {
                st.abandoned = true;
                return false;
            }
            continue;
        }
        let g = add_exp(ctx, st, total_exp);
        bump(st, g as f64 * ctx.level_up());
        return true;
    }
}

/// A TM/HM or tutor move the species can be taught (`tmhm_learnsets.h`,
/// `tutor_learnsets.h`).
pub(crate) fn can_learn(ctx: &Ctx, species: usize, mv: usize) -> bool {
    let sp = &ctx.dex.species[species];
    sp.tmhm.contains(&mv) || sp.tutor.contains(&mv)
}

/// The carry's species at `level` (`starter_line`, evolving at `evo_at`).
fn carry_species(ctx: &Ctx, level: u32) -> usize {
    let stage = (level >= ctx.evo_at[0]) as usize + (level >= ctx.evo_at[1]) as usize;
    ctx.starter_line[stage]
}

/// The pre-fight heal rule of the Torrent phase and of `entry` fights (`sim_battle`): the carry starts
/// the fight with at least the foes' best hit (after one Withdraw, `battle::entry_floor`) times
/// `torrent_hits` (1 for `entry` fights) plus one HP; below that the human drinks potions, above it
/// nothing is done (the Torrent window, at most 1/3 of the max, is kept). The playing controller
/// asks the same function before a fight (`play/early.rs`).
pub fn entry_heal_floor(
    ctx: &Ctx,
    entry: bool,
    dex: &crate::data::Dex,
    me: &crate::battle::Mon,
    foes: &[crate::battle::Mon],
) -> i32 {
    let hits = if entry { 1.0 } else { ctx.p.torrent_hits };
    (crate::battle::entry_floor(dex, me, foes) as f64 * hits).ceil() as i32 + 1
}

/// PLAN `early_torrent.steer_poison`: the HP a poisoned carry is walked down to before the steered
/// fight: the top of the Torrent window, `max / 3` (`decompiled/src/pokemon.c:2500`), but never below
/// the foes' best max roll after one Withdraw plus one (`battle::entry_floor`). The playing controller
/// walks to the same number (`play/early.rs`).
pub fn early_torrent_target(
    dex: &crate::data::Dex,
    me: &crate::battle::Mon,
    foes: &[crate::battle::Mon],
) -> i32 {
    let lo = crate::battle::entry_floor(dex, me, foes) + 1;
    (me.max_hp / 3).max(lo)
}

/// A wild encounter the human does not catch is fought for exp (not fled) while the carry is below
/// `fight_until_level`, or always with `fight_wild`, and only while the carry's HP fraction `hp` is at
/// least `wild_fight_floor` (the rule of `run_encounters`; the playing controller asks the same
/// function, `play/dex.rs`).
pub fn fights_wild(ctx: &Ctx, carry: u32, hp: f64) -> bool {
    (ctx.p.fight_wild || carry < ctx.p.fight_until_level) && hp >= ctx.p.wild_fight_floor
}

/// One `battle_sim` fight, described.
/// The Torrent phase of the plan (`Params::torrent_until`) is on at segment `seg_i`, unless a heal took
/// the carry out of the window (`State::torrent_lost`).
pub fn in_torrent_phase(ctx: &Ctx, seg_i: usize, torrent_lost: bool) -> bool {
    ctx.torrent_end.is_some_and(|e| seg_i <= e) && !torrent_lost
}

/// What a fight's tactics depend on besides the plan: the simulator's attempt state, or the real
/// game's party and bag in the playing controller (`play/battle.rs`).
pub struct TacIn<'a> {
    /// Index of the segment the fight belongs to (`Ctx::segs`), for the Torrent phase.
    pub seg_i: usize,
    pub torrent_lost: bool,
    /// The fighter's level and known moves (set-up moves are queued only when known).
    pub level: u32,
    pub moves: &'a [usize],
    /// Full Restores are the heal item (Indigo Plateau reached).
    pub league_items: bool,
    /// X items in the bag: Attack, Defend, Speed, Special, Dire Hit, Accuracy, Guard Spec.
    pub x_stock: [u32; 7],
    /// Living expendable teammates now, and when the fight began (the sacrifice budget is capped by
    /// the latter; the simulator retries a fight after a whiteout with the same cap).
    pub fodder: u32,
    pub fodder_at_start: u32,
    /// Heal items for the League round in progress.
    pub heal_items: u32,
    pub mewtwo_now: bool,
    /// Awakenings in the bag (`Tactics::awakenings`).
    pub awakenings: u32,
    /// Poison cures in the bag (`Tactics::cures`).
    pub cures: u32,
}

/// The Indigo mart at a League round's start (and after a League whiteout with `league_rebuy`): the
/// bag is topped up to `league_heals` heal items out of the purse. With `league_bag_honest` the Full Heals
/// (`league_full_heals`, 600) come first and the heals are `league_heal_item` at its `items.json` price
/// (the mart, `IndigoPlateau_PokemonCenter_1F/scripts.inc:54-60`, sells no Hyper Potion). Returns the
/// money spent.
fn league_topup(ctx: &Ctx, st: &mut State) -> u32 {
    let before = st.money;
    let price_of = |item: &str, dflt: u32| ctx.dex.item_prices.get(item).copied().unwrap_or(dflt);
    let honest = ctx.p.league_bag_honest;
    if honest && ctx.p.league_full_heals > st.lg_fh {
        let price = price_of("ITEM_FULL_HEAL", 600);
        let want = ctx.p.league_full_heals - st.lg_fh;
        let n = if ctx.p.money_model {
            want.min(st.money / price)
        } else {
            want
        };
        if ctx.p.money_model {
            st.money -= n * price;
        }
        st.lg_fh += n;
    }
    let max_potion = honest && ctx.p.league_heal_item == "max_potion";
    let price = if !honest {
        ctx.p.league_heal_price.max(1)
    } else if max_potion {
        price_of("ITEM_MAX_POTION", 2500)
    } else {
        price_of("ITEM_FULL_RESTORE", 3000)
    };
    let want = ctx.p.league_heals.saturating_sub(st.heal_items);
    let n = if ctx.p.money_model {
        want.min(st.money / price)
    } else {
        want
    };
    if ctx.p.money_model {
        st.money -= n * price;
    }
    st.heal_items += n;
    if max_potion {
        st.lg_max += n;
    } else {
        st.lg_fr += n;
    }
    if std::env::var_os("FRLG_SIM_TRACE").is_some() {
        eprintln!(
            "LEAGUE topup seg {} money {} -> {} bought {} want {} have {} fr {} max {} fh {}",
            st.seg_i, before, st.money, n, want, st.heal_items, st.lg_fr, st.lg_max, st.lg_fh
        );
    }
    before - st.money
}

/// One League heal item spent between fights (`league_preheal`): with `league_bag_honest` a Hyper Potion
/// (200 HP), else a Max Potion, else a Full Restore; the old pool heals in full. Returns the HP fraction
/// after it.
fn league_preheal_item(ctx: &Ctx, st: &mut State, cur: f64, max_hp: i32) -> f64 {
    st.heal_items -= 1;
    if !ctx.p.league_bag_honest {
        return 1.0;
    }
    let hypers = st.heal_items + 1 - st.lg_fr.min(st.heal_items + 1) - st.lg_max;
    if hypers > 0 {
        let hp = (max_hp as f64 * cur).round() as i32 + 200;
        (hp as f64 / max_hp as f64).min(1.0)
    } else if st.lg_max > 0 {
        st.lg_max -= 1;
        1.0
    } else {
        st.lg_fr = st.lg_fr.saturating_sub(1);
        1.0
    }
}

/// The [`Tactics`] of the fight named `name` (`Params::fight_tactics`, the global knobs): the one
/// place the simulator ([`sim_battle`]) and the playing controller build them.
pub fn fight_tactics(ctx: &Ctx, name: &str, league: bool, t: &TacIn) -> crate::battle::Tactics {
    let cfg = ctx.tactics.get(name).cloned().unwrap_or_default();
    let in_torrent = in_torrent_phase(ctx, t.seg_i, t.torrent_lost);
    let torrent = cfg.torrent || in_torrent;
    let heal_below = if cfg.heal_below >= 0.0 {
        cfg.heal_below
    } else if in_torrent {
        ctx.p.torrent_heal_below
    } else {
        ctx.p.heal_below
    };
    // X items limited by the bag.
    let mut stock = t.x_stock;
    let xs: Vec<XItem> = cfg
        .x
        .iter()
        .filter(|&&x| {
            if let Some(m) = x.setup_move(ctx.dex) {
                // A move, not stock: queued only for a fighter that knows it.
                return t.moves.contains(&m);
            }
            let i = x_index(x);
            if stock[i] > 0 {
                stock[i] -= 1;
                true
            } else {
                false
            }
        })
        .copied()
        .collect();
    crate::battle::Tactics {
        x_items: xs,
        heal_below,
        heal: potion_hp(ctx, t.seg_i, t.level, t.league_items),
        full_restore: t.league_items,
        sacs: cfg.sacs.min(t.fodder_at_start),
        sac_free: !(ctx.p.sac_honest && (!league || ctx.p.sac_honest_league)),
        helper_hp: ctx.p.helper_hp,
        torrent: (torrent || (league && ctx.p.league_torrent)) && !t.mewtwo_now,
        // PLAN `early_torrent`: the steered fight is fought from inside the window (no stall turns) and
        // with Withdraw first against a faster foe.
        torrent_stall: ctx.p.torrent_stall
            && !(ctx.p.early_torrent.no_stall && ctx.p.early_torrent.fight == name),
        torrent_early: ctx.p.torrent_early
            || (ctx.p.early_torrent.withdraw_first && ctx.p.early_torrent.fight == name),
        others_alive: t.fodder,
        heal_stock: if league { t.heal_items } else { u32::MAX },
        rollouts: if league { ctx.p.league_rollouts } else { 0 },
        setup_min: ctx.p.setup_min_turns,
        menu_slip: ctx.p.menu_slip,
        scarce: ctx.p.scarce_factor,
        sleep_no_potion: ctx.p.sleep_no_potion,
        awakenings: t.awakenings,
        cures: t.cures,
        // `league_bag_honest` is set by the simulator's League fight (`sim_battle`); the controller reads its
        // real bag (`play::battle::tactics_for`).
        bag_honest: false,
        fr_stock: u32::MAX,
        max_stock: 0,
        full_heals: 0,
    }
}

/// MODEL CHANGE `tcost_whole`: a non-League trainer fight priced by `trainer_cost` at the `turn_power` copy's
/// actions is the whole battle (fitted on the WR's battle times, which hold its reactions and heals).
fn whole_price(ctx: &Ctx, a: &BattleArgs) -> bool {
    ctx.p.tcost_whole
        && a.trainer
        && !a.league
        && ctx.p.turn_power > 0.0
        && ctx.p.trainer_cost.len() == 3
}

struct BattleArgs<'a> {
    /// Segment name, for the per-fight tactics lookup.
    name: &'a str,
    foes: &'a [crate::battle::Mon],
    trainer: bool,
    intro: u32,
    /// The trainer's AI bits and heal items (`trainers.h`).
    ai: u32,
    heals: u32,
    /// Exp-yield override (the guessed trainer groups).
    yield_: Option<u32>,
    league: bool,
    wild: bool,
}

/// `battle_sim` fight against `a.foes` with the exact engine. Returns false when the
/// human gives up (three whiteouts, or one in the League where a loss means walking the
/// rooms again).
fn sim_battle(ctx: &Ctx, st: &mut State, rng: &mut Rng, a: &BattleArgs) -> bool {
    use crate::battle::{fight, make_mon, Status1};
    let dex = ctx.dex;
    let t0 = st.clock;
    st.free_heal = false;
    if a.trainer && hazard_death(ctx, st, rng, a.name) {
        return false;
    }
    // Non-League trainer fights run `trainer_fit` x the fitted intro and per-mon overhead.
    let fit = if a.trainer && !a.league && ctx.p.trainer_fit > 0.0 {
        ctx.p.trainer_fit
    } else {
        1.0
    };
    // MODEL CHANGE `trainer_cost`: the played run's per-battle / per-action / per-KO frames.
    let tcost = (a.trainer && !a.league && ctx.p.trainer_cost.len() == 3).then(|| {
        let c = &ctx.p.trainer_cost;
        (c[0], c[1], c[2])
    });
    let fit = if tcost.is_some() { 1.0 } else { fit };
    if a.trainer {
        match tcost {
            // The outro is paid at the end as usual: `a` covers both.
            Some((a0, _, _)) => bump(st, (a0 - ctx.outro()).max(0.0)),
            None => bump(st, ctx.trainer_intro(a.intro) * fit),
        }
        let j = jitter(ctx, rng, 8) as f64;
        bump(st, if whole_price(ctx, a) { 0.0 } else { j });
    }
    let cfg = ctx.tactics.get(a.name).cloned().unwrap_or_default();
    let sacs = cfg.sacs;
    let in_torrent = in_torrent_phase(ctx, st.seg_i, st.torrent_lost);
    let torrent = cfg.torrent || in_torrent;
    // Sacrifices burn real party members; the budget spans a Centre-less chain, with
    // Revives (bag trip each) bringing fodder back between fights.
    while st.fodder < sacs && st.revives > 0 {
        st.revives -= 1;
        st.fodder += 1;
        bump(st, 450.0 * ctx.p.text_factor);
    }
    // Early helpers (`Params::early_helpers`): the mons caught so far (the forest bug, a Pidgey) are the
    // sacrifices of the early fights; each one that faints stays fainted.
    let helper_end = ctx.no_salvage_end.or(ctx.risk_end);
    if ctx.p.early_helpers && !a.league && helper_end.is_some_and(|e| st.seg_i <= e) {
        let alive: u32 = st
            .bodies
            .iter()
            .map(|&b| b as u32)
            .sum::<u32>()
            .saturating_sub(1);
        st.fodder = alive.saturating_sub(st.helpers_dead);
    }
    // League round start: the heal items come out of the purse (Indigo Plateau mart, 600
    // frames of menus; guess).
    if a.league && (a.name == "lorelei" || a.name == "lorelei-2") {
        league_topup(ctx, st);
        st.mewtwo_hp = 1.0;
        bump(st, 600.0 * ctx.p.text_factor);
    }
    // Low on attacking PP before a trainer fight: drink an Elixir (bag trip ~500f).
    // Before a League fight any empty slot is worth topping up — the next Centre is
    // several fights away.
    // A scarce slot (base PP <= 5: Blizzard, Fire Blast) below 3 PP counts as empty before a
    // League fight: the human tops up with the Max Elixir he kept for exactly this
    // ("Max Elixir between E4 fights", `STRATEGY.md` section 4).
    // With `mewtwo_pp` the fighter after the catch is Mewtwo: its own slots and PP are checked.
    let mw_pp = ctx.p.mewtwo_pp && ctx.p.mewtwo && st.mewtwo.is_some() && !a.wild;
    let (pp_moves, pp_vals): (Vec<usize>, Vec<i32>) = if mw_pp {
        (st.mewtwo_moves.clone(), st.mewtwo_pp.clone())
    } else {
        (st.slots.clone(), st.slot_pp.clone())
    };
    // `mewtwo_pp_floor`: before a League fight the human tops Mewtwo up when its Psychic PP is
    // below this (a fight spends about 3-6 of them).
    let psychic_low = mw_pp
        && a.league
        && ctx.p.mewtwo_pp_floor > 0
        && pp_moves
            .iter()
            .zip(&pp_vals)
            .any(|(&m, &p)| m == ctx.dex.move_id("MOVE_PSYCHIC") && p < ctx.p.mewtwo_pp_floor);
    let pp_low = psychic_low || pp_low_before_fight(ctx.dex, a.league, &pp_moves, &pp_vals);
    if a.trainer && pp_low {
        // Outside the League the last `league_max_reserve` Max Elixirs stay in the bag.
        let reserve = if a.league {
            0
        } else {
            ctx.p.league_max_reserve
        };
        let low_slots = if mw_pp {
            (0..pp_moves.len())
                .filter(|&k| pp_vals[k] * 2 < mewtwo_full_pp(ctx, st, k))
                .count()
        } else {
            0
        };
        let ether_first = mw_pp && low_slots <= 1 && (st.max_ethers > 0 || st.ethers > 0);
        if ether_first && use_ether(ctx, st) {
            // one slot topped up with an Ether / Max Ether (bag trip ~450f)
        } else if st.max_elixirs > reserve {
            st.max_elixirs -= 1;
            bump(st, 500.0 * ctx.p.text_factor);
            for k in 0..pp_moves.len() {
                if mw_pp {
                    st.mewtwo_pp[k] = mewtwo_full_pp(ctx, st, k);
                } else {
                    st.slot_pp[k] = carry_full_pp(ctx, st, k);
                }
            }
        } else if st.elixirs
            > if a.league {
                0
            } else {
                ctx.p.league_elixir_reserve
            }
        {
            st.elixirs -= 1;
            bump(st, 500.0 * ctx.p.text_factor);
            for k in 0..pp_moves.len() {
                let full = if mw_pp {
                    mewtwo_full_pp(ctx, st, k)
                } else {
                    carry_full_pp(ctx, st, k)
                };
                let v = if mw_pp {
                    &mut st.mewtwo_pp[k]
                } else {
                    &mut st.slot_pp[k]
                };
                *v = (*v + 10).min(full);
            }
        } else if mw_pp && (st.max_ethers > 0 || st.ethers > 0) {
            use_ether(ctx, st);
        }
    }
    let fodder_at_start = st.fodder;
    let mut tries = 0;
    loop {
        // Mewtwo takes over once it is caught; otherwise the starter line carries.
        let mewtwo_now = ctx.p.mewtwo && st.mewtwo.is_some() && !a.wild;
        let mut me = if let (true, Some((ivs, nature))) = (mewtwo_now, st.mewtwo) {
            let mut m = make_mon(
                dex,
                dex.id("SPECIES_MEWTWO"),
                mewtwo_level(ctx, st),
                ivs,
                st.mewtwo_evs,
                nature,
                st.badges,
            );
            m.moves = st.mewtwo_moves.clone();
            if ctx.p.mewtwo_pp {
                m.pp = st.mewtwo_pp.clone();
            }
            if ctx.p.mewtwo_power != 1.0 {
                m.stat[0] = (m.stat[0] as f64 * ctx.p.mewtwo_power) as i32;
                m.stat[3] = (m.stat[3] as f64 * ctx.p.mewtwo_power) as i32;
            }
            m
        } else {
            let mut m = make_mon(
                dex,
                carry_species(ctx, st.carry),
                st.carry,
                st.ivs,
                st.evs,
                st.nature,
                st.badges,
            );
            m.moves = st.slots.clone();
            m.pp = st.slot_pp.clone();
            tackle_only_mask(ctx, a.name, &mut m);
            m.status = st.carry_status;
            if ctx.p.ablate_torrent {
                m.ability = crate::data::Ability::None;
            }
            m.force_torrent = cfg.free_torrent;
            m.persim = st.persim;
            if st.mystic {
                m.boost = Some((11, 10));
            } else if st.ice {
                m.boost = Some((15, 10));
            } else if ctx.p.black_glasses && st.glasses && !st.leftovers {
                m.boost = Some((17, 10));
            }
            m.leftovers = st.leftovers;
            if a.league && std::env::var_os("FRLG_HACK_PP").is_some() {
                m.pp = vec![50; m.moves.len()];
            }
            // `carry_power`: the WR carry out-damages the engine's (see the param's doc).
            let cp = if a.league && ctx.p.league_power > 0.0 {
                ctx.p.league_power
            } else if ctx.power_early_end.is_some_and(|e| st.seg_i <= e) {
                ctx.p.carry_power_early
            } else {
                ctx.p.carry_power
            };
            let cp = cp * cfg.power;
            if cp != 1.0 {
                m.stat[0] = (m.stat[0] as f64 * cp) as i32;
                m.stat[3] = (m.stat[3] as f64 * cp) as i32;
            }
            m
        };
        // PLAN `early_torrent` (review item 11): a poisoned carry is walked down into the Torrent window
        // before the steered fight (Main97's Brock line, `STRATEGY.md` 359-370).
        st.steer_to = None;
        let et = &ctx.p.early_torrent;
        if tries == 0
            && !mewtwo_now
            && in_torrent
            && et.steer_poison
            && a.name == et.fight
            && matches!(st.carry_status, Status1::Poison | Status1::Toxic(_))
        {
            let target = early_torrent_target(dex, &me, a.foes);
            let tf = target as f64 / me.max_hp as f64;
            st.steer_to = Some(tf);
            let cur = (me.max_hp as f64 * st.hp_frac).round() as i32;
            st.steer_hp = 0;
            if cur > target {
                st.steer_hp = cur - target;
                clock_add(st, (cur - target) as f64 * ctx.p.poison_frames_per_hp);
                let before = st.hp_frac;
                st.hp_frac = tf;
                // The lane is steered on its own in `risk_fight` (to the same target): no sync of this drop.
                st.risk_real += st.hp_frac - before;
            }
            if et.cure {
                st.carry_status = Status1::None;
                me.status = Status1::None;
                clock_add(st, et.cure_frames * ctx.p.text_factor);
            }
        }
        // Heal with items before the fight when low (each use is a bag trip).
        if (cfg.entry || in_torrent) && !mewtwo_now {
            let lo = entry_heal_floor(ctx, cfg.entry, dex, &me, a.foes);
            let cur = (me.max_hp as f64 * st.hp_frac).round() as i32;
            if cur < lo {
                let per = potion_hp(ctx, st.seg_i, me.level, st.league_items);
                let uses = (lo - cur + per - 1) / per;
                bump(st, uses as f64 * ctx.turn());
                let before = st.hp_frac;
                st.hp_frac = ((cur + uses * per) as f64 / me.max_hp as f64).min(1.0);
                // The timing lane's potions are free; the risk lane does not inherit them.
                st.risk_real += st.hp_frac - before;
                st.free_heal = true;
            }
        } else if st.hp_frac < ctx.p.preheal_below && !mewtwo_now && !torrent {
            let uses = (((0.85 - st.hp_frac) * me.max_hp as f64)
                / potion_hp(ctx, st.seg_i, me.level, st.league_items) as f64)
                .ceil();
            bump(st, uses * ctx.turn());
            st.risk_real += 0.85 - st.hp_frac;
            st.hp_frac = 0.85;
            st.free_heal = true;
        }
        // Between League fights: one item when the fighter is low (Mewtwo carries its damage
        // from fight to fight like the carry does).
        if a.league && st.heal_items > 0 {
            let cur = if mewtwo_now { st.mewtwo_hp } else { st.hp_frac };
            // `league_torrent`: a carry that already sits in the Torrent window (<= 1/3 HP) and above
            // the foes' best opening hit stays there (the WR carries Lorelei's 28/153 into Bruno).
            let in_window = ctx.p.league_torrent && !mewtwo_now && {
                let hp = (me.max_hp as f64 * cur).round() as i32;
                let lo = crate::battle::entry_floor(dex, &me, a.foes) + 1;
                hp * 3 <= me.max_hp && hp >= lo
            };
            if in_window && std::env::var_os("FRLG_SIM_TRACE").is_some() {
                eprintln!("league-torrent: {} keeps hp {:.2}", a.name, cur);
            }
            if cur < ctx.p.league_preheal && !in_window {
                let max_hp = if mewtwo_now {
                    // Mewtwo's own max HP is not built yet here; the Hyper Potion's 200 of its ~228 at
                    // Lv 70 (guess: the fight's `mhp` trace) is near enough to call it a full heal.
                    200
                } else {
                    me.max_hp
                };
                let after = league_preheal_item(ctx, st, cur, max_hp);
                bump(st, ctx.turn());
                if mewtwo_now {
                    st.mewtwo_hp = after;
                } else {
                    st.hp_frac = after;
                }
            }
        }
        let t_in = st.clock;
        let start_frac = if mewtwo_now { st.mewtwo_hp } else { st.hp_frac };
        me.hp = ((me.max_hp as f64 * start_frac).round() as i32).clamp(1, me.max_hp);
        let mut tac = fight_tactics(
            ctx,
            a.name,
            a.league,
            &TacIn {
                seg_i: st.seg_i,
                torrent_lost: st.torrent_lost,
                level: me.level,
                league_items: st.league_items,
                x_stock: st.x_stock,
                moves: &me.moves,
                fodder: st.fodder,
                fodder_at_start,
                heal_items: st.heal_items,
                mewtwo_now,
                awakenings: st.awakenings,
                cures: st.cures,
            },
        );
        if a.league && ctx.p.league_bag_honest {
            // MODEL CHANGE `league_bag_honest`: the bag's real items (Hyper Potion 200, `item_effects.h:37-40`).
            tac.bag_honest = true;
            tac.heal = 200;
            tac.full_restore = st.lg_fr > 0;
            tac.fr_stock = st.lg_fr.min(st.heal_items);
            tac.max_stock = st.lg_max;
            tac.full_heals = st.lg_fh;
        }
        let xs = tac.x_items.clone();
        // Risk lane: the honest copy of this fight decides whether the attempt survives it.
        let mut risk_healed = false;
        let in_lane = !mewtwo_now && !a.league && ctx.risk_end.is_some_and(|e| st.seg_i <= e);
        let risk_lost = in_lane && {
            let (lost, healed) = risk_fight(ctx, st, rng, a, &tac, &cfg, in_torrent);
            risk_healed = healed;
            lost
        };
        // MODEL CHANGE `turn_power`: the clock's player actions come from a copy of the fight with the
        // carry's attacking stats at `turn_power` instead of `carry_power` (same RNG state).
        let cp_used = if a.league && ctx.p.league_power > 0.0 {
            ctx.p.league_power
        } else if ctx.power_early_end.is_some_and(|e| st.seg_i <= e) {
            ctx.p.carry_power_early
        } else {
            ctx.p.carry_power
        } * cfg.power;
        let turn_copy =
            (a.trainer && !a.league && !mewtwo_now && ctx.p.turn_power > 0.0).then(|| {
                let mut m2 = me.clone();
                let tp = if ctx.p.turn_power_boss > 0.0 && crate::params::is_boss(a.name) {
                    ctx.p.turn_power_boss
                } else {
                    ctx.p.turn_power
                };
                let f = tp / cp_used;
                m2.stat[0] = (m2.stat[0] as f64 * f) as i32;
                m2.stat[3] = (m2.stat[3] as f64 * f) as i32;
                let mut g2 = st.game;
                let o2 = fight(dex, &mut m2, &tac, a.foes, a.ai, a.heals, a.wild, &mut g2);
                if std::env::var_os("FRLG_DBG_COPY").is_some() {
                    eprintln!(
                        "copy {} f {:.3} stats {:?} won {} turns {}",
                        a.name, f, m2.stat, o2.won, o2.turns
                    );
                }
                o2.turns
            });
        let mut g = st.game;
        let out = fight(dex, &mut me, &tac, a.foes, a.ai, a.heals, a.wild, &mut g);
        st.game = g;
        let turns = turn_copy.unwrap_or(out.turns);
        if !mewtwo_now {
            st.persim &= me.persim;
        }
        // Awakenings: the honest lane spent its own (in `risk_fight`); outside it, this fight's.
        if !in_lane {
            st.awakenings = st.awakenings.saturating_sub(out.awakenings_used);
            st.cures = st.cures.saturating_sub(out.cures_used);
        }
        if a.trainer && st.log_fights {
            st.fight_log
                .push((a.name.to_string(), turns, out.won, 0.0, 0));
        }
        if a.trainer && std::env::var_os("FRLG_SIM_TRACE").is_some() {
            eprintln!(
                "fight {} L{} hp{:.2} hi{} mhp{:.2} x{:?} sacs{} pp{:?} moves{:?} -> won {} turns {} copy {:?} x_used {} sacs_used {} kos {} heals {} frs {} end_hp {:.2} stats {:?}/{}",
                a.name, me.level, st.hp_frac, st.heal_items, st.mewtwo_hp, st.x_stock, tac.sacs, me.pp,
                me.moves.iter().map(|&m| dex.moves[m].name.trim_start_matches("MOVE_").to_string()).collect::<Vec<_>>(),
                out.won, out.turns, turn_copy, out.x_used, out.sacs_used, out.kos.len(), out.heals_used, out.full_restores_used, out.hp_frac, me.stat, me.max_hp
            );
        }
        for x in xs
            .iter()
            .filter(|&&x| x.setup_move(dex).is_none())
            .take(out.x_used as usize)
        {
            st.x_stock[x_index(*x)] -= 1;
        }
        bump(
            st,
            turns as f64 * tcost.map_or_else(|| ctx.turn(), |(_, t, _)| t),
        );
        if mewtwo_now && ctx.p.mewtwo_hp_bar > 0.0 {
            bump(st, out.hp_bar * ctx.p.mewtwo_hp_bar);
        }
        if mewtwo_now && ctx.p.mewtwo_heal_extra > 0.0 {
            bump(
                st,
                (out.heals_used + out.full_restores_used) as f64 * ctx.p.mewtwo_heal_extra,
            );
        }
        bump(
            st,
            out.kos.len() as f64 * tcost.map_or_else(|| ctx.mon_overhead() * fit, |(_, _, m)| m),
        );
        // A sacrifice costs its faint animation and the send-outs both ways. guess.
        bump(st, (out.sacs_used + out.sacs_survived) as f64 * 900.0);
        st.fodder = st.fodder.saturating_sub(out.sacs_used);
        if ctx.p.early_helpers && !a.league {
            st.helpers_dead += out.sacs_used;
        }
        if a.league {
            st.heal_items = st
                .heal_items
                .saturating_sub(out.heals_used + out.full_restores_used);
            if ctx.p.league_bag_honest {
                st.lg_fr = st.lg_fr.saturating_sub(out.full_restores_used);
                st.lg_max = st.lg_max.saturating_sub(out.maxes_used);
                st.lg_fh = st.lg_fh.saturating_sub(out.full_heals_used);
                // A Heal with no potion left was a Full Restore (`battle::commit`).
                st.lg_fr = st.lg_fr.min(st.heal_items);
            }
        }
        // `tcost_whole`: the fitted price already holds the WR's reactions and mashing; the draws are kept
        // (common random numbers) and not charged.
        let keep = if whole_price(ctx, a) { 0.0 } else { 1.0 };
        for _ in 0..out.events {
            bump(st, react(ctx, rng) as f64 * keep);
        }
        bump(st, jitter(ctx, rng, turns * 2 + 3) as f64 * keep);
        if a.trainer && std::env::var_os("FRLG_DBG_EXTRA").is_some() {
            eprintln!(
                "extra {} pre {} battle {} turns {} kos {} sacs {} events {}",
                a.name,
                t_in - t0,
                st.clock - t_in,
                turns,
                out.kos.len(),
                out.sacs_used + out.sacs_survived,
                out.events
            );
        }
        if let Some(l) = st.fight_log.last_mut() {
            if l.0 == a.name && l.3 == 0.0 {
                l.3 = (st.clock - t0) as f64;
            }
        }
        if !out.won && std::env::var_os("FRLG_RISK_TRACE").is_some() {
            eprintln!(
                "timing-loss {} L{} risk_lost {}",
                a.name, st.carry, risk_lost
            );
        }
        if out.won && !risk_lost {
            if mewtwo_now {
                st.mewtwo_hp = out.hp_frac;
                if ctx.p.mewtwo_pp {
                    st.mewtwo_pp = me.pp.clone();
                }
            } else {
                st.hp_frac = out.hp_frac;
                // `risk_adopt`: the lane's potions were drunk for real, so the carry leaves the fight
                // as healthy as the lane did and the Torrent window (<= 1/3 HP) is gone with them.
                if ctx.p.risk_adopt && risk_healed {
                    st.hp_frac = st.hp_frac.max(st.risk_hp);
                    st.torrent_lost = ctx.p.risk_torrent_lost && st.hp_frac * 3.0 > 1.0;
                } else if st.hp_frac * 3.0 <= 1.0 {
                    // Back inside the window by damage: the Torrent phase resumes.
                    st.torrent_lost = false;
                }
            }
            if !mewtwo_now {
                st.carry_status = out.end_status;
                // `tackle_only_model`: the masked moves keep the PP they had before the fight.
                let masked = tackle_only_masked(ctx, a.name, &st.slots, &st.slot_pp);
                st.slot_pp = me
                    .pp
                    .iter()
                    .enumerate()
                    .map(|(k, &pp)| {
                        if masked.contains(&k) {
                            st.slot_pp.get(k).copied().unwrap_or(pp)
                        } else {
                            pp
                        }
                    })
                    .collect();
                if !in_torrent || ctx.p.poison_frames_per_hp <= 0.0 {
                    cure_status(ctx, st);
                }
            }
            for &(sp, lvl) in &out.kos {
                let yld = a.yield_.unwrap_or(dex.species[sp].exp_yield);
                // `Cmd_getexp` gives the EVs first, then the exp: a level-up in the same KO already
                // recomputes the stats with them.
                if mewtwo_now && ctx.p.mewtwo_evs {
                    let mut real = st.mewtwo_evs_real;
                    add_evs(ctx, &mut real, dex.species[sp].ev_yield);
                    st.mewtwo_evs_real = real;
                    if !ctx.p.ev_lag {
                        st.mewtwo_evs = real;
                    }
                }
                if mewtwo_now && ctx.p.mewtwo_levels {
                    let before = mewtwo_level(ctx, st);
                    st.mewtwo_exp += exp_gain(yld as u8, lvl as u8, a.trainer);
                    let gained = mewtwo_level(ctx, st).saturating_sub(before);
                    bump(st, gained as f64 * ctx.level_up());
                    if gained > 0 {
                        st.mewtwo_evs = st.mewtwo_evs_real;
                    }
                }
                if !mewtwo_now {
                    let mut real = st.evs_real;
                    add_evs(ctx, &mut real, dex.species[sp].ev_yield);
                    st.evs_real = real;
                    // Without `ev_lag` the stats follow the EVs at once (the old model); with it
                    // `add_exp` copies them in on a level-up.
                    if !ctx.p.ev_lag {
                        st.evs = real;
                    }
                    let e = exp_gain(yld as u8, lvl as u8, a.trainer);
                    let g = add_exp(ctx, st, e);
                    bump(st, g as f64 * ctx.level_up());
                    if a.trainer && st.log_fights {
                        if let Some(l) = st.fight_log.last_mut() {
                            l.4 += g;
                        }
                    }
                }
            }
            if a.trainer {
                bump(st, ctx.outro());
            }
            st.risk_real = st.hp_frac;
            return true;
        }
        // A wild battle the engine ended without a win and without fainting is a flee
        // (a wild Abra's Teleport): carry on.
        if a.wild && out.hp_frac > 0.0 && !risk_lost {
            st.hp_frac = out.hp_frac;
            st.carry_status = out.end_status;
            cure_status(ctx, st);
            return true;
        }
        st.whiteouts += 1;
        if risk_lost && ctx.p.risk_abandon && !a.league {
            st.abandoned = true;
            return false;
        }
        let rule = ctx.salvage.get(st.seg_i).copied().unwrap_or(-1.0);
        // The runner does not salvage an early death: he resets.
        if rule < 0.0 && ctx.no_salvage_end.is_some_and(|e| st.seg_i <= e) {
            st.abandoned = true;
            return false;
        }
        if a.league {
            if ctx.p.league_redo <= 0.0 || st.target.is_none() {
                st.abandoned = true;
                return false;
            }
            // Back to the first room: the rooms are walked and fought again.
            let spent = (st.clock - st.league_t0) as f64;
            bump(st, spent * ctx.p.league_redo);
            // MODEL CHANGE `league_whiteout_money`: `ComputeWhiteOutMoneyLoss` with 8 badges
            // (`overworld.c:227-268`): highest party level x 4 x 30.
            if ctx.p.league_whiteout_money && ctx.p.money_model {
                let mut top = st.carry;
                if st.mewtwo.is_some() {
                    top = top.max(mewtwo_level(ctx, st));
                }
                st.money -= (top * 4 * 30).min(st.money);
            }
            // PLAN `league_rebuy`: the respawn is the Indigo Center, the mart is in the room.
            if ctx.p.league_rebuy && league_topup(ctx, st) > 0 {
                bump(st, 600.0 * ctx.p.text_factor);
            }
        }
        bump(st, ctx.k("whiteout", WHITEOUT as f64));
        if rule >= 0.0 {
            if let Some(t) = st.target {
                let i = st.seg_i;
                let fl = ctx.floor_after[i] as f64;
                let rem = fl + rule * (ctx.mean_after[i] - fl).max(0.0);
                if st.clock as f64 + rem > t as f64 {
                    st.abandoned = true;
                    return false;
                }
            }
        }
        st.hp_frac = 1.0;
        st.carry_status = Status1::None;
        // A whiteout lands at a Pokémon Center: PP comes back too.
        for k in 0..st.slots.len() {
            st.slot_pp[k] = carry_full_pp(ctx, st, k);
        }
        restore_mewtwo_pp(ctx, st);
        tries += 1;
        if a.wild {
            // The wild mon is gone; the run continues.
            return true;
        }
        // Three whiteouts on one fight: the human resets.
        if tries >= 3 {
            st.abandoned = true;
            return false;
        }
    }
}

/// Soft-reset to the last save (`Params::reload_save_at`): `lost` frames of the dying fight and the reload
/// are added to the clock, the saved state comes back, and the segment to replay from is returned; `None`
/// when there is no save, no reload left, or the replay no longer fits the target.
fn try_reload(
    ctx: &Ctx,
    st: &mut State,
    ckpt: &Option<(usize, State)>,
    reloads: &mut u32,
    lost: f64,
) -> Option<usize> {
    let (ci, snap) = ckpt.as_ref()?;
    if *reloads >= ctx.p.reload_max {
        return None;
    }
    let t = st.target?;
    let now = st.clock + (lost + ctx.p.reload_frames as f64 * ctx.p.text_factor).round() as u64;
    let k = ci.saturating_sub(1);
    let fl = if *ci > 0 {
        ctx.floor_after[k] as f64
    } else {
        0.0
    };
    let mean = if *ci > 0 { ctx.mean_after[k] } else { fl };
    if now as f64 + fl + ctx.p.reload_aggr * (mean - fl).max(0.0) > t as f64 {
        return None;
    }
    let (off, whiteouts) = (st.off, st.whiteouts + 1);
    *st = snap.clone();
    st.clock = now;
    st.off = off;
    st.whiteouts = whiteouts;
    st.abandoned = false;
    *reloads += 1;
    Some(*ci)
}

/// The risk lane of one fight (`Params::risk_until`): the same fight again with the formula's own
/// damage (`risk_power`), foes scaled by the noise knob `risk_foe`, from the lane's own HP and with a
/// finite potion budget. Returns true when the carry faints in it (or the fight's `risk_noise` draw
/// hits). The lane's HP follows the timing lane's heals and rests (`risk_real`).
fn risk_fight(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    a: &BattleArgs,
    tac: &crate::battle::Tactics,
    cfg: &TacCfg,
    in_torrent: bool,
) -> (bool, bool) {
    use crate::battle::{entry_floor, fight, make_mon};
    let dex = ctx.dex;
    let mut me = make_mon(
        dex,
        carry_species(ctx, st.carry),
        st.carry,
        st.ivs,
        st.evs,
        st.nature,
        st.badges,
    );
    me.moves = st.slots.clone();
    me.pp = st.slot_pp.clone();
    tackle_only_mask(ctx, a.name, &mut me);
    me.status = st.carry_status;
    if st.mystic {
        me.boost = Some((11, 10));
    } else if st.ice {
        me.boost = Some((15, 10));
    } else if ctx.p.black_glasses && st.glasses && !st.leftovers {
        me.boost = Some((17, 10));
    }
    me.leftovers = st.leftovers;
    me.persim = st.persim;
    let win = ctx
        .risk_win
        .get(st.seg_i)
        .copied()
        .unwrap_or((0.0, 0.0, None));
    let rp = if cfg.risk_power > 0.0 {
        cfg.risk_power
    } else if win.0 > 0.0 {
        win.0
    } else {
        ctx.p.risk_power
    };
    if rp != 1.0 {
        me.stat[0] = (me.stat[0] as f64 * rp) as i32;
        me.stat[3] = (me.stat[3] as f64 * rp) as i32;
    }
    let rf = if cfg.risk_foe > 0.0 {
        cfg.risk_foe
    } else if win.1 > 0.0 {
        win.1
    } else {
        ctx.p.risk_foe
    };
    let mut foes: Vec<crate::battle::Mon> = a.foes.to_vec();
    if rf != 1.0 {
        for f in &mut foes {
            f.stat[0] = (f.stat[0] as f64 * rf) as i32;
            f.stat[3] = (f.stat[3] as f64 * rf) as i32;
        }
    }
    // Lane sync: what the timing lane gained since the last fight (a heal, a rest) the lane gains
    // too; what it lost between fights (field poison) it loses too.
    // MODEL CHANGE `risk_entry_paid`: the timing lane's free pre-fight potions are no real heal.
    let free = std::mem::take(&mut st.free_heal);
    st.risk_hp = if st.hp_frac >= 0.999 && !(ctx.p.risk_entry_paid && free) {
        1.0
    } else {
        (st.risk_hp + st.hp_frac - st.risk_real).clamp(0.0, 1.0)
    };
    // PLAN `early_torrent`: the human walks the real (lane) HP into the window too; the walk is charged once,
    // for the larger of the two lanes' drops.
    if let Some(tf) = st.steer_to.take() {
        let lane_cur = (me.max_hp as f64 * st.risk_hp).round() as i32;
        let target = (me.max_hp as f64 * tf).round() as i32;
        if lane_cur > target {
            let extra = (lane_cur - target - st.steer_hp).max(0);
            clock_add(st, extra as f64 * ctx.p.poison_frames_per_hp);
            st.risk_hp = tf;
        }
    }
    let mut potions = if cfg.potions >= 0 {
        cfg.potions as u32
    } else {
        win.2.unwrap_or(ctx.p.risk_potions)
    };
    // Potions are paid for (Potion 300, Super Potion 700, Hyper Potion 1200 at the heal tier of the level:
    // `items.json`) out of the purse and each use is a bag turn: no free lunch in healing.
    let per = potion_hp(ctx, st.seg_i, st.carry, st.league_items);
    let price = potion_price(per);
    // MODEL CHANGE `potion_bag_from`: from there on the potions come out of the bag (paid at the stop).
    let bag = ctx.potion_bag_from.is_some_and(|f| st.seg_i >= f);
    // MODEL CHANGE `early_bag`: before the Pewter mart only the picked-up Potions exist.
    let early = ctx.early_bag_end.is_some_and(|e| st.seg_i <= e) && !bag;
    let pay = (ctx.p.risk_pay && ctx.p.money_model && !bag) || early;
    if bag {
        // MODEL CHANGE `mid_bag`: the early pickups still in the bag join the counted bag.
        if ctx.p.mid_bag {
            st.potions += std::mem::take(&mut st.free_potions);
        }
        potions = potions.min(st.potions);
    }
    if early {
        potions = potions.min(st.free_potions);
    } else if pay {
        potions = potions.min(st.free_potions + st.money / price);
    }
    let mut healed = false;
    // The entry heal of the Torrent phase, as in the timing lane but paid from the potion budget.
    if (cfg.entry || in_torrent) && potions > 0 {
        let hits = if cfg.entry { 1.0 } else { ctx.p.torrent_hits };
        let lo = (entry_floor(dex, &me, &foes) as f64 * hits).ceil() as i32 + 1;
        let cur = (me.max_hp as f64 * st.risk_hp).round() as i32;
        if cur < lo {
            let per = potion_hp(ctx, st.seg_i, me.level, st.league_items);
            let uses = (((lo - cur + per - 1) / per).max(1) as u32).min(potions);
            potions -= uses;
            healed = true;
            if bag {
                st.potions -= uses;
                bump(st, uses as f64 * ctx.turn());
            }
            if pay {
                spend_potions(st, uses, price);
                bump(st, uses as f64 * ctx.turn());
                potions = potions.min(st.free_potions + if early { 0 } else { st.money / price });
            }
            st.risk_hp = ((cur + uses as i32 * per) as f64 / me.max_hp as f64).min(1.0);
        }
    }
    me.hp = ((me.max_hp as f64 * st.risk_hp).round() as i32).clamp(1, me.max_hp);
    let mut t2 = tac.clone();
    t2.heal_stock = potions;
    t2.rollouts = 0;
    let mut g = crate::rng::GameRng::from_u32(rng.next_u64() as u32);
    let out = fight(dex, &mut me, &t2, &foes, a.ai, a.heals, a.wild, &mut g);
    // A berry eaten in either lane is gone.
    st.persim &= me.persim;
    st.awakenings = st.awakenings.saturating_sub(out.awakenings_used);
    st.cures = st.cures.saturating_sub(out.cures_used);
    healed |= out.heals_used > 0;
    if bag {
        let n = out.heals_used.min(potions);
        st.potions -= n;
        bump(st, n as f64 * ctx.turn());
    }
    if pay {
        let n = out.heals_used.min(potions);
        spend_potions(st, n, price);
        // `tcost_whole`: the clock's actions (the `turn_power` copy, heal turns included) already pay the
        // in-battle heals; the lane's potions cost money only.
        if !whole_price(ctx, a) {
            bump(st, n as f64 * ctx.turn());
        }
    }
    let fled = a.wild && !out.won && out.hp_frac > 0.0;
    let mut lost = !out.won && !fled;
    if std::env::var_os("FRLG_RISK_TRACE").is_some() {
        eprintln!(
            "risk {} L{} hp_in {:.2} pots {} -> won {} turns {} hp_out {:.2} used {} seg {} bag {}",
            a.name,
            st.carry,
            st.risk_hp,
            potions,
            !lost,
            out.turns,
            out.hp_frac,
            out.heals_used,
            st.seg_i,
            st.potions
        );
    }
    if !lost {
        st.risk_hp = out.hp_frac.max(0.0);
        if let Some(&p) = ctx.p.risk_noise.get(a.name) {
            lost = rng.chance(p);
        }
    }
    (lost, healed)
}

/// Clock-only cost (no VBlank advance of the game's stream): the small review-round plan costs below keep
/// the pair's game draws common, so a 5-second plan change is not drowned in reshuffled fights.
fn clock_add(st: &mut State, f: f64) {
    if f >= 0.0 {
        st.clock += f.round() as u64;
    } else {
        st.clock = st.clock.saturating_sub((-f).round() as u64);
    }
}

/// The review-round-1 plan keys (R3) that act at one segment: rival name, early Potions, the Pewter mart
/// after Brock (TM39 sale, Poké Balls).
fn review_keys(ctx: &Ctx, st: &mut State, name: &str) {
    let p = ctx.p;
    if p.rival_name_len == 1 {
        if name == "names" {
            clock_add(st, p.rival_name_frames * p.text_factor);
        } else if name.starts_with("rival-") || name == "champion" || name == "champion-2" {
            // 6 rival fights + the two Champion fights carry the ~85 prints.
            clock_add(st, -p.rival_print_saving / 8.0);
        }
    }
    for (pick, anchor) in [
        ("bedroom_pc", "house"),
        ("route1_clerk", "to-viridian"),
        ("forest_hidden", "forest-trainers"),
    ] {
        if name == anchor && p.early_potions.iter().any(|e| e == pick) {
            st.free_potions += 1;
            let f = p.early_potion_frames.get(pick).copied().unwrap_or(0.0);
            clock_add(st, f * p.text_factor);
        }
    }
    if name == "deliver" && p.viridian_potions > 0 {
        let n = if p.money_model {
            p.viridian_potions.min(st.money / 300)
        } else {
            p.viridian_potions
        };
        if p.money_model {
            st.money -= n * 300;
        }
        st.free_potions += n;
        let f = if p.early_balls > 0 { 150.0 } else { 900.0 };
        clock_add(st, f * p.text_factor);
    }
    if name == "brock" && !st.abandoned {
        if p.tm39_sell {
            st.money += 1500;
            clock_add(st, p.tm39_sell_frames * p.text_factor);
        }
        if p.pewter_balls > 0 {
            let n = if p.money_model {
                p.pewter_balls.min(st.money / 200)
            } else {
                p.pewter_balls
            };
            if p.money_model {
                st.money -= n * 200;
            }
            st.balls[0] += n;
            clock_add(st, p.pewter_ball_frames * p.text_factor);
        }
    }
}

/// MODEL CHANGE `tackle_only_model`: the move slots whose PP is hidden in the fight `name` (the damaging
/// moves other than Tackle, when the carry still has Tackle PP). Empty when the key is off or does not apply.
/// Decided on the slots and PP the carry enters the fight with.
fn tackle_only_masked(ctx: &Ctx, name: &str, moves: &[usize], pp: &[i32]) -> Vec<usize> {
    if !ctx.p.tackle_only_model || !ctx.p.early_torrent.tackle_only.iter().any(|t| t == name) {
        return vec![];
    }
    let tackle = ctx.dex.move_id("MOVE_TACKLE");
    let Some(ti) = moves.iter().position(|&m| m == tackle) else {
        return vec![];
    };
    if pp.get(ti).is_some_and(|&p| p <= 0) {
        return vec![];
    }
    (0..moves.len())
        .filter(|&k| k != ti && ctx.dex.moves[moves[k]].power > 0)
        .collect()
}

fn tackle_only_mask(ctx: &Ctx, name: &str, me: &mut crate::battle::Mon) {
    for k in tackle_only_masked(ctx, name, &me.moves, &me.pp) {
        if let Some(p) = me.pp.get_mut(k) {
            *p = 0;
        }
    }
}

/// Does any caught species learn HM01 (`Params::cut_honest`)?
fn has_cutter(ctx: &Ctx, st: &State) -> bool {
    let cut = ctx.dex.move_id("MOVE_CUT");
    (0..ctx.dex.species.len()).any(|i| {
        let d = ctx.dex.species[i].dex;
        d != 0 && st.caught.has(d) && can_learn(ctx, i, cut)
    })
}

/// MODEL CHANGE `Params::cut_honest`: the HM01 carrier (a hunt for one, the teach, or the walk around the tree).
fn cut_carrier(ctx: &Ctx, st: &mut State, rng: &mut Rng, name: &str) {
    let p = ctx.p;
    if name == p.cut_hunt_after && !has_cutter(ctx, st) {
        let cut = ctx.dex.move_id("MOVE_CUT");
        let lg = p.version == "LG";
        let skip: Vec<String> = ctx
            .dex
            .area(&p.cut_hunt_site, Area::Land, lg, &p.rod)
            .map(|(_, slots, _)| {
                slots
                    .iter()
                    .filter(|s| !can_learn(ctx, s.species, cut))
                    .map(|s| ctx.dex.species[s.species].name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let h = Hunt {
            after: name.to_string(),
            site: p.cut_hunt_site.clone(),
            area: "land".into(),
            steps: p.cut_hunt_steps,
            skip,
            detour_tiles: 0,
            on_route: true,
            stop_after: 1,
            repel_level: 0,
        };
        let c0 = st.clock;
        hunt(ctx, st, rng, &h, Era::Run);
        if std::env::var_os("FRLG_CATCH_TRACE").is_some() {
            let s = (st.clock - c0) as f64 / 59.7275;
            eprintln!("cut-hunt {} got {} s {:.0}", name, has_cutter(ctx, st), s);
        }
    } else if name == p.cut_hunt_after && std::env::var_os("FRLG_CATCH_TRACE").is_some() {
        eprintln!("cut-hunt {name} already");
    }
    if name == p.cut_teach_at && p.cut_rehunt && !has_cutter(ctx, st) {
        // MODEL CHANGE `cut_rehunt`: no legal walk-around; back to the Route 24 grass for another hunt.
        let tile = Era::Bike.frames_per_tile() * p.path_overhead;
        bump(st, p.cut_rehunt_tiles as f64 * tile);
        let have: u32 = st.balls.iter().sum();
        if have < p.cut_rehunt_balls {
            let qty = if p.money_model {
                (p.cut_rehunt_balls - have).min(st.money / BALL_PRICE[0])
            } else {
                p.cut_rehunt_balls - have
            };
            if qty > 0 {
                if p.money_model {
                    st.money -= qty * BALL_PRICE[0];
                }
                st.balls[0] += qty;
                bump(st, MART as f64 * p.text_factor + 32.0 * tile);
            }
        }
        let cut = ctx.dex.move_id("MOVE_CUT");
        let skip: Vec<String> = ctx
            .dex
            .area(&p.cut_rehunt_site, Area::Land, p.version == "LG", &p.rod)
            .map(|(_, slots, _)| {
                slots
                    .iter()
                    .filter(|s| !can_learn(ctx, s.species, cut))
                    .map(|s| ctx.dex.species[s.species].name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let h = Hunt {
            after: name.to_string(),
            site: p.cut_rehunt_site.clone(),
            area: "land".into(),
            steps: p.cut_rehunt_steps,
            skip,
            detour_tiles: 0,
            on_route: true,
            stop_after: 1,
            repel_level: 0,
        };
        let c0 = st.clock;
        hunt(ctx, st, rng, &h, Era::Bike);
        if std::env::var_os("FRLG_CATCH_TRACE").is_some() {
            let s = (st.clock - c0) as f64 / 59.7275;
            eprintln!("cut-rehunt {} got {} s {:.0}", name, has_cutter(ctx, st), s);
        }
    }
    if name == p.cut_teach_at {
        if has_cutter(ctx, st) {
            if p.cut_box_honest {
                // MODEL CHANGE `cut_box_honest`: the earliest-obtained HM01 learner, withdrawn when boxed.
                let cut = ctx.dex.move_id("MOVE_CUT");
                let first = (0..ctx.dex.species.len())
                    .filter(|&i| {
                        let d = ctx.dex.species[i].dex;
                        d != 0 && st.caught.has(d) && can_learn(ctx, i, cut)
                    })
                    .map(|i| ctx.dex.species[i].dex)
                    .min_by_key(|&d| st.catch_seq[d as usize]);
                if let Some(d) = first {
                    pc_visit(ctx, st, &[d], name);
                }
            }
            clock_add(st, p.cut_teach_frames * p.text_factor);
        } else {
            bump(
                st,
                p.cut_detour_tiles as f64 * Era::Run.frames_per_tile() * p.path_overhead,
            );
        }
    }
}

/// Spend `n` risk-lane potions: the free early pickups first (`Params::early_potions`), the rest bought at
/// `price` from the purse.
fn spend_potions(st: &mut State, n: u32, price: u32) {
    let f = n.min(st.free_potions);
    st.free_potions -= f;
    st.money = st.money.saturating_sub((n - f) * price);
}

/// Mart price of the potion that heals `per` HP (`items.json`: Potion 300, Super Potion 700, Hyper
/// Potion 1200).
fn potion_price(per: i32) -> u32 {
    match per {
        20 => 300,
        50 => 700,
        _ => 1200,
    }
}

/// Field poison (`field_control_avatar.c:710-733`, `field_poison.c:92-119`: 1 HP per 5 steps, a
/// poisoned mon at 0 faints). The human drinks an Antidote (frames, 100 money) when the HP reaches
/// `poison_floor`; the cure is also a bag trip (400 frames, as `cure_status`).
fn field_poison(ctx: &Ctx, st: &mut State, frames: f64) {
    use crate::battle::{make_mon, Status1};
    st.poison_acc += frames;
    let lose = (st.poison_acc / ctx.p.poison_frames_per_hp).floor();
    if lose < 1.0 {
        return;
    }
    st.poison_acc -= lose * ctx.p.poison_frames_per_hp;
    let me = make_mon(
        ctx.dex,
        carry_species(ctx, st.carry),
        st.carry,
        st.ivs,
        st.evs,
        st.nature,
        st.badges,
    );
    let hp = (me.max_hp as f64 * st.hp_frac).round() as i32 - lose as i32;
    if hp <= ctx.p.poison_floor {
        st.hp_frac = ctx.p.poison_floor.max(1) as f64 / me.max_hp as f64;
        bump(st, 400.0 * ctx.p.text_factor);
        if ctx.p.money_model && st.money >= 100 {
            st.money -= 100;
        }
        st.carry_status = Status1::None;
        st.poison_acc = 0.0;
    } else {
        st.hp_frac = hp as f64 / me.max_hp as f64;
    }
}

fn x_index(x: XItem) -> usize {
    match x {
        XItem::Attack => 0,
        XItem::Defend => 1,
        XItem::Speed => 2,
        XItem::Special => 3,
        XItem::DireHit => 4,
        XItem::Accuracy => 5,
        XItem::GuardSpec => 6,
        XItem::CalmMind | XItem::Move(_) => unreachable!("a set-up move is not stock"),
    }
}

/// Cure a status that outlived a battle with the matching item: frames and (money model)
/// the mart price (`items.json`: Antidote 100, Paralyze Heal 200, Awakening 250, Burn
/// Heal 250, Ice Heal 250). Poison that stays would otherwise hurt in the overworld.
fn cure_status(ctx: &Ctx, st: &mut State) {
    use crate::battle::Status1;
    if !ctx.p.cure_status || !st.carry_status.any() {
        return;
    }
    let price = match st.carry_status {
        Status1::Poison | Status1::Toxic(_) => 100,
        Status1::Paralysis => 200,
        _ => 250,
    };
    if ctx.p.money_model {
        if st.money < price {
            return; // live with it until the next Pokémon Center
        }
        st.money -= price;
    }
    bump(st, 400.0 * ctx.p.text_factor);
    st.carry_status = Status1::None;
}

/// A death the fight engine does not produce (`Params::hazard`): the human's own misjudgment or an
/// unlucky event in the named segment. Same consequence as losing the fight: a whiteout, and an
/// abandoned attempt while no salvage is allowed. Returns true when the attempt is over.
fn hazard_death(ctx: &Ctx, st: &mut State, rng: &mut Rng, name: &str) -> bool {
    let p = ctx.p.hazard.get(name).copied().unwrap_or(0.0);
    if p <= 0.0 || !rng.chance(p) {
        return false;
    }
    st.whiteouts += 1;
    if ctx.no_salvage_end.is_some_and(|e| st.seg_i <= e) {
        st.abandoned = true;
        return true;
    }
    bump(st, ctx.k("whiteout", WHITEOUT as f64));
    st.hp_frac = 1.0;
    st.carry_status = crate::battle::Status1::None;
    false
}

pub fn is_league(name: &str) -> bool {
    let n = name.trim_end_matches("-2");
    matches!(n, "lorelei" | "bruno" | "agatha" | "lance" | "champion")
}

fn group_battle(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    n: u32,
    mons: u32,
    level: u32,
    yld: u32,
) -> bool {
    for _ in 0..n {
        if ctx.p.battle_sim {
            let foes: Vec<crate::battle::Mon> = (0..mons)
                .map(|_| crate::battle::generic_mon(ctx.dex, level))
                .collect();
            let args = BattleArgs {
                name: "group",
                foes: &foes,
                trainer: true,
                intro: 600,
                ai: 1,
                heals: 0,
                yield_: Some(yld),
                league: false,
                wild: false,
            };
            if !sim_battle(ctx, st, rng, &args) {
                return false;
            }
            continue;
        }
        let enemies: Vec<(u32, u32, f64)> = (0..mons).map(|_| (level, yld, 1.0)).collect();
        if !battle(ctx, st, rng, &enemies, true, 600, 1.0) {
            return false;
        }
    }
    true
}

/// Scene frames of an evolution event (guess: menu to the party screen, the item or the lead swap,
/// the evolution animation and the dex page) by kind.
const EVO_STONE: u32 = 1100;
const EVO_CANDY: u32 = 700;
const EVO_CANDY_EACH: u32 = 300;
const EVO_LEVEL: u32 = 2000;
/// Walking into the Celadon 4F shop and buying one stone (guess).
const EVO_STONE_SHOP: u32 = 800;

/// One `Evolve` event (see `params::Evolve`). Skipped for free when the evolved form is already in
/// the dex, the pre-evolution is not, or the stone / candies / money are not there.
/// The stone an `Evolve` uses, as an item constant (`ITEM_MOON_STONE`), if it names one.
pub fn evo_item(evo: &Evolve) -> Option<String> {
    (!evo.item.is_empty()).then(|| format!("ITEM_{}_STONE", evo.item))
}

/// `evolve`'s pre-evolution: the Kanto species whose `evo` list reaches `evo.species` the way the
/// plan says (`EVO_ITEM` with the plan's stone, else `EVO_LEVEL`; candies count as levels), among
/// the species the human has a body of (`has`, by national number). Returns its national number
/// and the evolution parameter (the level, or the item). Shared with the playing controller
/// (`play/dex.rs`), which passes the party and the boxes.
pub fn evo_source(ctx: &Ctx, evo: &Evolve, has: &dyn Fn(u32) -> bool) -> Option<(u32, String)> {
    let t = ctx.dex.id(&evo.species);
    let item = evo_item(evo);
    ctx.dex.species.iter().find_map(|sp| {
        if sp.dex == 0 || !has(sp.dex) {
            return None;
        }
        sp.evo.iter().find_map(|(k, param, target)| {
            let ok = *target == t
                && match evo.via.as_str() {
                    "stone" => k == "EVO_ITEM" && item.as_deref().is_none_or(|i| i == param),
                    _ => k == "EVO_LEVEL",
                };
            ok.then(|| (sp.dex, param.clone()))
        })
    })
}

fn evolve(ctx: &Ctx, st: &mut State, rng: &mut Rng, evo: &Evolve, seg: &Seg) {
    let t = ctx.dex.id(&evo.species);
    let tdex = ctx.dex.species[t].dex;
    if tdex == 0 || st.caught.has(tdex) {
        return;
    }
    let item = evo_item(evo);
    let Some((pre, param)) = evo_source(ctx, evo, &|d| st.bodies[d as usize] > 0) else {
        return;
    };
    let lv = st.catch_lv[pre as usize] as u32;
    let mut frames = evo.frames;
    match evo.via.as_str() {
        "stone" => {
            if frames == 0 {
                frames = if ctx.p.evo_stone_frames > 0 {
                    ctx.p.evo_stone_frames
                } else {
                    EVO_STONE
                };
            }
            let have = stone_ix(&evo.item).filter(|&ix| st.stones[ix] > 0);
            if let Some(ix) = have {
                st.stones[ix] -= 1;
            } else if !evo.buy {
                return;
            } else if !ctx.p.stone_shops.is_empty()
                && (evo.item.eq_ignore_ascii_case("MOON")
                    || ctx.stone_shop_from.is_none_or(|f| st.seg_i < f))
            {
                // MODEL CHANGE `stone_shops`: no Celadon 4F visit yet, or a stone no mart sells.
                return;
            } else {
                let price = item
                    .as_ref()
                    .and_then(|i| ctx.dex.item_prices.get(i))
                    .copied()
                    .unwrap_or(2100);
                if ctx.p.money_model {
                    if st.money < price {
                        return;
                    }
                    st.money -= price;
                }
                frames += EVO_STONE_SHOP;
            }
        }
        "candy" => {
            let need = if evo.candies > 0 {
                evo.candies
            } else {
                let at: u32 = param.parse().unwrap_or(1);
                if lv >= at {
                    1
                } else {
                    (at - lv).max(1)
                }
            };
            if st.candies < need {
                return;
            }
            st.candies -= need;
            if frames == 0 {
                let base = if ctx.p.evo_candy_frames > 0 {
                    ctx.p.evo_candy_frames
                } else {
                    EVO_CANDY
                };
                frames = base + EVO_CANDY_EACH * need;
            }
        }
        _ if ctx.p.level_evo_honest => {
            // MODEL CHANGE `level_evo_honest`: a non-fighting mon levels only by Rare Candy.
            let at: u32 = param.parse().unwrap_or(1);
            let need = at.saturating_sub(lv).max(1);
            if st.candies < need
                || (ctx.p.level_evo_max_candies > 0 && need > ctx.p.level_evo_max_candies)
            {
                return;
            }
            st.candies -= need;
            if frames == 0 {
                let base = if ctx.p.evo_candy_frames > 0 {
                    ctx.p.evo_candy_frames
                } else {
                    EVO_CANDY
                };
                frames = base + EVO_CANDY_EACH * need;
            }
        }
        _ => {
            let at: u32 = param.parse().unwrap_or(1);
            if lv + ctx.p.level_gap < at {
                return;
            }
            if frames == 0 {
                frames = EVO_LEVEL;
            }
        }
    }
    pc_visit(ctx, st, &[pre], &evo.after);
    st.bodies[pre as usize] -= 1;
    bump(st, frames as f64 * ctx.p.text_factor);
    bump(st, jitter(ctx, rng, seg.inputs) as f64);
    let at_lv = if evo.via == "stone" {
        lv
    } else {
        lv.max(param.parse().unwrap_or(lv))
    };
    st.cur_src = match evo.via.as_str() {
        "stone" => 4,
        "candy" => 5,
        _ if ctx.p.level_evo_honest => 5,
        _ => 6,
    };
    got(st, tdex, at_lv);
    st.cur_src = 0;
}

fn hunt(ctx: &Ctx, st: &mut State, rng: &mut Rng, h: &Hunt, era: Era) {
    let area = hunt_area(h);
    let skip = hunt_skip(ctx, h);
    let safari = is_safari_hunt(ctx, h);
    if safari && h.on_route && st.safari_done {
        // The plan's `safari_stop` was reached: the zone was left for good.
        return;
    }
    let c_before = st.caught.count();
    let stop_new = hunt_stop_new(ctx, h, st.safari_new);
    let mut steps = h.steps;
    if !safari {
        st.safari_open = false;
    } else {
        if !st.safari_open && h.on_route {
            // The route's own `safari-fee-and-hms` segment paid the gate.
            st.safari_open = true;
            st.safari_steps = 600;
            st.safari_balls = 30;
        }
        if !st.safari_open {
            if ctx.p.money_model {
                // 500 at the gate (`decompiled/data/maps/FuchsiaCity_SafariZone_Entrance/scripts.inc`).
                if st.money < 500 {
                    return;
                }
                st.money -= 500;
            }
            bump(st, SAFARI_ENTRY as f64 * ctx.p.text_factor);
            st.safari_open = true;
            st.safari_steps = 600;
            st.safari_balls = 30;
        }
        st.safari_steps = st.safari_steps.saturating_sub(h.detour_tiles * 2);
        // MODEL CHANGE `safari_walk_steps`: the in-zone walk to this hunt also runs the counter.
        if let Some(&w) = ctx.p.safari_walk_steps.get(&h.site) {
            st.safari_steps = st.safari_steps.saturating_sub(w);
        }
        if area == Area::Land {
            // PLAN `safari_drain`: this hunt takes what is left on the counter.
            if ctx.p.safari_drain == h.site {
                steps = st.safari_steps;
            }
            steps = steps.min(st.safari_steps);
        }
    }
    let walk_tiles = if h.on_route {
        h.detour_tiles
    } else if ctx.p.detour_map {
        ctx.dex
            .fly_tiles(&h.site, &h.area)
            .unwrap_or(h.detour_tiles)
    } else {
        h.detour_tiles
    };
    // On the bike where the map allows it (the Power Plant forbids cycling).
    let walk_era = if ctx.p.detour_bike && ctx.p.bike && h.site != "MAP_POWER_PLANT" {
        Era::Bike
    } else {
        Era::Run
    };
    // An Escape Rope replaces the walk back out of a dungeon site (rope 550,
    // `items.json`; warps to the entrance, `EscapeRopeFieldEffect`).
    let dungeon = [
        "SEAFOAM",
        "VICTORY_ROAD",
        "POKEMON_TOWER",
        "DIGLETTS",
        "CAVE",
        "TUNNEL",
        "MT_MOON",
    ]
    .iter()
    .any(|d| h.site.contains(d));
    let ways = if ctx.p.rope_fills && dungeon {
        if ctx.p.money_model && st.money >= 550 {
            st.money -= 550;
        }
        bump(st, 350.0 * ctx.p.text_factor);
        1.0
    } else {
        2.0
    };
    bump(
        st,
        walk_tiles as f64 * ways * walk_era.frames_per_tile() * ctx.p.path_overhead,
    );
    let per_step = if area == Area::Land {
        era.frames_per_tile() * ctx.p.path_overhead
    } else {
        // Casting and waiting for a bite. guess.
        180.0
    };
    // Repel hunt: a Lv `repel_level` lead plus a Repel blocks every land encounter below that
    // level (`decompiled/src/wild_encounter.c:286,601-622`; fishing is not checked, :301-307).
    // Repels are bought and used as the steps run out; the lead swap is a party-menu trip. guess.
    let mut covered = 0u32;
    if h.repel_level > 0 && area == Area::Land && st.repel_ok {
        bump(st, 300.0 * ctx.p.text_factor);
        let mut left = steps;
        while left > 0 {
            if st.repel_left == 0 {
                let (dur, price) = match st.max_tier {
                    0 => (100, 350),
                    1 => (200, 500),
                    _ => (250, 700),
                };
                if ctx.p.money_model {
                    if st.money < price {
                        break;
                    }
                    st.money -= price;
                }
                bump(st, REPEL_USE * ctx.p.text_factor);
                st.repel_left = dur;
            }
            let take = left.min(st.repel_left);
            st.repel_left -= take;
            left -= take;
            covered += take;
        }
    }
    let covered = covered.min(steps);
    let mut used = 0;
    for (n, repel) in [(covered, true), (steps - covered, false)] {
        if n == 0 {
            continue;
        }
        let args = EncArgs {
            site: &h.site,
            area,
            steps: n,
            era,
            skip: &skip,
            stop_when_done: true,
            repel,
            repel_lvl: h.repel_level,
            stop_new,
            pass_catch: true,
        };
        let u = run_encounters(ctx, st, rng, &args);
        used += u;
        bump(st, u as f64 * per_step);
        if u < n {
            break;
        }
    }
    if safari {
        st.safari_new += st.caught.count() - c_before;
        if area == Area::Land {
            st.safari_steps = st.safari_steps.saturating_sub(used);
        }
        let stop = ctx.p.safari_stop > 0 && st.safari_new >= ctx.p.safari_stop;
        if stop {
            st.safari_done = true;
        }
        // PLAN `safari_drain`: the counter ran out in the drain hunt, the game warps the player to the gate
        // (`SafariZone_EventScript_TimesUp`, `safari_zone.c:42-53`): the route's walk back out is not walked.
        if !ctx.p.safari_drain.is_empty()
            && ctx.p.safari_drain == h.site
            && area == Area::Land
            && st.safari_steps == 0
        {
            let back =
                ctx.p.safari_exit_tiles as f64 * Era::Run.frames_per_tile() * ctx.p.path_overhead;
            st.clock = st.clock.saturating_sub(back.round() as u64);
            st.safari_done = true;
        }
        if stop || st.safari_steps == 0 || st.safari_balls == 0 {
            st.safari_new = 0;
            bump(st, SAFARI_EXIT as f64 * ctx.p.text_factor);
            st.safari_open = false;
        }
    }
}

/// Counts the encounter rolls of `steps` steps (steps in the cooldown pass the dice at 5%, so
/// they cost 0.05 of a roll) against the quiet window; the last of the steps is the roll that hit.
/// Returns `Some(suppress)` for an encounter that falls inside the window of a quiet seed, `None`
/// when it falls outside (or no table is active). Roll counting per step is a guess: the real
/// stream index also moves with the 60% tile gate (journal 2026-11-13-12-30).
fn quiet_account(st: &mut State, steps: u64, cooldown: u64) -> Option<bool> {
    if st.quiet_hits_left == u32::MAX || st.quiet_rolls >= st.quiet_window {
        return None;
    }
    let cd = steps.min(cooldown) as f64;
    let rolls = cd * 0.05 + (steps as f64 - cd);
    let inside = st.quiet_rolls + rolls <= st.quiet_window;
    st.quiet_rolls += rolls;
    if !inside {
        return None;
    }
    if st.quiet_hits_left == 0 {
        Some(true)
    } else {
        st.quiet_hits_left -= 1;
        Some(false)
    }
}

/// The `manip` starter: a planned (nature, IVs) when every press hit, else an independent random
/// draw that is kept only if it happens to meet the tier. Returns (kept, strength, IVs, nature).
fn manip_starter(
    ctx: &Ctx,
    draw: &crate::manip::AttemptDraw,
    rng: &mut Rng,
) -> (bool, f64, [u32; 6], u8) {
    let mp = &ctx.p.manip;
    let (ivs, nat) = if draw.planned {
        crate::manip::sample_good(mp, rng)
    } else {
        let mut g = frlg_rng::Rng(rng.next_u64() as u32);
        let genome = frlg_mon::create::gift_mon(&mut g);
        let iv = genome.ivs;
        (
            [
                iv.hp as u32,
                iv.atk as u32,
                iv.def as u32,
                iv.spe as u32,
                iv.spa as u32,
                iv.spd as u32,
            ],
            genome.nature(),
        )
    };
    let squirtle = frlg_mon::species::by_id(frlg_mon::species::SQUIRTLE).expect("base stats");
    let s50 = calc_stats(
        squirtle,
        frlg_mon::stats::Ivs {
            hp: ivs[0] as u8,
            atk: ivs[1] as u8,
            def: ivs[2] as u8,
            spe: ivs[3] as u8,
            spa: ivs[4] as u8,
            spd: ivs[5] as u8,
        },
        50,
        nat,
    );
    let neutral = calc_stats(
        squirtle,
        frlg_mon::stats::Ivs {
            hp: 15,
            atk: 15,
            def: 15,
            spe: 15,
            spa: 15,
            spd: 15,
        },
        50,
        0,
    );
    let score = |s: &frlg_mon::stats::Stats| s.spa as f64 + 0.6 * s.hp as f64;
    let strength = (score(&s50) / score(&neutral)).clamp(0.85, 1.15);
    let kept = draw.planned || crate::manip::qualifies(mp, &ivs, nat);
    (kept, strength, ivs, nat)
}

fn starter_ok(ctx: &Ctx, rng: &mut Rng) -> (bool, f64, [u32; 6], u8) {
    let squirtle = frlg_mon::species::by_id(match ctx.p.starter.as_str() {
        "bulbasaur" => frlg_mon::species::BULBASAUR,
        "charmander" => frlg_mon::species::CHARMANDER,
        _ => frlg_mon::species::SQUIRTLE,
    })
    .expect("starter base stats");
    let mut g = frlg_rng::Rng(rng.next_u64() as u32);
    let genome = frlg_mon::create::gift_mon(&mut g);
    let s5 = calc_stats(squirtle, genome.ivs, 5, genome.nature());
    let total = s5.hp as u32
        + s5.atk as u32
        + s5.def as u32
        + s5.spe as u32
        + s5.spa as u32
        + s5.spd as u32;
    let s50 = calc_stats(squirtle, genome.ivs, 50, genome.nature());
    let base = calc_stats(
        squirtle,
        frlg_mon::stats::Ivs {
            hp: 15,
            atk: 15,
            def: 15,
            spe: 15,
            spa: 15,
            spd: 15,
        },
        50,
        0,
    );
    let score = |s: &frlg_mon::stats::Stats| s.spa as f64 + 0.6 * s.hp as f64;
    let strength = (score(&s50) / score(&base)).clamp(0.85, 1.15);
    let m = &ctx.p.starter_min_stats;
    let stats_ok = [s5.hp, s5.atk, s5.def, s5.spe, s5.spa, s5.spd]
        .iter()
        .zip(m)
        .all(|(&v, &lo)| v as u32 >= lo);
    let ok = total >= ctx.p.starter_min_total && stats_ok;
    let iv = genome.ivs;
    (
        ok,
        strength,
        [
            iv.hp as u32,
            iv.atk as u32,
            iv.def as u32,
            iv.spe as u32,
            iv.spa as u32,
            iv.spd as u32,
        ],
        genome.nature(),
    )
}

/// Runs one attempt. `target` is the active T in frames; `None` disables resets.
pub fn attempt(
    ctx: &Ctx,
    seed: u64,
    idx: u64,
    target: Option<u64>,
    mut rec: Option<&mut Vec<SegStats>>,
    mut split_clock: Option<&mut Vec<u64>>,
    mut probe: Option<&mut Probe>,
) -> Outcome {
    let mut st = State {
        target,
        league_t0: 0,
        clock: 0,
        off: 0,
        exp: exp_for_level(Growth::MediumSlow, 5),
        carry: 5,
        strength: 1.0,
        power: 0.0,
        caught: Caught::default(),
        abandoned: false,
        safari_open: false,
        safari_steps: 0,
        safari_balls: 0,
        // `decompiled/src/new_game.c:128`, five Poké Balls from Oak
        // (`PalletTown_ProfessorOaksLab/scripts.inc:660`).
        money: 3000,
        balls: [5, 0, 0],
        max_tier: 0,
        repel_left: 0,
        repel_ok: false,
        awakenings: 0,
        cures: 0,
        coin_case: false,
        ivs: [15; 6],
        nature: 0,
        quiet_hits_left: u32::MAX,
        quiet_rolls: 0.0,
        quiet_window: 0.0,
        evs: [0; 6],
        evs_real: [0; 6],
        mewtwo_evs: [0; 6],
        mewtwo_evs_real: [0; 6],
        glasses: false,
        persim: false,
        ice: false,
        leftovers: false,
        itemfinder: false,
        mystic: false,
        hp_frac: 1.0,
        risk_hp: 1.0,
        risk_real: 1.0,
        free_heal: false,
        torrent_lost: false,
        helpers_dead: 0,
        badges: 0,
        league_items: false,
        heal_items: 0,
        lg_fr: 0,
        lg_max: 0,
        lg_fh: 0,
        mewtwo_hp: 1.0,
        mewtwo_exp: 0,
        log_fights: probe.is_some(),
        candies_eaten: 0,
        fight_log: vec![],
        seg_i: 0,
        poison_acc: 0.0,
        catch_lv: [0; 152],
        catch_seq: [0; 152],
        bodies: [0; 152],
        src: [0; 152],
        cur_src: 0,
        stones: [0; 5],
        ball_tier: None,
        catch_n: 0,
        pc_paid: 0,
        safari_new: 0,
        safari_done: false,
        whiteouts: 0,
        game: GameRng::from_u32(0),
        slots: vec![],
        slot_pp: vec![],
        learned_to: 5,
        carry_status: Status1::None,
        x_stock: [0; 7],
        mewtwo: None,
        mewtwo_moves: vec![],
        mewtwo_pp: vec![],
        mewtwo_bonus: vec![],
        pp_ups: 0,
        ethers: 0,
        max_ethers: 0,
        evo_checked: false,
        fodder: 0,
        taught_once: vec![],
        elixirs: 0,
        max_elixirs: 0,
        revives: 0,
        candies: 0,
        potions: 0,
        free_potions: 0,
        steer_to: None,
        steer_hp: 0,
        pc_seg: None,
        pc_catch_n: 0,
        slot_bonus: vec![],
    };
    let manip = ctx.p.starter_model == "manip";
    let mut draw = crate::manip::AttemptDraw::default();
    if manip {
        draw = crate::manip::draw(&ctx.p.manip, &mut Rng::new(seed, idx, 0x4D41_4E49));
        // Power-on to the title press and the wait for the table's frame: off the timer, in the wall.
        st.off += (draw.pre_s * FPS).round() as u64;
        if draw.quiet {
            st.quiet_hits_left = ctx.p.manip.quiet_max_hits;
            // The density table (`ManipParams::quiet_p`, doc 3.4) is for hits within 60 rate
            // tests; a wider window with the same density would be a free lunch (plan 384 descent:
            // window 150 "won" 1.7 h that way), so the window is capped at the table's.
            st.quiet_window = ctx.p.manip.quiet_window.min(60) as f64;
        }
    }
    got(&mut st, ctx.dex.species[ctx.starter_line[0]].dex, 5);
    // The starter's level-5 moves (`level_up_learnsets.h`).
    for &(l, m) in &ctx.dex.species[ctx.starter_line[0]].learnset {
        if l <= 5 && !st.slots.contains(&m) {
            st.slots.push(m);
            st.slot_pp.push(ctx.dex.moves[m].pp as i32);
        }
    }
    // Fidelity fit: the speed factor of every WR split window this attempt (`Params::win_cv`),
    // one mean-preserving lognormal draw per window, applied to each segment in it.
    let win_f: Vec<f64> = if ctx.p.win_cv > 0.0 {
        let mut ends = crate::wrsplits::window_ends(ctx.segs);
        ends.sort_unstable();
        let sigma = ctx.p.win_cv;
        let mut f = vec![1.0; ctx.segs.len()];
        let mut start = 0;
        for &e in &ends {
            let mut r = Rng::new(seed, idx, hash_str(&ctx.segs[e].name) ^ 0x31D0_C0DE);
            // Box-Muller.
            let (u1, u2) = (1.0 - r.f64(), r.f64());
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let fac = (sigma * z - sigma * sigma / 2.0).exp();
            for x in &mut f[start..=e] {
                *x = fac;
            }
            start = e + 1;
        }
        f
    } else {
        vec![]
    };
    // Seconds behind the WR at the last WR split passed: the pace gate of the skip lever (`Params::cond_skips`).
    let mut behind = 0.0f64;
    let cut_table: Option<&Vec<f64>> = target.and_then(|t| {
        ctx.cut_tables
            .iter()
            .find(|(tt, _)| *tt == t)
            .map(|(_, v)| v)
    });
    let adj_table: Option<&Vec<(u8, f64, f64)>> =
        target.and_then(|t| ctx.cut_adj.iter().find(|(tt, _)| *tt == t).map(|(_, v)| v));
    let mut start = 0usize;
    let mut reloads = 0u32;
    let mut ckpt: Option<(usize, State)> = None;
    'attempt: loop {
        for (i, seg) in ctx.segs.iter().enumerate().skip(start) {
            let salt = (reloads as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let mut rng = Rng::new(seed, idx, hash_str(&seg.name) ^ salt);
            // The game's stream, re-keyed per segment (see `GameRng`).
            st.game = GameRng::from_u32(rng.next_u64() as u32);
            let before = st.clock;
            let wo_before = st.whiteouts;
            st.seg_i = i;
            if exp_trace() {
                eprintln!(
                    "exp-trace {idx} {} {} {} {}",
                    seg.name, st.carry, st.exp, st.clock
                );
            }
            if seg.name == "indigo-heal" || seg.name == "league-entry" {
                st.league_t0 = st.clock;
            }
            if ctx.p.strength_carrier && seg.name == "victory-road" {
                let how = strength_carrier(ctx, &mut st);
                if std::env::var_os("FRLG_SIM_TRACE").is_some() {
                    eprintln!("strength-carrier {how}");
                }
            }
            // The Torrent phase walks past the Pokémon Centers it would have stopped at.
            // PLAN `torrent_heals`: these Centers are visited even in the Torrent phase.
            let skip_heal = seg.name.starts_with("center-heal@")
                && ctx.torrent_end.is_some_and(|e| i <= e)
                && !st.torrent_lost
                && !ctx
                    .p
                    .torrent_heals
                    .iter()
                    .any(|h| seg.name.strip_prefix("center-heal@") == Some(h.as_str()));
            // Save point (`Params::reload_save_at`): the human saves before this segment.
            if target.is_some()
                && !ctx.p.reload_save_at.is_empty()
                && ckpt.as_ref().is_none_or(|c| c.0 != i)
                && ctx.p.reload_save_at.iter().any(|n| *n == seg.name)
            {
                bump(&mut st, ctx.p.reload_save_frames as f64 * ctx.p.text_factor);
                ckpt = Some((i, st.clone()));
            }
            // Fidelity fit: an unsalvaged death in this segment (evaluation runs only).
            if target.is_some() && !ctx.p.fatal.is_empty() {
                if let Some(&p) = ctx.p.fatal.get(&seg.name) {
                    if Rng::new(seed, idx, hash_str(&seg.name) ^ 0xFA7A_1D1E ^ salt).chance(p) {
                        if let Some(ci) = try_reload(
                            ctx,
                            &mut st,
                            &ckpt,
                            &mut reloads,
                            ctx.p.reload_fatal_frames as f64,
                        ) {
                            start = ci;
                            continue 'attempt;
                        }
                        st.abandoned = true;
                        st.whiteouts += 1;
                        if let Some(r) = rec.as_deref_mut() {
                            r[i].abandons += 1;
                            r[i].whiteouts += 1;
                        }
                        break;
                    }
                }
            }
            let skipped = behind > ctx.cond_skip[i];
            if skipped {
                if let Some(r) = rec.as_deref_mut() {
                    r[i].skips += 1;
                }
            }
            match &seg.kind {
                _ if skipped => {}
                Kind::Text { .. } => {
                    if !skip_heal {
                        // `floor_frames`: `frames` x `text_factor`, or a MODEL CHANGE `fly_frames`.
                        bump(&mut st, ctx.floor_frames(seg));
                    }
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    bump(&mut st, mistakes(ctx, &mut rng, seg.inputs as f64 / 100.0));
                }
                Kind::Walk { tiles, era, enc } => {
                    // `floor_frames`: tiles x frames per tile x `path_overhead`, or a MODEL CHANGE `vod_frames`.
                    bump(&mut st, ctx.floor_frames(seg));
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    bump(&mut st, mistakes(ctx, &mut rng, *tiles as f64 / 100.0));
                    if let Some(e) = enc {
                        let covered = use_repels(ctx, &mut st, seg, *tiles);
                        let guarded =
                            (e.steps as u64 * covered as u64 / (*tiles).max(1) as u64) as u32;
                        for (steps, repel) in [(guarded, true), (e.steps - guarded, false)] {
                            let args = EncArgs {
                                site: &e.site,
                                area: e.area,
                                steps,
                                era: *era,
                                skip: &[],
                                stop_when_done: false,
                                repel,
                                repel_lvl: 0,
                                stop_new: 0,
                                pass_catch: !ctx.p.pass_off.contains(&seg.name),
                            };
                            run_encounters(ctx, &mut st, &mut rng, &args);
                        }
                    }
                }
                Kind::Wander { enc, era } => {
                    let covered = use_repels(ctx, &mut st, seg, enc.steps);
                    let guarded = covered.min(enc.steps);
                    for (steps, repel) in [(guarded, true), (enc.steps - guarded, false)] {
                        let args = EncArgs {
                            site: &enc.site,
                            area: enc.area,
                            steps,
                            era: *era,
                            skip: &[],
                            stop_when_done: false,
                            repel,
                            repel_lvl: 0,
                            stop_new: 0,
                            pass_catch: !ctx.p.pass_off.contains(&seg.name),
                        };
                        run_encounters(ctx, &mut st, &mut rng, &args);
                    }
                }
                Kind::Trainer {
                    party,
                    intro,
                    adv,
                    money,
                    full,
                    ai,
                    heals,
                    cures,
                } => {
                    let heals = &if ctx.p.ai_cure_items {
                        heals.saturating_sub(*cures)
                    } else {
                        *heals
                    };
                    if ctx.p.battle_sim && !full.is_empty() {
                        let foes: Vec<_> = full
                            .iter()
                            .map(|m| crate::battle::enemy_mon(ctx.dex, m))
                            .collect();
                        let args = BattleArgs {
                            name: &seg.name,
                            foes: &foes,
                            trainer: true,
                            intro: *intro,
                            ai: *ai,
                            heals: *heals,
                            yield_: None,
                            league: is_league(&seg.name),
                            wild: false,
                        };
                        if !sim_battle(ctx, &mut st, &mut rng, &args) {
                            break;
                        }
                    } else {
                        let enemies = enemy_list(ctx, &st, party);
                        let hand = if ctx.p.type_strength > 0.0 { 1.0 } else { *adv };
                        if !battle(ctx, &mut st, &mut rng, &enemies, true, *intro, hand) {
                            break;
                        }
                    }
                    st.money += *money;
                }
                Kind::Parties {
                    parties,
                    money,
                    fulls,
                    ais,
                } => {
                    let mut ok = true;
                    for (pi, p) in parties.iter().enumerate() {
                        if ctx.p.battle_sim && fulls.get(pi).is_some_and(|f| !f.is_empty()) {
                            let foes: Vec<_> = fulls[pi]
                                .iter()
                                .map(|m| crate::battle::enemy_mon(ctx.dex, m))
                                .collect();
                            let args = BattleArgs {
                                name: &seg.name,
                                foes: &foes,
                                trainer: true,
                                intro: 600,
                                ai: ais.get(pi).copied().unwrap_or(1),
                                heals: 0,
                                yield_: None,
                                league: is_league(&seg.name),
                                wild: false,
                            };
                            if !sim_battle(ctx, &mut st, &mut rng, &args) {
                                ok = false;
                                break;
                            }
                            continue;
                        }
                        let enemies = enemy_list(ctx, &st, p);
                        if !battle(ctx, &mut st, &mut rng, &enemies, true, 600, 1.0) {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        break;
                    }
                    st.money += *money;
                }
                Kind::Group {
                    n,
                    mons,
                    level,
                    yield_,
                } => {
                    if hazard_death(ctx, &mut st, &mut rng, &seg.name) {
                        break;
                    }
                    if !group_battle(ctx, &mut st, &mut rng, *n, *mons, *level, *yield_) {
                        break;
                    }
                    // Class value 5 is a guess; the formula is `battle_script_commands.c:5373`.
                    st.money += n * 4 * level * 5;
                }
                Kind::Hunt { hunt: h, era } => hunt(ctx, &mut st, &mut rng, h, *era),
                Kind::Grind {
                    battles,
                    level,
                    yield_,
                } => {
                    for _ in 0..*battles {
                        let gap = rng.geometric(0.07) + 1;
                        bump(
                            &mut st,
                            gap as f64 * Era::Run.frames_per_tile() + ctx.enc_intro(),
                        );
                        let t = turns_per_mon(
                            *level as f64,
                            effective_level(&st, ctx.p),
                            st.strength,
                            1.0,
                        );
                        bump(&mut st, t * ctx.turn() + ctx.mon_overhead());
                        let g = add_exp(ctx, &mut st, exp_gain(*yield_ as u8, *level as u8, false));
                        bump(&mut st, g as f64 * ctx.level_up());
                    }
                }
                Kind::OakCheck => {
                    if let Some(pr) = probe.as_deref_mut() {
                        pr.oak_count = st.caught.count();
                        pr.oak_missing = (1..=151).filter(|&d| !st.caught.has(d)).collect();
                    }
                    // Once the list is exhausted the human goes round it again (at most 6 times).
                    let rounds = if ctx.p.fill.is_empty() { 0 } else { 6 };
                    for (hi, h) in (0..rounds).flat_map(|_| ctx.p.fill.iter().enumerate()) {
                        if st.caught.count() >= 60 {
                            break;
                        }
                        let (c0, n0) = (st.clock, st.caught.count());
                        // The Power Plant forbids cycling (`allow_cycling` in its map.json).
                        let era = if ctx.p.hunt_bike && ctx.p.bike && h.site != "MAP_POWER_PLANT" {
                            Era::Bike
                        } else {
                            Era::Run
                        };
                        hunt(ctx, &mut st, &mut rng, h, era);
                        if let Some(pr) = probe.as_deref_mut() {
                            if pr.fill.len() <= hi {
                                pr.fill.resize(hi + 1, (0.0, 0.0, 0));
                            }
                            let e = &mut pr.fill[hi];
                            e.0 += (st.clock - c0) as f64;
                            e.1 += (st.caught.count() - n0) as f64;
                            e.2 += 1;
                        }
                        if let Some(t) = target {
                            let left = if ctx.p.dex_resets {
                                ctx.fill_slope * (60.0 - st.caught.count() as f64).max(0.0)
                            } else {
                                0.0
                            };
                            let rem = ctx.floor_after[i] as f64
                                + ctx.aggr[i]
                                    * (ctx.mean_after[i] + left - ctx.floor_after[i] as f64)
                                        .max(0.0);
                            if st.caught.count() < 60 && st.clock as f64 + rem > t as f64 {
                                if let Some(r) = rec.as_deref_mut() {
                                    r[i].resets += 1;
                                }
                                return Outcome {
                                    wall: st.clock + st.off,
                                    run: st.clock,
                                    finished: false,
                                    caught: st.caught.count(),
                                    whiteouts: st.whiteouts,
                                    off: st.off,
                                    end_seg: st.seg_i as u32,
                                    kind: 1,
                                };
                            }
                        }
                    }
                    if st.caught.count() < 60 {
                        if let Some(pr) = probe.as_deref_mut() {
                            pr.missing = (1..=151).filter(|&d| !st.caught.has(d)).collect();
                        }
                        st.abandoned = true;
                        break;
                    }
                }
                Kind::Stones => {
                    for s in &ctx.p.stones {
                        let want = format!("ITEM_{s}_STONE");
                        // Bought in Celadon (Fire/Thunder/Water/Leaf) or picked up (Moon). guess frames.
                        bump(
                            &mut st,
                            if s == "MOON" { 500.0 } else { 1500.0 } * ctx.p.text_factor,
                        );
                        'found: for sp in &ctx.dex.species {
                            for (k, param, target) in &sp.evo {
                                if k == "EVO_ITEM"
                                    && *param == want
                                    && sp.dex != 0
                                    && st.caught.has(sp.dex)
                                {
                                    let t = ctx.dex.species[*target].dex;
                                    if !st.caught.has(t) {
                                        let lv = st.catch_lv[sp.dex as usize] as u32;
                                        got(&mut st, t, lv);
                                        bump(&mut st, 700.0 * ctx.p.text_factor);
                                        break 'found;
                                    }
                                }
                            }
                        }
                    }
                }
                Kind::Special { special } if special.kind == "fight" => {
                    // MODEL CHANGE `marowak_fight`: a forced one-off wild battle (the Marowak ghost,
                    // `PokemonTower_6F/scripts.inc:9` `setwildbattle SPECIES_MAROWAK, 30`).
                    bump(&mut st, special.frames as f64 * ctx.p.text_factor);
                    for n in &special.species {
                        let foe = wild_mon(ctx, &mut st, ctx.dex.id(n), special.level);
                        let ok = sim_battle(
                            ctx,
                            &mut st,
                            &mut rng,
                            &BattleArgs {
                                name: &seg.name,
                                foes: &[foe],
                                trainer: false,
                                intro: 0,
                                ai: 0,
                                heals: 0,
                                yield_: None,
                                league: false,
                                wild: true,
                            },
                        );
                        if !ok || st.abandoned {
                            break;
                        }
                    }
                }
                Kind::Special { special }
                    if special.requires.iter().any(|r| {
                        let d = ctx.dex.species[ctx.dex.id(r)].dex;
                        st.bodies[d as usize] == 0
                    }) => {}
                Kind::Special { special }
                    if special.kind == "buy"
                        && (special
                            .species
                            .iter()
                            .all(|n| st.caught.has(ctx.dex.species[ctx.dex.id(n)].dex))
                            || (ctx.p.money_model && st.money < buy_cost(ctx, special.cost))) => {}
                Kind::Special { special } => {
                    if special.kind == "buy" && ctx.p.money_model {
                        st.money -= buy_cost(ctx, special.cost);
                    }
                    if special.kind == "buy" && ctx.p.coin_case_frames > 0 && !st.coin_case {
                        // MODEL CHANGE `coin_case_frames`: the restaurant's Coin Case first.
                        st.coin_case = true;
                        bump(&mut st, ctx.p.coin_case_frames as f64 * ctx.p.text_factor);
                    }
                    if special.kind == "sell" && ctx.p.money_model {
                        // Items sold at a Poke Mart: `cost` is the money received.
                        st.money += special.cost;
                    }
                    if special.kind == "trade" {
                        let req: Vec<u32> = special
                            .requires
                            .iter()
                            .map(|r| ctx.dex.species[ctx.dex.id(r)].dex)
                            .collect();
                        pc_visit(ctx, &mut st, &req, &special.after);
                        for &d in &req {
                            st.bodies[d as usize] -= 1;
                        }
                    }
                    bump(&mut st, special.frames as f64 * ctx.p.text_factor);
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    for n in &special.species {
                        let id = ctx.dex.id(n);
                        let sp = &ctx.dex.species[id];
                        if special.kind == "gift"
                            || special.kind == "trade"
                            || special.kind == "buy"
                        {
                            st.cur_src = match special.kind.as_str() {
                                "trade" => 2,
                                "buy" => 3,
                                _ => 1,
                            };
                            got(&mut st, sp.dex, special.level);
                            st.cur_src = 0;
                        } else {
                            bump(&mut st, ctx.enc_intro() + react(ctx, &mut rng) as f64);
                            let turns = (((1.0 - special.hp_frac) / 0.25).ceil() as u32).max(1);
                            bump(&mut st, turns as f64 * ctx.turn());
                            if rng.chance((0.04 * turns as f64).min(0.9)) {
                                continue;
                            }
                            let ball = match special.ball.as_str() {
                                "poke" => 10,
                                "great" => 15,
                                _ => 20,
                            };
                            if throw_balls(
                                ctx,
                                &mut st,
                                &mut rng,
                                sp.catch_rate,
                                special.hp_frac,
                                Status1::None,
                                Some(ball),
                                special.max_throws,
                            ) {
                                got(&mut st, sp.dex, special.level);
                                if sp.dex != 0
                                    && (special.level as f64 * ctx.p.power_bonus) > st.power
                                    && special.level >= 60
                                {
                                    st.power = special.level as f64 * ctx.p.power_bonus;
                                }
                            }
                        }
                    }
                }
                Kind::Evolve { evo } => {
                    evolve(ctx, &mut st, &mut rng, evo, seg);
                }
                Kind::Mart { mart } if mart.topup => {
                    // MODEL CHANGE `Mart::topup`: top the bag up to the stop's targets; the visit is paid
                    // only when something is bought.
                    let mut bought = false;
                    for (t, want) in [(0, mart.poke), (1, mart.great), (2, mart.ultra)] {
                        let qty = want
                            .saturating_sub(st.balls[t])
                            .min(st.money / BALL_PRICE[t]);
                        st.money -= qty * BALL_PRICE[t];
                        st.balls[t] += qty;
                        bought |= qty > 0;
                    }
                    if ctx.potion_bag_from.is_some() {
                        let price = potion_price(potion_hp(ctx, i, st.carry, false));
                        let qty = mart
                            .potions
                            .saturating_sub(st.potions)
                            .min(st.money / price);
                        st.money -= qty * price;
                        st.potions += qty;
                        bought |= qty > 0;
                    }
                    if bought {
                        bump(&mut st, MART as f64 * ctx.p.text_factor);
                        bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    }
                }
                Kind::Mart { mart } => {
                    bump(&mut st, MART as f64 * ctx.p.text_factor);
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    for (t, want) in [(0, mart.poke), (1, mart.great), (2, mart.ultra)] {
                        if t > st.max_tier {
                            continue;
                        }
                        let qty = want.min(st.money / BALL_PRICE[t]);
                        st.money -= qty * BALL_PRICE[t];
                        st.balls[t] += qty;
                    }
                }
                Kind::TeleportOrWalk { frames, tiles } => {
                    // Abra is national dex 63.
                    if st.caught.has(63) {
                        bump(&mut st, *frames as f64 * ctx.p.text_factor);
                    } else if let Some(&f) = ctx.p.vod_frames.get(&seg.name) {
                        // MODEL CHANGE `vod_frames`: the walk as the WR VOD times it (no Abra yet).
                        bump(&mut st, f);
                    } else {
                        bump(
                            &mut st,
                            *tiles as f64 * Era::Run.frames_per_tile() * ctx.p.path_overhead,
                        );
                    }
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                }
                Kind::Mewtwo => {
                    bump(&mut st, 1500.0 * ctx.p.text_factor);
                    bump(&mut st, ctx.throw_cost() + ctx.catch_end());
                    got(&mut st, 150, 70);
                    st.power = 70.0 * ctx.p.power_bonus;
                    // Roll its IVs and nature like any wild mon (`pokemon.c`, `CreateBoxMon`:
                    // personality then two IV words); its default level-70 moves are
                    // Swift/Recover/Safeguard/Psychic (`level_up_learnsets.h`), extended by
                    // `tms` entries with `mewtwo: true`.
                    let mut g = st.game;
                    let pid = (g.roll() as u32) | ((g.roll() as u32) << 16);
                    let iv1 = g.roll() as u32;
                    let iv2 = g.roll() as u32;
                    st.game = g;
                    let ivs = [
                        iv1 & 0x1F,
                        (iv1 >> 5) & 0x1F,
                        (iv1 >> 10) & 0x1F,
                        iv2 & 0x1F,
                        (iv2 >> 5) & 0x1F,
                        (iv2 >> 10) & 0x1F,
                    ];
                    st.mewtwo = Some((ivs, (pid % 25) as u8));
                    st.mewtwo_exp = slow_exp(70);
                    let k = st.candies.min(ctx.p.mewtwo_candies);
                    if k > 0 && ctx.p.mewtwo_levels {
                        st.candies -= k;
                        st.mewtwo_exp = slow_exp(70 + k);
                        bump(&mut st, 400.0 * k as f64 * ctx.p.text_factor);
                    }
                    if st.mewtwo_moves.is_empty() {
                        st.mewtwo_moves = ctx.dex.default_moves(ctx.dex.id("SPECIES_MEWTWO"), 70);
                    }
                    restore_mewtwo_pp(ctx, &mut st);
                    let f = apply_pp_ups(ctx, &mut st);
                    bump(&mut st, f);
                }
                Kind::Teach { teach } => {
                    let mv = ctx.dex.move_id(&teach.mv);
                    let mewtwo_sp = ctx.dex.id("SPECIES_MEWTWO");
                    let learner = if teach.mewtwo {
                        mewtwo_sp
                    } else {
                        carry_species(ctx, st.carry)
                    };
                    // MODEL CHANGE `Teach::prize` with `coin_case_frames`: a Game Corner TM is paid in coins
                    // bought in whole purchases, and needs the Coin Case.
                    let prize = teach.prize && ctx.p.coin_case_frames > 0;
                    let cost = if prize {
                        buy_cost(ctx, teach.cost)
                    } else {
                        teach.cost
                    };
                    let affordable = !ctx.p.money_model || st.money >= cost;
                    let is_hm = matches!(
                        ctx.dex.moves[mv].name.as_str(),
                        "MOVE_CUT"
                            | "MOVE_FLY"
                            | "MOVE_SURF"
                            | "MOVE_STRENGTH"
                            | "MOVE_FLASH"
                            | "MOVE_ROCK_SMASH"
                            | "MOVE_WATERFALL"
                            | "MOVE_DIVE"
                    );
                    let possible = can_learn(ctx, learner, mv)
                        && (!teach.mewtwo || st.mewtwo.is_some())
                        && (is_hm || !st.taught_once.contains(&mv));
                    if affordable && possible {
                        if ctx.p.money_model {
                            st.money -= cost;
                        }
                        if prize && !st.coin_case {
                            st.coin_case = true;
                            bump(&mut st, ctx.p.coin_case_frames as f64 * ctx.p.text_factor);
                        }
                        if !is_hm {
                            st.taught_once.push(mv);
                        }
                        bump(&mut st, teach.frames as f64 * ctx.p.text_factor);
                        bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                        if teach.mewtwo {
                            if st.mewtwo_moves.len() < 4 {
                                st.mewtwo_moves.push(mv);
                                st.mewtwo_pp.push(ctx.dex.moves[mv].pp as i32);
                                st.mewtwo_bonus.resize(st.mewtwo_moves.len() - 1, 0);
                                st.mewtwo_bonus.push(0);
                            } else {
                                // Psychic and Recover stay; the weakest other slot goes
                                // (Safeguard, then Swift, then an earlier teach).
                                let keep: Vec<usize> = if ctx.p.mewtwo_keep.is_empty() {
                                    vec![
                                        ctx.dex.move_id("MOVE_PSYCHIC"),
                                        ctx.dex.move_id("MOVE_RECOVER"),
                                    ]
                                } else {
                                    ctx.p
                                        .mewtwo_keep
                                        .iter()
                                        .map(|m| ctx.dex.move_id(m))
                                        .collect()
                                };
                                if let Some(i) = st
                                    .mewtwo_moves
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, m)| !keep.contains(m))
                                    .min_by_key(|(_, &m)| ctx.dex.moves[m].power)
                                    .map(|(i, _)| i)
                                {
                                    st.mewtwo_moves[i] = mv;
                                    st.mewtwo_pp[i] = ctx.dex.moves[mv].pp as i32;
                                    st.mewtwo_bonus.resize(st.mewtwo_moves.len(), 0);
                                    st.mewtwo_bonus[i] = 0;
                                }
                            }
                        } else {
                            offer_move(ctx, &mut st, mv, true);
                        }
                    }
                }
                Kind::Elixir { pickup } => {
                    bump(&mut st, pickup.frames as f64 * ctx.p.text_factor);
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    match pickup.kind.as_str() {
                        "max" => st.max_elixirs += 1,
                        "ether" => st.ethers += 1,
                        "maxether" => st.max_ethers += 1,
                        // Route 11 east gate, 2F aide (`Route11_EastEntrance_2F/scripts.inc:56-73`): the
                        // Itemfinder only at 30 caught Kanto species (`GetPokedexCount` -> `GetKantoPokedexCount`,
                        // `prof_pc.c:23-29`, `pokedex.c:75-93`, `REQUIRED_CAUGHT_MONS 30`); below that the
                        // trip is wasted. `frames` is the whole trip (extra Fly, walks, dialogue).
                        "itemfinder" => {
                            if st.caught.count() >= 30 {
                                st.itemfinder = true;
                            }
                        }
                        // The two underfoot items on the route (Cape Brink PP Max, Route 16 Leftovers) are
                        // found only by using the Itemfinder on the tile (`itemfinder.c:241-246`).
                        "ppmax" if !st.itemfinder => {}
                        "leftovers" if !st.itemfinder => {}
                        "ppup" | "ppmax" => {
                            st.pp_ups += if pickup.kind == "ppmax" { 3 } else { 1 };
                            apply_carry_pp_ups(ctx, &mut st);
                            let f = apply_pp_ups(ctx, &mut st);
                            bump(&mut st, f);
                        }
                        "candy" if ctx.p.candy_early && st.candies_eaten < ctx.p.candy_early_n => {
                            st.candies_eaten += 1;
                            // Eaten on the spot: a level now (400 frames), through the normal exp path
                            // so evolutions and level-up moves happen.
                            let need = exp_for_level(Growth::MediumSlow, (st.carry + 1) as u8)
                                .saturating_sub(st.exp);
                            let g = add_exp(ctx, &mut st, need.max(1));
                            bump(
                                &mut st,
                                400.0 * ctx.p.text_factor + g as f64 * ctx.level_up(),
                            );
                        }
                        "candy" => st.candies += 1,
                        "neverice" => st.ice = true,
                        "leftovers" => st.leftovers = true,
                        // Any heal-to-full item found counts as a League heal (replaces one purchase).
                        "heal" => st.heal_items += 1,
                        "revive" => st.revives += 1,
                        // Vitamins found on the route (`item_effects.h`: +10 EVs, refused at 100 EVs in
                        // the stat; stats recalculated on use). Order HP, Atk, Def, Spe, SpA, SpD.
                        "hpup" | "protein" | "iron" | "carbos" | "calcium" | "zinc" => {
                            let ix = match pickup.kind.as_str() {
                                "hpup" => 0,
                                "protein" => 1,
                                "iron" => 2,
                                "carbos" => 3,
                                "calcium" => 4,
                                _ => 5,
                            };
                            if st.evs_real[ix] < 100 {
                                let mut y = [0u32; 6];
                                y[ix] = 10.min(100 - st.evs_real[ix]);
                                let mut real = st.evs_real;
                                add_evs(ctx, &mut real, y);
                                st.evs_real = real;
                                st.evs = real;
                            }
                        }
                        k if stone_ix(k).is_some() => st.stones[stone_ix(k).unwrap_or(0)] += 1,
                        _ => st.elixirs += 1,
                    }
                }
                Kind::XBuy { xbuy } => {
                    if std::env::var_os("FRLG_SIM_TRACE").is_some() {
                        eprintln!(
                            "STATE xbuy {} money {} x_stock {:?}",
                            seg.name, st.money, st.x_stock
                        );
                    }
                    bump(&mut st, xbuy.frames as f64 * ctx.p.text_factor);
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    // Prices (`items.json`): X Attack 500, X Defend 550, X Speed 350,
                    // X Special 350, Dire Hit 650.
                    // Bought scarcest-first so a short purse fills the specials before
                    // the cheaper nice-to-haves (prices `items.json`).
                    {
                        let qty = if ctx.p.money_model {
                            xbuy.revives.min(st.money / 1500)
                        } else {
                            xbuy.revives
                        };
                        if ctx.p.money_model {
                            st.money -= qty * 1500;
                        }
                        st.revives += qty;
                    }
                    {
                        // Hyper Potions for the League, after the Revives.
                        let price = ctx
                            .dex
                            .item_prices
                            .get("ITEM_HYPER_POTION")
                            .copied()
                            .unwrap_or(1200);
                        let qty = if ctx.p.money_model {
                            xbuy.heals.min(st.money / price)
                        } else {
                            xbuy.heals
                        };
                        if ctx.p.money_model {
                            st.money -= qty * price;
                        }
                        st.heal_items += qty;
                    }
                    for (i, want, price) in [
                        (3usize, xbuy.special, 350),
                        (2, xbuy.speed, 350),
                        (0, xbuy.attack, 500),
                        (1, xbuy.defend, 550),
                        (4, xbuy.dire, 650),
                        (5, xbuy.accuracy, 950),
                        (6, xbuy.guard, 700),
                    ] {
                        let qty = if ctx.p.money_model {
                            want.min(st.money / price)
                        } else {
                            want
                        };
                        if ctx.p.money_model {
                            st.money -= qty * price;
                        }
                        st.x_stock[i] += qty;
                    }
                }
            }
            if st.abandoned {
                if let Some(ci) = try_reload(ctx, &mut st, &ckpt, &mut reloads, 0.0) {
                    start = ci;
                    continue 'attempt;
                }
                if let Some(r) = rec.as_deref_mut() {
                    r[i].abandons += 1;
                    r[i].whiteouts += (st.whiteouts - wo_before) as u64;
                }
                break;
            }
            // Fidelity fit: execution-loss events when the segment completes (`Params::slow`).
            if !ctx.p.slow.is_empty() {
                for (k, ev) in ctx.p.slow.iter().enumerate() {
                    let hit = match ev.seg.strip_suffix('*') {
                        Some(pre) => seg.name.starts_with(pre),
                        None => seg.name == ev.seg,
                    };
                    if !hit {
                        continue;
                    }
                    let mut r = Rng::new(seed, idx, hash_str(&seg.name) ^ (0x510E_0000 + k as u64));
                    if r.chance((ev.p * ctx.p.slow_scale).min(1.0)) {
                        let u = r.f64();
                        bump(&mut st, -(1.0 - u).ln() * ev.mean_s * FPS);
                    }
                }
            }
            // Fidelity fit: execution noise on the WR split windows (`Params::win_cv`).
            if !win_f.is_empty() && !seg.off_timer {
                let d = st.clock.saturating_sub(before) as f64;
                let delta = (d * (win_f[i] - 1.0)).round() as i64;
                st.clock = (st.clock as i64 + delta).max(before as i64) as u64;
            }
            // Marts that sell better balls: Lavender (Great), Fuchsia (Ultra)
            // (`decompiled/data/maps/LavenderTown_Mart/scripts.inc:29`, `FuchsiaCity_Mart/scripts.inc:25`).
            if !ctx.p.mystic_after.is_empty()
                && seg.name == ctx.p.mystic_after
                && !st.mystic
                && (!ctx.p.money_model || st.money >= ctx.p.mystic_cost)
            {
                if ctx.p.money_model {
                    st.money -= ctx.p.mystic_cost;
                }
                st.mystic = true;
                bump(&mut st, ctx.p.mystic_frames as f64 * ctx.p.text_factor);
            }
            // MODEL CHANGE `trash_cans`: the Vermilion Gym switches, after the walk into the gym.
            if ctx.p.trash_cans && seg.name == "surge-walk" {
                let mut r = Rng::new(seed, idx, hash_str("surge-cans"));
                let n = trash_can_checks(&mut r);
                bump(
                    &mut st,
                    ctx.p.cans_fixed as f64 + n as f64 * ctx.p.can_frames as f64,
                );
            }
            // MODEL CHANGE `played_extra`: controller-measured time the segment model misses.
            if let Some(&x) = ctx.p.played_extra.get(&seg.name) {
                bump(&mut st, x);
            }
            match seg.name.as_str() {
                "black-glasses-swap" => {
                    st.glasses = true;
                    st.persim = false;
                }
                // MODEL CHANGE `awakenings`: bought in the Cerulean mart menus of this segment
                // (`CeruleanCity_Mart/scripts.inc:30`, 250 each `items.json`); the WR bag at Cerulean
                // holds three (`video-analysis/stream-2026-11-14-main97.md`). No extra frames: the
                // segment is the runner's measured potion/Persim menu time.
                "persim-and-potions" if ctx.p.awakenings > 0 || ctx.p.persim_held => {
                    // MODEL CHANGE `persim_held`: the berry fetched in this segment is held from here on.
                    st.persim = ctx.p.persim_held && ctx.p.persim && ctx.p.item_swaps;
                    let n = if ctx.p.money_model {
                        ctx.p.awakenings.min(st.money / 250)
                    } else {
                        ctx.p.awakenings
                    };
                    if ctx.p.money_model {
                        st.money -= n * 250;
                    }
                    st.awakenings += n;
                }
                // MODEL CHANGE `ball_shop_from`: the early balls are bought at the Viridian mart once
                // the parcel is delivered (`ViridianCity_Mart/scripts.inc`: Poke Ball), the last shop
                // before Cerulean on this route.
                "deliver" if ctx.ball_shop_from.is_some() && ctx.p.early_balls > 0 => {
                    let qty = if ctx.p.money_model {
                        ctx.p.early_balls.min(st.money / BALL_PRICE[0])
                    } else {
                        ctx.p.early_balls
                    };
                    if ctx.p.money_model {
                        st.money -= qty * BALL_PRICE[0];
                    }
                    st.balls[0] += qty;
                    bump(&mut st, MART as f64 * ctx.p.text_factor);
                }
                "brock" => {
                    st.repel_ok = true;
                    st.badges |= 1;
                }
                "koga-badge" => {
                    st.max_tier = 2;
                    st.badges |= 1 << 4;
                }
                // Thunder Badge boosts the player's battle speed by 10%
                // (`GetWhoStrikesFirst`, `battle_main.c:3444-3446`).
                "surge-badge" => st.badges |= 1 << 2,
                "blaine-badge-and-sevii-scene" => st.badges |= 1 << 6,
                // Giovanni hands over TM26 Earthquake
                // (`ViridianCity_Gym/scripts.inc:28-30`); teaching it is a plan choice via
                // `tms`, not automatic (a taught move displaces a slot).
                "indigo-heal" => {
                    if std::env::var_os("FRLG_SIM_TRACE").is_some() {
                        eprintln!(
                        "STATE indigo-heal money {} x_stock {:?} revives {} fodder {} candies {} carry {} elixirs {} max {} potions {} heals {}",
                        st.money, st.x_stock, st.revives, st.fodder, st.candies, st.carry, st.elixirs, st.max_elixirs, st.potions, st.heal_items
                    );
                    }
                    st.league_items = true;
                    // MODEL CHANGE `league_bag_honest`: the field bag's Hyper Potions still held are League
                    // heals too (the Hyper tier is 200 HP, `item_effects.h:37-40`).
                    if ctx.p.league_bag_honest && potion_hp(ctx, i, st.carry, false) == 200 {
                        st.heal_items += std::mem::take(&mut st.potions);
                    }
                    // Rare Candies kept for the League go down now: a level each, worth the
                    // most exp this late (~400 frames per use).
                    while st.candies > ctx.p.mewtwo_candies && st.carry < 100 {
                        st.candies -= 1;
                        st.carry += 1;
                        st.exp = st
                            .exp
                            .max(exp_for_level(Growth::MediumSlow, st.carry as u8));
                        bump(&mut st, 400.0 * ctx.p.text_factor);
                    }
                    // A Rare Candy runs `CalculateMonStats`: the cached stats pick up every EV gained since
                    // the last level-up (`pokemon.c`, `PokemonUseItemEffects` ITEM3_LEVEL_UP).
                    st.evs = st.evs_real;
                    learn_moves(ctx, &mut st);
                }
                // HM03 Surf is taught to the carry after the Safari trip (the route needs a
                // surfer anyway; `plan.md` HM gates).
                "safari-walk" => {
                    let surf = ctx.dex.move_id("MOVE_SURF");
                    if can_learn(ctx, carry_species(ctx, st.carry), surf) {
                        offer_move(ctx, &mut st, surf, true);
                    }
                }
                "lavender-heal" => st.max_tier = st.max_tier.max(1),
                _ => {}
            }
            // PLAN `full_heals`: Full Heals bought at a mart on this segment's way (Fuchsia: `FuchsiaCity_Mart/
            // scripts.inc:28`, 600 each, `items.json`), one mart visit.
            if ctx.p.full_heals > 0 && seg.name == ctx.p.full_heal_at {
                let price = ctx.p.full_heal_price.max(1);
                let n = if ctx.p.money_model {
                    ctx.p.full_heals.min(st.money / price)
                } else {
                    ctx.p.full_heals
                };
                if ctx.p.money_model {
                    st.money -= n * price;
                }
                st.cures += n;
                bump(&mut st, ctx.p.full_heal_frames * ctx.p.text_factor);
            }
            review_keys(ctx, &mut st, &seg.name);
            if ctx.p.cut_honest {
                cut_carrier(ctx, &mut st, &mut rng, &seg.name);
            }
            // Poison outside battle: 1 HP per 5 steps while walking or reading text.
            if ctx.p.poison_frames_per_hp > 0.0
                && matches!(
                    seg.kind,
                    Kind::Text { .. } | Kind::Walk { .. } | Kind::Wander { .. }
                )
                && matches!(st.carry_status, Status1::Poison | Status1::Toxic(_))
            {
                let d = (st.clock - before) as f64;
                field_poison(ctx, &mut st, d);
            }
            // league-entry is the Indigo Plateau Centre before the rematch chain.
            if !skip_heal && (seg.name.contains("heal") || seg.name == "league-entry") {
                // The party fillers (gift mons, caught fodder) stand revived; five slots
                // beside the carry once the party has filled out (guess: by Misty).
                st.fodder = if st.carry >= 20 { 5 } else { 2 };
                st.hp_frac = 1.0;
                st.carry_status = Status1::None;
                for k in 0..st.slots.len() {
                    st.slot_pp[k] = carry_full_pp(ctx, &st, k);
                }
                restore_mewtwo_pp(ctx, &mut st);
            }
            if seg.off_timer {
                let d = st.clock - before;
                st.off += d;
                st.clock = before;
            }
            // The evolution stat gate: the human opens the summary at the first evolution
            // and resets unless the minima hold (finer IV information than level 5 gives).
            if !st.evo_checked
                && st.carry >= ctx.evo_at[0]
                && ctx.p.evo_min_stats.iter().any(|&m| m > 0)
            {
                st.evo_checked = true;
                bump(&mut st, 250.0 * ctx.p.text_factor);
                let m = crate::battle::make_mon(
                    ctx.dex,
                    carry_species(ctx, st.carry),
                    st.carry,
                    st.ivs,
                    st.evs,
                    st.nature,
                    0,
                );
                let stats = [
                    m.max_hp, m.stat[0], m.stat[1], m.stat[2], m.stat[3], m.stat[4],
                ];
                let ok = stats
                    .iter()
                    .zip(ctx.p.evo_min_stats)
                    .all(|(&v, lo)| v as u32 >= lo);
                if !ok {
                    if let Some(r) = rec.as_deref_mut() {
                        r[i].resets += 1;
                    }
                    return Outcome {
                        wall: st.clock + st.off,
                        run: st.clock,
                        finished: false,
                        caught: st.caught.count(),
                        whiteouts: st.whiteouts,
                        off: st.off,
                        end_seg: st.seg_i as u32,
                        kind: 4,
                    };
                }
            }
            if seg.starter_check && manip {
                let mp = &ctx.p.manip;
                // On-timer: the route's own segments plus the extra human seconds and the idle frames
                // waiting for the table's frame.
                let extra = (mp.s_title_naming + mp.s_naming_ball + mp.s_ball_stats) * FPS;
                // Not `bump`: the game stream is re-keyed at the next segment, and an idle of
                // hundreds of thousands of frames (one-dial tiers) would spin the LCG for nothing.
                st.clock += (extra + draw.idle_frames).round() as u64;
                let (ok, strength, ivs, nat) =
                    manip_starter(ctx, &draw, &mut Rng::new(seed, idx, 0x5748));
                st.strength = strength;
                st.ivs = ivs;
                st.nature = nat;
                if let Some(pr) = probe.as_deref_mut() {
                    pr.phase = Some(crate::manip::PhaseSample {
                        wall_frames: (st.clock + st.off) as f64
                            + if ok { 0.0 } else { mp.s_reset * FPS },
                        pass: ok,
                        hits: draw.title_hit as u8 + draw.naming_hit as u8 + draw.ball_hit as u8,
                        quiet: draw.quiet,
                        planned: draw.planned,
                    });
                }
                if !ok {
                    st.off += (mp.s_reset * FPS).round() as u64;
                    if let Some(r) = rec.as_deref_mut() {
                        r[i].resets += 1;
                    }
                    return Outcome {
                        wall: st.clock + st.off,
                        run: st.clock,
                        finished: false,
                        caught: st.caught.count(),
                        whiteouts: st.whiteouts,
                        off: st.off,
                        end_seg: st.seg_i as u32,
                        kind: 2,
                    };
                }
            } else if seg.starter_check {
                let (mut ok, mut strength, mut ivs, mut nat) =
                    starter_ok(ctx, &mut Rng::new(seed, idx, 0x5747));
                if !ok && ctx.p.starter_save_reroll {
                    // Saved in the lab before choosing: soft reset, Continue, choose again. The timer
                    // keeps running, so every re-roll is run time but not a lost attempt.
                    let mut k = 0u64;
                    while !ok && k < 20_000 {
                        k += 1;
                        bump(&mut st, ctx.p.reroll_frames as f64 * ctx.p.text_factor);
                        (ok, strength, ivs, nat) =
                            starter_ok(ctx, &mut Rng::new(seed, idx, 0x5747 + k * 7919));
                    }
                }
                st.strength = strength;
                st.ivs = ivs;
                st.nature = nat;
                if !ok {
                    let wall = st.clock + st.off;
                    if let Some(r) = rec.as_deref_mut() {
                        r[i].resets += 1;
                    }
                    return Outcome {
                        wall,
                        run: st.clock,
                        finished: false,
                        caught: st.caught.count(),
                        whiteouts: st.whiteouts,
                        off: st.off,
                        end_seg: st.seg_i as u32,
                        kind: 2,
                    };
                }
            }
            if let Some(r) = rec.as_deref_mut() {
                let d = (st.clock - before) as f64;
                r[i].n += 1;
                r[i].sum += d;
                r[i].sumsq += d * d;
                r[i].whiteouts += (st.whiteouts - wo_before) as u64;
                r[i].level_sum += st.carry as f64;
                r[i].caught_sum += st.caught.count() as f64;
                r[i].money_sum += st.money as f64;
                r[i].balls_sum += st.balls.iter().sum::<u32>() as f64;
            }
            if let Some(sc) = split_clock.as_deref_mut() {
                sc[i] = st.clock;
            }
            if ctx.wr_at[i] >= 0.0 {
                behind = st.clock as f64 / FPS - ctx.wr_at[i];
            }
            if let Some(pr) = probe.as_deref_mut() {
                if pr.snap_at == Some(i) {
                    pr.snap = Some(Snapshot {
                        mons: (1..=151u32)
                            .filter(|&d| st.caught.has(d))
                            .map(|d| {
                                (
                                    d,
                                    st.catch_lv[d as usize] as u32,
                                    st.catch_seq[d as usize] as u32,
                                    st.src[d as usize],
                                )
                            })
                            .collect(),
                        candies: st.candies,
                        money: st.money,
                        clock: st.clock,
                        carry: st.carry,
                        carry_state: CarryState {
                            ivs: st.ivs,
                            evs: st.evs,
                            evs_real: st.evs_real,
                            nature: st.nature,
                            slots: st.slots.clone(),
                            x_stock: st.x_stock,
                            glasses: st.glasses,
                            mystic: st.mystic,
                            ice: st.ice,
                            leftovers: st.leftovers,
                            heal_items: st.heal_items,
                            candies: st.candies,
                            mewtwo: st.mewtwo,
                            mewtwo_evs: st.mewtwo_evs,
                            mewtwo_level: mewtwo_level(ctx, &st),
                            mewtwo_moves: st.mewtwo_moves.clone(),
                            elixirs: st.elixirs,
                            max_elixirs: st.max_elixirs,
                            revives: st.revives,
                            badges: st.badges,
                        },
                    });
                }
                pr.fights.append(&mut st.fight_log);
                if pr.caught_at.len() <= i {
                    pr.caught_at.resize(i + 1, 0);
                }
                pr.caught_at[i] = st.caught.count();
                if pr.level_at.len() <= i {
                    pr.level_at.resize(i + 1, 0);
                }
                pr.level_at[i] = st.carry;
            }
            if seg.split {
                if let Some(pr) = probe.as_deref_mut() {
                    pr.split_log.push((i as u32, st.clock, st.clock + st.off));
                    pr.split_x.push((st.caught.count(), st.carry));
                }
                if let Some(t) = target {
                    let cut = cut_table.map(|c| c[i]).filter(|c| !c.is_nan());
                    let mean = if ctx.p.dex_resets && i < ctx.oak_idx {
                        // The fill still to come depends on how many species are in the bag by then.
                        let dex_proj = st.caught.count() as f64 + ctx.gain_after[i];
                        ctx.mean_nofill_after[i]
                            + ctx.fill_base
                            + ctx.fill_slope * (60.0 - dex_proj).max(0.0)
                    } else {
                        ctx.mean_after[i]
                    };
                    let remaining = ctx.floor_after[i] as f64
                        + match cut {
                            Some(_) => 0.0,
                            None => ctx.aggr[i] * (mean - ctx.floor_after[i] as f64).max(0.0),
                        };
                    let shift = match adj_table.map(|a| a[i]) {
                        Some((1, r, k)) => k * (st.caught.count() as f64 - r),
                        Some((2, r, k)) => k * (st.carry as f64 - r),
                        _ => 0.0,
                    };
                    let over_cut = cut.is_some_and(|c| st.clock as f64 > c + shift);
                    if over_cut || st.clock as f64 + remaining > t as f64 {
                        if let Some(r) = rec.as_deref_mut() {
                            r[i].resets += 1;
                        }
                        return Outcome {
                            wall: st.clock + st.off,
                            run: st.clock,
                            finished: false,
                            caught: st.caught.count(),
                            whiteouts: st.whiteouts,
                            off: st.off,
                            end_seg: st.seg_i as u32,
                            kind: 1,
                        };
                    }
                }
            }
        }
        break;
    }
    let finished = !st.abandoned;
    Outcome {
        wall: st.clock + st.off,
        run: st.clock,
        finished,
        caught: st.caught.count(),
        whiteouts: st.whiteouts,
        off: st.off,
        end_seg: st.seg_i as u32,
        kind: if finished { 0 } else { 3 },
    }
}
