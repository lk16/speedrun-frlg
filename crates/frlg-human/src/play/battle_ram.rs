//! The battle state in RAM, read raw: the battlers, both parties, the bag, the menus a battle
//! drives. Every offset cites its struct in `decompiled/`; addresses come from the ROM's symbol
//! table (`$FRLG_ARTIFACTS/rom/pokeleafgreen.sym`), so a missing symbol fails at startup.
//!
//! Nothing here interprets: conversion to the simulator's types is `play/battle.rs`.
use frlg_emu::{Emu, SymbolTable};
use frlg_route::Observer;

/// `struct BattlePokemon`, `decompiled/include/pokemon.h:170-206`.
mod bmon {
    pub const SPECIES: u32 = 0x00;
    /// atk, def, spe, spa, spd (u16 each).
    pub const STATS: u32 = 0x02;
    pub const MOVES: u32 = 0x0C;
    /// `s8 statStages[NUM_BATTLE_STATS]`: HP, Atk, Def, Spe, SpA, SpD, Acc, Eva; 6 = neutral.
    pub const STAGES: u32 = 0x18;
    pub const TYPE1: u32 = 0x21;
    pub const TYPE2: u32 = 0x22;
    pub const PP: u32 = 0x24;
    pub const HP: u32 = 0x28;
    pub const LEVEL: u32 = 0x2A;
    pub const MAX_HP: u32 = 0x2C;
    pub const ITEM: u32 = 0x2E;
    pub const STATUS1: u32 = 0x4C;
    pub const STATUS2: u32 = 0x50;
    pub const SIZE: u32 = 0x58;
}

/// `struct Pokemon` (`decompiled/include/pokemon.h:128-141`): a `struct BoxPokemon` (`:105-126`,
/// 80 bytes) then the unencrypted battle stats.
mod pmon {
    pub const PERSONALITY: u32 = 0x00;
    pub const OT_ID: u32 = 0x04;
    /// `union secure`: four 12-byte substructs, XOR-encrypted with `otId ^ personality`
    /// (`DecryptBoxMon`, `decompiled/src/pokemon.c:2807-2815`).
    pub const SECURE: u32 = 0x20;
    pub const STATUS: u32 = 0x50;
    pub const LEVEL: u32 = 0x54;
    pub const HP: u32 = 0x56;
    pub const MAX_HP: u32 = 0x58;
    /// attack, defense, speed, spAttack, spDefense (u16 each).
    pub const STATS: u32 = 0x5A;
    pub const SIZE: u32 = 0x64;
}

/// `GetSubstruct` (`decompiled/src/pokemon.c:2863-2896`): for `personality % 24`, the position of
/// substruct type 0 (species, item), 1 (moves, PP), 2 (EVs), 3 (misc).
const SUBSTRUCT_POS: [[u8; 4]; 24] = [
    [0, 1, 2, 3],
    [0, 1, 3, 2],
    [0, 2, 1, 3],
    [0, 3, 1, 2],
    [0, 2, 3, 1],
    [0, 3, 2, 1],
    [1, 0, 2, 3],
    [1, 0, 3, 2],
    [2, 0, 1, 3],
    [3, 0, 1, 2],
    [2, 0, 3, 1],
    [3, 0, 2, 1],
    [1, 2, 0, 3],
    [1, 3, 0, 2],
    [2, 1, 0, 3],
    [3, 1, 0, 2],
    [2, 3, 0, 1],
    [3, 2, 0, 1],
    [1, 2, 3, 0],
    [1, 3, 2, 0],
    [2, 1, 3, 0],
    [3, 1, 2, 0],
    [2, 3, 1, 0],
    [3, 2, 1, 0],
];

/// `struct SaveBlock1` (`decompiled/include/global.h:759-782`): the bag pockets, `struct ItemSlot
/// {u16 itemId; u16 quantity}` (`global.h:400-404`), counts from `include/constants/global.h:36-40`.
mod sb1 {
    pub const BAG_ITEMS: u32 = 0x0310;
    pub const BAG_ITEMS_COUNT: u32 = 42;
    pub const BAG_KEY_ITEMS: u32 = 0x03B8;
    pub const BAG_KEY_ITEMS_COUNT: u32 = 30;
    pub const BAG_POKE_BALLS: u32 = 0x0430;
    pub const BAG_POKE_BALLS_COUNT: u32 = 13;
    /// `u8 flags[]` (`global.h:790`).
    pub const FLAGS: u32 = 0x0EE0;
}

/// `struct SaveBlock2` (`decompiled/include/global.h:327-358`).
mod sb2 {
    /// `u16 optionsTextSpeed:3, optionsWindowFrameType:5, optionsSound:1, optionsBattleStyle:1, ...`.
    pub const OPTIONS: u32 = 0x014;
    pub const BATTLE_STYLE_BIT: u16 = 1 << 9;
    /// `u32 encryptionKey`: bag quantities are stored XORed with it (`GetBagItemQuantity`,
    /// `decompiled/src/item.c:21-24`).
    pub const ENCRYPTION_KEY: u32 = 0xF20;
}

/// `FLAG_BADGE01_GET` (`decompiled/include/constants/flags.h`), badges 1-8 consecutive.
const FLAG_BADGE01_GET: u32 = 0x820;

/// Bag pockets as `gBagMenuState.pocket` numbers them (`POCKET_* - 1`,
/// `decompiled/src/item_menu.c:712,833`).
pub const POCKET_ITEMS: u16 = 0;
pub const POCKET_KEY_ITEMS: u16 = 1;
pub const POCKET_POKE_BALLS: u16 = 2;

/// One battler, `struct BattlePokemon` as is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawBattleMon {
    pub species: u16,
    pub stats: [u16; 5],
    pub moves: [u16; 4],
    pub stages: [i8; 8],
    pub types: (u8, u8),
    pub pp: [u8; 4],
    pub hp: u16,
    pub level: u8,
    pub max_hp: u16,
    pub item: u16,
    pub status1: u32,
    pub status2: u32,
}

/// One party slot, decrypted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPartyMon {
    pub personality: u32,
    pub species: u16,
    pub item: u16,
    pub moves: [u16; 4],
    pub pp: [u8; 4],
    pub status: u32,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub stats: [u16; 5],
    /// HP, Atk, Def, Spe, SpA, SpD.
    pub ivs: [u32; 6],
    pub evs: [u32; 6],
    pub is_egg: bool,
}

#[derive(Debug, Clone)]
pub struct BattleRam {
    g_battle_mons: u32,
    g_battler_party_indexes: u32,
    g_player_party: u32,
    g_player_party_count: u32,
    g_enemy_party: u32,
    g_enemy_party_count: u32,
    g_trainer_battle_opponent_a: u32,
    g_side_timers: u32,
    g_side_statuses: u32,
    g_disable_structs: u32,
    g_battle_communication: u32,
    g_bag_menu_state: u32,
    g_party_menu: u32,
    s_move_selection_cursor_pos: u32,
    g_battlescript_curr_instr: u32,
    g_last_moves: u32,
    g_move_to_learn: u32,
}

impl BattleRam {
    pub fn new(syms: &SymbolTable) -> Result<Self, String> {
        let addr = |name: &str| {
            syms.get(name)
                .map(|s| s.addr)
                .ok_or_else(|| format!("{name} is not in the symbol table"))
        };
        Ok(BattleRam {
            g_battle_mons: addr("gBattleMons")?,
            g_battler_party_indexes: addr("gBattlerPartyIndexes")?,
            g_player_party: addr("gPlayerParty")?,
            g_player_party_count: addr("gPlayerPartyCount")?,
            g_enemy_party: addr("gEnemyParty")?,
            g_enemy_party_count: addr("gEnemyPartyCount")?,
            g_trainer_battle_opponent_a: addr("gTrainerBattleOpponent_A")?,
            g_side_timers: addr("gSideTimers")?,
            g_side_statuses: addr("gSideStatuses")?,
            g_disable_structs: addr("gDisableStructs")?,
            g_battle_communication: addr("gBattleCommunication")?,
            g_bag_menu_state: addr("gBagMenuState")?,
            g_party_menu: addr("gPartyMenu")?,
            s_move_selection_cursor_pos: addr("sMoveSelectionCursorPos")?,
            g_battlescript_curr_instr: addr("gBattlescriptCurrInstr")?,
            g_last_moves: addr("gLastMoves")?,
            g_move_to_learn: addr("gMoveToLearn")?,
        })
    }

    /// `gBattleMons[battler]` (0 = player, 1 = foe in a single battle).
    pub fn battle_mon(&self, emu: &mut Emu, battler: u32) -> RawBattleMon {
        let b = self.g_battle_mons + battler * bmon::SIZE;
        RawBattleMon {
            species: emu.read16(b + bmon::SPECIES),
            stats: std::array::from_fn(|i| emu.read16(b + bmon::STATS + 2 * i as u32)),
            moves: std::array::from_fn(|i| emu.read16(b + bmon::MOVES + 2 * i as u32)),
            stages: std::array::from_fn(|i| emu.read8(b + bmon::STAGES + i as u32) as i8),
            types: (emu.read8(b + bmon::TYPE1), emu.read8(b + bmon::TYPE2)),
            pp: std::array::from_fn(|i| emu.read8(b + bmon::PP + i as u32)),
            hp: emu.read16(b + bmon::HP),
            level: emu.read8(b + bmon::LEVEL),
            max_hp: emu.read16(b + bmon::MAX_HP),
            item: emu.read16(b + bmon::ITEM),
            status1: emu.read32(b + bmon::STATUS1),
            status2: emu.read32(b + bmon::STATUS2),
        }
    }

    /// `gBattlerPartyIndexes[battler]`: the party slot on the field.
    pub fn party_index(&self, emu: &mut Emu, battler: u32) -> usize {
        emu.read16(self.g_battler_party_indexes + 2 * battler) as usize
    }

    fn party_mon(&self, emu: &mut Emu, base: u32) -> RawPartyMon {
        let pid = emu.read32(base + pmon::PERSONALITY);
        let key = pid ^ emu.read32(base + pmon::OT_ID);
        let pos = SUBSTRUCT_POS[(pid % 24) as usize];
        let word = |emu: &mut Emu, sub: usize, off: u32| -> u32 {
            emu.read32(base + pmon::SECURE + pos[sub] as u32 * 12 + off) ^ key
        };
        // Substruct 0: `u16 species; u16 heldItem; ...` (`pokemon.h:8-16`).
        let w0 = word(emu, 0, 0);
        // Substruct 1: `u16 moves[4]; u8 pp[4]` (`pokemon.h:18-22`).
        let m01 = word(emu, 1, 0);
        let m23 = word(emu, 1, 4);
        let pp = word(emu, 1, 8);
        // Substruct 2: `u8 hpEV .. spDefenseEV` (`pokemon.h:24-38`).
        let ev = word(emu, 2, 0).to_le_bytes();
        let ev2 = word(emu, 2, 4).to_le_bytes();
        // Substruct 3, the u32 at 0x04: six 5-bit IVs, then `isEgg:1` at bit 30 (`pokemon.h:50-61`).
        let ivs = word(emu, 3, 4);
        RawPartyMon {
            personality: pid,
            species: w0 as u16,
            item: (w0 >> 16) as u16,
            moves: [
                m01 as u16,
                (m01 >> 16) as u16,
                m23 as u16,
                (m23 >> 16) as u16,
            ],
            pp: pp.to_le_bytes(),
            status: emu.read32(base + pmon::STATUS),
            level: emu.read8(base + pmon::LEVEL),
            hp: emu.read16(base + pmon::HP),
            max_hp: emu.read16(base + pmon::MAX_HP),
            stats: std::array::from_fn(|i| emu.read16(base + pmon::STATS + 2 * i as u32)),
            ivs: std::array::from_fn(|i| (ivs >> (5 * i)) & 0x1F),
            evs: [ev[0], ev[1], ev[2], ev[3], ev2[0], ev2[1]].map(u32::from),
            is_egg: ivs & (1 << 30) != 0,
        }
    }

    /// `gPlayerParty[0..gPlayerPartyCount]`.
    pub fn player_party(&self, emu: &mut Emu) -> Vec<RawPartyMon> {
        let n = (emu.read8(self.g_player_party_count) as u32).min(6);
        (0..n)
            .map(|i| self.party_mon(emu, self.g_player_party + i * pmon::SIZE))
            .collect()
    }

    /// `gEnemyParty[0..gEnemyPartyCount]` (`gEnemyPartyCount` is set by the battle setup; a wild
    /// battle has one).
    pub fn enemy_party(&self, emu: &mut Emu) -> Vec<RawPartyMon> {
        let n = (emu.read8(self.g_enemy_party_count) as u32).clamp(1, 6);
        (0..n)
            .map(|i| self.party_mon(emu, self.g_enemy_party + i * pmon::SIZE))
            .filter(|m| m.species != 0)
            .collect()
    }

    /// `gTrainerBattleOpponent_A`: the trainer id (`include/constants/opponents.h`).
    pub fn trainer_id(&self, emu: &mut Emu) -> u16 {
        emu.read16(self.g_trainer_battle_opponent_a)
    }

    /// (Reflect, Light Screen) turns left on `side` (0 player, 1 foe): `gSideTimers[side]`
    /// (`struct SideTimer`, `decompiled/include/battle.h:237-251`, 12 bytes), counted only while the
    /// `gSideStatuses` bit is up (`SIDE_STATUS_REFLECT` 1, `SIDE_STATUS_LIGHTSCREEN` 2,
    /// `include/constants/battle.h:185-186`).
    pub fn screens(&self, emu: &mut Emu, side: u32) -> (u8, u8) {
        let st = emu.read16(self.g_side_statuses + 2 * side);
        let t = self.g_side_timers + 12 * side;
        let r = if st & 1 != 0 { emu.read8(t) } else { 0 };
        let l = if st & 2 != 0 { emu.read8(t + 2) } else { 0 };
        (r, l)
    }

    /// `gDisableStructs[battler]` (`struct DisableStruct`, `decompiled/include/battle.h:139-166`,
    /// 0x1C bytes): the disabled move and its timer.
    pub fn disabled(&self, emu: &mut Emu, battler: u32) -> (u16, u8) {
        let b = self.g_disable_structs + 0x1C * battler;
        (emu.read16(b + 0x04), emu.read8(b + 0x0B) & 0x0F)
    }

    /// `gLastMoves[battler]`.
    pub fn last_move(&self, emu: &mut Emu, battler: u32) -> u16 {
        emu.read16(self.g_last_moves + 2 * battler)
    }

    /// `gBattleCommunication[CURSOR_POSITION]` (index 1): the battle YES/NO box cursor
    /// (`Cmd_yesnoboxlearnmove`, `decompiled/src/battle_script_commands.c:5142-5188`).
    pub fn yesno_cursor(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.g_battle_communication + 1)
    }

    /// The battle-script opcode about to run (`*gBattlescriptCurrInstr`), e.g. 0x5A
    /// `yesnoboxlearnmove` (`battle_script_commands.c:404`).
    pub fn script_opcode(&self, emu: &mut Emu) -> u8 {
        let p = emu.read32(self.g_battlescript_curr_instr);
        if (0x0800_0000..0x0A00_0000).contains(&p) {
            emu.read8(p)
        } else {
            0
        }
    }

    /// `gMoveToLearn`: the move a level-up offers (`battle_script_commands.c` `Cmd_yesnoboxlearnmove`,
    /// `evolution_scene.c`).
    pub fn move_to_learn(&self, emu: &mut Emu) -> u16 {
        emu.read16(self.g_move_to_learn)
    }

    /// `gBagMenuState` (`struct BagStruct`, `decompiled/include/item_menu.h:12-20`): the pocket and
    /// the list row (`itemsAbove + cursorPos` of that pocket).
    pub fn bag_cursor(&self, emu: &mut Emu) -> (u16, u16) {
        let pocket = emu.read16(self.g_bag_menu_state + 6);
        let p = pocket.min(2) as u32;
        let above = emu.read16(self.g_bag_menu_state + 8 + 2 * p);
        let cur = emu.read16(self.g_bag_menu_state + 0x0E + 2 * p);
        (pocket, above + cur)
    }

    /// `gPartyMenu.slotId` (`struct PartyMenu`, `decompiled/include/party_menu.h:8-20`: two
    /// pointers, a bitfield byte, then `s8 slotId`).
    pub fn party_menu_slot(&self, emu: &mut Emu) -> i8 {
        emu.read8(self.g_party_menu + 9) as i8
    }

    /// `sMoveSelectionCursorPos` of the summary screen's move list (0-3 the known moves, 4 the new
    /// one; `decompiled/src/pokemon_summary_screen.c:326,3778-3870`).
    pub fn summary_move_cursor(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.s_move_selection_cursor_pos)
    }
}

/// The bag pocket `pocket` (0 Items, 1 Key Items, 2 Poke Balls) as (item, quantity), in list order.
pub fn bag_pocket(obs: &Observer, emu: &mut Emu, pocket: u16) -> Vec<(u16, u16)> {
    let (Some(s1), Some(s2)) = (obs.save_block1(emu), obs.save_block2(emu)) else {
        return vec![];
    };
    let key = emu.read32(s2 + sb2::ENCRYPTION_KEY) as u16;
    let (off, n) = match pocket {
        POCKET_ITEMS => (sb1::BAG_ITEMS, sb1::BAG_ITEMS_COUNT),
        POCKET_KEY_ITEMS => (sb1::BAG_KEY_ITEMS, sb1::BAG_KEY_ITEMS_COUNT),
        _ => (sb1::BAG_POKE_BALLS, sb1::BAG_POKE_BALLS_COUNT),
    };
    (0..n)
        .map(|i| {
            let a = s1 + off + 4 * i;
            (emu.read16(a), emu.read16(a + 2) ^ key)
        })
        .take_while(|&(id, _)| id != 0)
        .collect()
}

/// Badges as a bit mask (bit i = badge i+1, the simulator's `make_mon` order).
pub fn badges(obs: &Observer, emu: &mut Emu) -> u8 {
    let Some(s1) = obs.save_block1(emu) else {
        return 0;
    };
    let mut b = 0u8;
    for i in 0..8u32 {
        let id = FLAG_BADGE01_GET + i;
        if emu.read8(s1 + sb1::FLAGS + id / 8) & (1 << (id & 7)) != 0 {
            b |= 1 << i;
        }
    }
    b
}

/// Battle style SET (no "Will you change POKéMON?" between a trainer's mons):
/// `optionsBattleStyle`, `OPTIONS_BATTLE_STYLE_SET` = 1 (`include/constants/global.h:106-107`).
pub fn battle_style_set(obs: &Observer, emu: &mut Emu) -> bool {
    obs.save_block2(emu)
        .is_some_and(|s2| emu.read16(s2 + sb2::OPTIONS) & sb2::BATTLE_STYLE_BIT != 0)
}
