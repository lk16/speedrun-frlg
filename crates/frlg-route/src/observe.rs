//! What the route watches: a handful of named RAM probes, each one traceable
//! to a line in the decomp.
//!
//! Addresses come from `pokefirered.sym` rather than being written down here,
//! so a rebuild moves them without editing this file. Struct *offsets* are not
//! in the sym file and are transcribed from the decomp headers below -- each
//! one carries its citation, and every one of them is checked against the
//! running game by `tests/observe.rs`, which is what stops a mis-transcribed
//! offset from turning into a plausible-looking wrong number.

use frlg_emu::{Emu, SymbolTable};

/// `struct Main`, `decompiled/include/main.h:12`.
mod main_off {
    /// `/*0x004*/ MainCallback callback2` -- which screen is running.
    pub const CALLBACK2: u32 = 0x004;
    /// `/*0x02E*/ u16 newKeys` -- keys pressed this frame, after L=A remapping.
    pub const NEW_KEYS: u32 = 0x02E;
    /// `/*0x439*/ u8 inBattle:1` -- second bit of the bitfield byte at 0x439.
    pub const FLAGS: u32 = 0x439;
    pub const IN_BATTLE_BIT: u8 = 1 << 1;
}

/// `struct SaveBlock1`, `decompiled/include/global.h:759`, whose members carry
/// their own offsets in the comments. `struct WarpData` is
/// `decompiled/include/global.h:392`: `s8 mapGroup; s8 mapNum; s8 warpId; s16 x, y;`.
mod sb1_off {
    /// `/*0x0000*/ struct Coords16 pos` -- `s16 x, y` (`include/global.h:161`).
    pub const POS_X: u32 = 0x0000;
    pub const POS_Y: u32 = 0x0002;
    /// `/*0x0004*/ struct WarpData location`, so mapGroup is +0 and mapNum +1.
    pub const MAP_GROUP: u32 = 0x0004;
    pub const MAP_NUM: u32 = 0x0005;
    /// `/*0x0034*/ u8 playerPartyCount` -- the *saved* count, not the live
    /// one. `SavePlayerParty` copies `gPlayerPartyCount` into it
    /// (`decompiled/src/load_save.c:164`) and `LoadPlayerParty` copies it back
    /// out (`:174`), so between saves it is stale. Kept for completeness;
    /// [`Observer::party_count`] reads the live global instead.
    pub const SAVED_PARTY_COUNT: u32 = 0x0034;
    /// `/*0x1000*/ u16 vars[VARS_COUNT]` -- the script variables, indexed from
    /// `VARS_START 0x4000` (`decompiled/include/constants/vars.h:4`).
    pub const VARS: u32 = 0x1000;
}

/// `VARS_START`, the id every `VAR_*` constant is an offset from.
pub const VARS_START: u16 = 0x4000;

/// `VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB`
/// (`decompiled/include/constants/vars.h:137`). 1 once Oak has walked the
/// player in, 2 once he has offered the starters, 3 once the rival has taken
/// his -- which is the state the battle trigger wants.
pub const VAR_OAKS_LAB_SCENE: u16 = 0x4055;

/// `VAR_STARTER_MON` (`decompiled/include/constants/vars.h:98`):
/// 0 Bulbasaur, 1 Squirtle, 2 Charmander.
pub const VAR_STARTER_MON: u16 = 0x4031;

/// `struct BattlePokemon`, `decompiled/include/pokemon.h:170`. Unlike a party
/// `struct Pokemon`, its substructs are not encrypted, so species and HP can be
/// read straight out.
mod battle_mon_off {
    pub const SPECIES: u32 = 0x00;
    /// `/*0x0C*/ u16 moves[MAX_MON_MOVES]`.
    pub const MOVES: u32 = 0x0C;
    /// `/*0x28*/ u16 hp`, `/*0x2A*/ u8 level`, `/*0x2C*/ u16 maxHP`.
    pub const HP: u32 = 0x28;
    pub const LEVEL: u32 = 0x2A;
    pub const MAX_HP: u32 = 0x2C;
    /// `sizeof(struct BattlePokemon)` -- the last member is `/*0x54*/ u32 otId`,
    /// and the sym file's 0x160 for `gBattleMons` is 4 of these.
    pub const SIZE: u32 = 0x58;
}

/// `struct PlayerAvatar`, `decompiled/include/global.fieldmap.h:365`.
mod avatar_off {
    /// `/*0x00*/ u8 flags` -- zero until the overworld sets the player up.
    pub const FLAGS: u32 = 0x00;
    /// `/*0x02*/ u8 runningState` -- 0 not moving, 1 turning, 2 moving.
    pub const RUNNING_STATE: u32 = 0x02;
    /// `/*0x03*/ u8 tileTransitionState` -- 0 settled on a tile, 1 mid-step.
    pub const TILE_TRANSITION_STATE: u32 = 0x03;
    /// `/*0x06*/ bool8 preventStep` -- set while a script owns the player.
    pub const PREVENT_STEP: u32 = 0x06;
}

/// `B_OUTCOME_WON`, `decompiled/include/constants/battle.h:76`.
pub const B_OUTCOME_WON: u8 = 1;
/// `B_OUTCOME_LOST`, `decompiled/include/constants/battle.h:77`.
pub const B_OUTCOME_LOST: u8 = 2;

/// The addresses the probes need, resolved once so a missing symbol is one
/// clear error at startup rather than a zero read in the middle of a route.
#[derive(Debug, Clone)]
pub struct Observer {
    syms: SymbolTable,
    g_main: u32,
    g_save_block1_ptr: u32,
    g_battle_outcome: u32,
    g_battle_mons: u32,
    g_battle_type_flags: u32,
    g_rng_value: u32,
    g_player_avatar: u32,
    g_player_party_count: u32,
}

impl Observer {
    pub fn new(syms: SymbolTable) -> Result<Self, String> {
        let addr = |name: &str| {
            syms.get(name)
                .map(|s| s.addr)
                .ok_or_else(|| format!("{name} is not in the symbol table"))
        };
        Ok(Self {
            g_main: addr("gMain")?,
            g_save_block1_ptr: addr("gSaveBlock1Ptr")?,
            g_battle_outcome: addr("gBattleOutcome")?,
            g_battle_mons: addr("gBattleMons")?,
            g_battle_type_flags: addr("gBattleTypeFlags")?,
            g_rng_value: addr("gRngValue")?,
            g_player_avatar: addr("gPlayerAvatar")?,
            g_player_party_count: addr("gPlayerPartyCount")?,
            syms,
        })
    }

    pub fn symbols(&self) -> &SymbolTable {
        &self.syms
    }

    /// `gMain.callback2`, with the Thumb bit still on -- compare with
    /// [`Observer::callback2_is`] rather than against a raw address.
    pub fn callback2(&self, emu: &mut Emu) -> u32 {
        emu.read32(self.g_main + main_off::CALLBACK2)
    }

    /// Which screen is running, as a symbol name.
    pub fn callback2_name(&self, emu: &mut Emu) -> String {
        let addr = self.callback2(emu);
        self.syms.describe(addr)
    }

    /// True when `gMain.callback2` points inside the named function. A
    /// callback is compared by containment, not equality, because the sym file
    /// records the entry point and the pointer is the entry point | 1.
    pub fn callback2_is(&self, emu: &mut Emu, name: &str) -> bool {
        let addr = self.callback2(emu);
        matches!(self.syms.covering(addr), Some((sym, _)) if sym == name)
    }

    /// `gMain.inBattle`.
    pub fn in_battle(&self, emu: &mut Emu) -> bool {
        emu.read8(self.g_main + main_off::FLAGS) & main_off::IN_BATTLE_BIT != 0
    }

    /// `gMain.newKeys` -- what the game itself saw pressed this frame.
    pub fn new_keys(&self, emu: &mut Emu) -> u16 {
        emu.read16(self.g_main + main_off::NEW_KEYS)
    }

    /// `gSaveBlock1Ptr`. Null before the save block is allocated, which is the
    /// case on the title screen, so every save-block probe returns an Option.
    pub fn save_block1(&self, emu: &mut Emu) -> Option<u32> {
        let ptr = emu.read32(self.g_save_block1_ptr);
        // EWRAM only; anything else means "not allocated yet" rather than a
        // pointer worth dereferencing.
        (0x0200_0000..0x0204_0000).contains(&ptr).then_some(ptr)
    }

    /// `gSaveBlock1Ptr->location`: the map the player is standing on.
    pub fn map(&self, emu: &mut Emu) -> Option<(u8, u8)> {
        let sb1 = self.save_block1(emu)?;
        Some((
            emu.read8(sb1 + sb1_off::MAP_GROUP),
            emu.read8(sb1 + sb1_off::MAP_NUM),
        ))
    }

    /// `gSaveBlock1Ptr->pos`, in map tiles.
    pub fn pos(&self, emu: &mut Emu) -> Option<(i16, i16)> {
        let sb1 = self.save_block1(emu)?;
        Some((
            emu.read16(sb1 + sb1_off::POS_X) as i16,
            emu.read16(sb1 + sb1_off::POS_Y) as i16,
        ))
    }

    /// `gPlayerPartyCount` (`decompiled/include/pokemon.h:285`) -- the live
    /// party size. This is what `givemon` moves; the save block's copy only
    /// catches up when the game saves, which cost a debugging session once.
    pub fn party_count(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.g_player_party_count)
    }

    /// `gSaveBlock1Ptr->playerPartyCount`, the saved copy. Only interesting
    /// when checking that a save happened.
    pub fn saved_party_count(&self, emu: &mut Emu) -> Option<u8> {
        let sb1 = self.save_block1(emu)?;
        Some(emu.read8(sb1 + sb1_off::SAVED_PARTY_COUNT))
    }

    /// A script variable by its `VAR_*` id, e.g. [`VAR_OAKS_LAB_SCENE`].
    pub fn var(&self, emu: &mut Emu, id: u16) -> Option<u16> {
        let sb1 = self.save_block1(emu)?;
        let index = id.checked_sub(VARS_START)? as u32;
        Some(emu.read16(sb1 + sb1_off::VARS + index * 2))
    }

    /// `gBattleOutcome`. Stale between battles -- it is only meaningful once a
    /// battle has ended, so a route clears it or compares it to a battle it
    /// knows started.
    pub fn battle_outcome(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.g_battle_outcome)
    }

    pub fn battle_type_flags(&self, emu: &mut Emu) -> u32 {
        emu.read32(self.g_battle_type_flags)
    }

    /// `gBattleMons[i]` -- species, level, HP, max HP.
    pub fn battle_mon(&self, emu: &mut Emu, index: u32) -> BattleMon {
        let base = self.g_battle_mons + index * battle_mon_off::SIZE;
        BattleMon {
            species: emu.read16(base + battle_mon_off::SPECIES),
            level: emu.read8(base + battle_mon_off::LEVEL),
            hp: emu.read16(base + battle_mon_off::HP),
            max_hp: emu.read16(base + battle_mon_off::MAX_HP),
            moves: std::array::from_fn(|i| emu.read16(base + battle_mon_off::MOVES + 2 * i as u32)),
        }
    }

    /// `gRngValue` -- the whole RNG state (`decompiled/include/random.h:6`).
    pub fn rng(&self, emu: &mut Emu) -> u32 {
        emu.read32(self.g_rng_value)
    }

    /// `gPlayerAvatar.flags`. Zero until the overworld sets the player up,
    /// which makes it a cheap "am I actually on the field" test.
    pub fn player_avatar_flags(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.g_player_avatar + avatar_off::FLAGS)
    }

    /// True when the player is settled on a tile and no script is holding
    /// them: `runningState == 0 && tileTransitionState == 0 && !preventStep`.
    /// This is the frame on which a direction press turns into a step, so it
    /// is what movement segments wait for.
    pub fn player_can_step(&self, emu: &mut Emu) -> bool {
        self.player_avatar_flags(emu) != 0
            && emu.read8(self.g_player_avatar + avatar_off::RUNNING_STATE) == 0
            && emu.read8(self.g_player_avatar + avatar_off::TILE_TRANSITION_STATE) == 0
            && emu.read8(self.g_player_avatar + avatar_off::PREVENT_STEP) == 0
    }

    /// `gPlayerAvatar.preventStep` -- set while a script owns the player.
    pub fn prevent_step(&self, emu: &mut Emu) -> bool {
        emu.read8(self.g_player_avatar + avatar_off::PREVENT_STEP) != 0
    }

    /// Everything at once, for logging a route's progress.
    pub fn snapshot(&self, emu: &mut Emu) -> Snapshot {
        Snapshot {
            frame: emu.frame(),
            callback2: self.callback2_name(emu),
            in_battle: self.in_battle(emu),
            map: self.map(emu),
            pos: self.pos(emu),
            party_count: self.party_count(emu),
            battle_outcome: self.battle_outcome(emu),
            rng: self.rng(emu),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleMon {
    pub species: u16,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub moves: [u16; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub frame: u32,
    pub callback2: String,
    pub in_battle: bool,
    pub map: Option<(u8, u8)>,
    pub pos: Option<(i16, i16)>,
    pub party_count: u8,
    pub battle_outcome: u8,
    pub rng: u32,
}

impl std::fmt::Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{:<7} {:<28}", self.frame, self.callback2)?;
        match (self.map, self.pos) {
            (Some((g, n)), Some((x, y))) => write!(f, " map {g}.{n} at ({x},{y})")?,
            _ => write!(f, " (no save block)")?,
        }
        write!(f, " party {}", self.party_count)?;
        if self.in_battle {
            write!(f, " IN-BATTLE")?;
        }
        write!(f, " rng {:#010x}", self.rng)
    }
}
