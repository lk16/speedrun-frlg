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

/// `struct SaveBlock2`, `decompiled/include/global.h:327`, offsets from the
/// members' own comments.
mod sb2_off {
    /// `/*0x000*/ u8 playerName[PLAYER_NAME_LENGTH + 1]` -- 7 chars + EOS.
    pub const PLAYER_NAME: u32 = 0x000;
    /// `/*0x014*/ u16` bitfield: `optionsTextSpeed:3` then
    /// `optionsWindowFrameType:5`, `optionsSound:1`, `optionsBattleStyle:1`,
    /// `optionsBattleSceneOff:1`. GCC allocates little-endian bitfields from
    /// the least significant bit, so textSpeed is bits 0-2 and battleSceneOff
    /// is bit 10 -- `tests/observe.rs` checks both against the running game.
    pub const OPTIONS: u32 = 0x014;
    pub const TEXT_SPEED_MASK: u16 = 0x0007;
    pub const BATTLE_SCENE_OFF_BIT: u16 = 1 << 10;
}

/// `OPTIONS_TEXT_SPEED_FAST`, `decompiled/include/constants/global.h:101`.
pub const TEXT_SPEED_FAST: u16 = 2;

/// `EOS`, `decompiled/include/characters.h:182` -- names are EOS-terminated.
pub const EOS: u8 = 0xFF;
/// `PLAYER_NAME_LENGTH`, `decompiled/include/constants/global.h:64`.
pub const PLAYER_NAME_LENGTH: u32 = 7;

/// `struct Task`, `decompiled/include/task.h:15`: `TaskFunc func` at 0x0,
/// `bool8 isActive` at 0x4, then prev/next/priority and `s16 data[16]` --
/// 40 bytes a task, `NUM_TASKS 16` of them in `gTasks` (`task.h:10`).
mod task_off {
    pub const FUNC: u32 = 0x0;
    pub const IS_ACTIVE: u32 = 0x4;
    /// `s16 data[NUM_TASK_DATA]` -- `data[0]` is what the `#define tState
    /// data[0]` convention across the decomp reads.
    pub const DATA: u32 = 0x8;
    pub const SIZE: u32 = 40;
    pub const COUNT: u32 = 16;
}

/// The naming screen's input gate: `SetInputState` writes the input task's
/// `tState` (`data[0]`, `decompiled/src/naming_screen.c:1554`), and only
/// `INPUT_STATE_ENABLED` (= 1, the enum at `naming_screen.c:135`) routes
/// presses to the keyboard -- `Input_Disabled` drops them.
pub const NAMING_INPUT_ENABLED: i16 = 1;

/// `struct OptionMenu`, `decompiled/src/option_menu.c:38`: `u16 option[7]`,
/// `/*0x0E*/ u16 cursorPos`, `/*0x10*/ u8 loadState`. `Task_OptionMenu`
/// (`option_menu.c:359`) only feeds input to `OptionMenu_ProcessInput` in
/// `loadState` 2, after the fade-in -- presses before that are dropped.
mod option_menu_off {
    pub const OPTIONS: u32 = 0x00;
    pub const CURSOR_POS: u32 = 0x0E;
    pub const LOAD_STATE: u32 = 0x10;
    pub const ACCEPTING_INPUT: u8 = 2;
}

/// `MENUITEM_TEXTSPEED` / `MENUITEM_BATTLESCENE`
/// (`decompiled/src/option_menu.c:20`) -- rows of the option menu, and
/// indices into its `option[]`.
pub const MENUITEM_TEXTSPEED: u32 = 0;
pub const MENUITEM_BATTLESCENE: u32 = 1;

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
    /// `/*0x3A4C*/ u8 rivalName[PLAYER_NAME_LENGTH + 1]`
    /// (`decompiled/include/global.h:813`).
    pub const RIVAL_NAME: u32 = 0x3A4C;
    /// `/*0x0EE0*/ u8 flags[NUM_FLAG_BYTES]` (`decompiled/include/global.h:790`).
    pub const FLAGS: u32 = 0x0EE0;
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

/// `VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN` (`decompiled/include/constants/vars.h`):
/// 0 lying across the road, 1 standing (tutorial pending), 2+ done, road open
/// (`data/maps/ViridianCity/scripts.inc:5-27`).
pub const VAR_VIRIDIAN_OLD_MAN: u16 = 0x4051;

/// `VAR_MAP_SCENE_VIRIDIAN_CITY_MART` (`decompiled/include/constants/vars.h`):
/// 1 once the clerk has handed over Oak's Parcel
/// (`data/maps/ViridianCity_Mart/scripts.inc:19-33`).
pub const VAR_VIRIDIAN_MART: u16 = 0x4057;

/// `FLAG_DEFEATED_BROCK 0x4B0` (`decompiled/include/constants/flags.h:1236`),
/// set by `PewterCity_Gym_EventScript_DefeatedBrock`
/// (`data/maps/PewterCity_Gym/scripts.inc:14`).
pub const FLAG_DEFEATED_BROCK: u16 = 0x4B0;

/// `FLAG_BADGE01_GET` = `SYS_FLAGS + 0x20` = 0x820
/// (`decompiled/include/constants/flags.h:1324,1364`).
pub const FLAG_BADGE01_GET: u16 = 0x820;

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
/// `B_OUTCOME_RAN`, `decompiled/include/constants/battle.h:79`.
pub const B_OUTCOME_RAN: u8 = 4;

/// `BATTLE_TYPE_TRAINER` (`decompiled/include/constants/battle.h:45`).
pub const BATTLE_TYPE_TRAINER: u32 = 0x8;

/// The addresses the probes need, resolved once so a missing symbol is one
/// clear error at startup rather than a zero read in the middle of a route.
#[derive(Debug, Clone)]
pub struct Observer {
    syms: SymbolTable,
    g_main: u32,
    g_save_block1_ptr: u32,
    g_battle_outcome: u32,
    g_battle_mons: u32,
    g_battle_main_func: u32,
    g_battle_type_flags: u32,
    g_rng_value: u32,
    g_player_avatar: u32,
    g_player_party_count: u32,
    g_save_block2_ptr: u32,
    g_tasks: u32,
    s_option_menu_ptr: u32,
    s_start_menu_callback: u32,
    s_start_menu_cursor_pos: u32,
    g_battler_controller_funcs: u32,
    g_move_selection_cursor: u32,
    g_action_selection_cursor: u32,
    s_wild_encounter_data: u32,
    s_lock_field_controls: u32,
    g_player_party: u32,
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
            g_battle_main_func: addr("gBattleMainFunc")?,
            g_battle_type_flags: addr("gBattleTypeFlags")?,
            g_rng_value: addr("gRngValue")?,
            g_player_avatar: addr("gPlayerAvatar")?,
            g_player_party_count: addr("gPlayerPartyCount")?,
            g_save_block2_ptr: addr("gSaveBlock2Ptr")?,
            g_tasks: addr("gTasks")?,
            s_option_menu_ptr: addr("sOptionMenuPtr")?,
            s_start_menu_callback: addr("sStartMenuCallback")?,
            s_start_menu_cursor_pos: addr("sStartMenuCursorPos")?,
            g_battler_controller_funcs: addr("gBattlerControllerFuncs")?,
            g_move_selection_cursor: addr("gMoveSelectionCursor")?,
            g_action_selection_cursor: addr("gActionSelectionCursor")?,
            s_wild_encounter_data: addr("sWildEncounterData")?,
            s_lock_field_controls: addr("sLockFieldControls")?,
            g_player_party: addr("gPlayerParty")?,
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

    /// `gSaveBlock2Ptr`, with the same not-yet-allocated guard as
    /// [`Observer::save_block1`].
    pub fn save_block2(&self, emu: &mut Emu) -> Option<u32> {
        let ptr = emu.read32(self.g_save_block2_ptr);
        (0x0200_0000..0x0204_0000).contains(&ptr).then_some(ptr)
    }

    /// Characters before the terminating `EOS` at `addr`, capped at
    /// `PLAYER_NAME_LENGTH` -- the length of a player or rival name.
    fn name_len_at(&self, emu: &mut Emu, addr: u32) -> u32 {
        (0..PLAYER_NAME_LENGTH)
            .take_while(|i| emu.read8(addr + i) != EOS)
            .count() as u32
    }

    /// `gSaveBlock2Ptr->playerName`'s length. 0 until a name is set.
    pub fn player_name_len(&self, emu: &mut Emu) -> Option<u32> {
        let sb2 = self.save_block2(emu)?;
        Some(self.name_len_at(emu, sb2 + sb2_off::PLAYER_NAME))
    }

    /// `gSaveBlock1Ptr->rivalName`'s length. 0 until a name is set.
    pub fn rival_name_len(&self, emu: &mut Emu) -> Option<u32> {
        let sb1 = self.save_block1(emu)?;
        Some(self.name_len_at(emu, sb1 + sb1_off::RIVAL_NAME))
    }

    /// `gSaveBlock2Ptr->optionsTextSpeed` -- compare with [`TEXT_SPEED_FAST`].
    pub fn options_text_speed(&self, emu: &mut Emu) -> Option<u16> {
        let sb2 = self.save_block2(emu)?;
        Some(emu.read16(sb2 + sb2_off::OPTIONS) & sb2_off::TEXT_SPEED_MASK)
    }

    /// `gSaveBlock2Ptr->optionsBattleSceneOff` -- true means no battle
    /// animations (`decompiled/src/battle_main.c:2259`).
    pub fn options_battle_scene_off(&self, emu: &mut Emu) -> Option<bool> {
        let sb2 = self.save_block2(emu)?;
        Some(emu.read16(sb2 + sb2_off::OPTIONS) & sb2_off::BATTLE_SCENE_OFF_BIT != 0)
    }

    /// True when some entry of `gTasks` is active and its `func` sits inside
    /// the named function -- "is this screen's input handler running", for
    /// menus whose state lives in a task rather than in `gMain.callback2`.
    pub fn task_active(&self, emu: &mut Emu, name: &str) -> bool {
        (0..task_off::COUNT).any(|i| {
            let base = self.g_tasks + i * task_off::SIZE;
            emu.read8(base + task_off::IS_ACTIVE) != 0
                && matches!(
                    self.syms.covering(emu.read32(base + task_off::FUNC)),
                    Some((sym, _)) if sym == name
                )
        })
    }

    /// `data[0]` of the active task running the named function, if any -- the
    /// decomp's `tState` convention. [`Observer::task_active`] with a state.
    pub fn task_state(&self, emu: &mut Emu, name: &str) -> Option<i16> {
        (0..task_off::COUNT).find_map(|i| {
            let base = self.g_tasks + i * task_off::SIZE;
            let active = emu.read8(base + task_off::IS_ACTIVE) != 0
                && matches!(
                    self.syms.covering(emu.read32(base + task_off::FUNC)),
                    Some((sym, _)) if sym == name
                );
            active.then(|| emu.read16(base + task_off::DATA) as i16)
        })
    }

    /// True once the naming screen routes presses to its keyboard: the input
    /// task (`Task_HandleInput`, `decompiled/src/naming_screen.c:1582`) is in
    /// `INPUT_STATE_ENABLED`, which `MainState_WaitFadeIn` switches on once
    /// the fade-in is done (`naming_screen.c:655`).
    pub fn naming_screen_accepting_input(&self, emu: &mut Emu) -> bool {
        self.task_state(emu, "Task_HandleInput") == Some(NAMING_INPUT_ENABLED)
    }

    /// True once the start menu is drawn and taking input: `sStartMenuCallback`
    /// has reached `StartCB_HandleInput`. `Task_StartMenuHandleInput` alone is
    /// not enough -- the menu draws over several frames first
    /// (`task50_startmenu` -> `DoDrawStartMenu`, `decompiled/src/start_menu.c:303`),
    /// and presses during the draw are dropped.
    pub fn start_menu_taking_input(&self, emu: &mut Emu) -> bool {
        let cb = emu.read32(self.s_start_menu_callback);
        matches!(self.syms.covering(cb), Some((sym, _)) if sym == "StartCB_HandleInput")
    }

    /// True once the options menu is taking input: `sOptionMenuPtr` is
    /// allocated and its `loadState` has reached the input case of
    /// `Task_OptionMenu` (`decompiled/src/option_menu.c:359`).
    pub fn option_menu_accepting_input(&self, emu: &mut Emu) -> bool {
        self.option_menu(emu).is_some_and(|ptr| {
            emu.read8(ptr + option_menu_off::LOAD_STATE) == option_menu_off::ACCEPTING_INPUT
        })
    }

    fn option_menu(&self, emu: &mut Emu) -> Option<u32> {
        let ptr = emu.read32(self.s_option_menu_ptr);
        (0x0200_0000..0x0204_0000).contains(&ptr).then_some(ptr)
    }

    /// `sStartMenuCursorPos` -- which row the start menu cursor is on.
    pub fn start_menu_cursor(&self, emu: &mut Emu) -> u8 {
        emu.read8(self.s_start_menu_cursor_pos)
    }

    /// `sOptionMenuPtr->cursorPos` -- which option row the cursor is on.
    pub fn option_menu_cursor(&self, emu: &mut Emu) -> Option<u16> {
        let ptr = self.option_menu(emu)?;
        Some(emu.read16(ptr + option_menu_off::CURSOR_POS))
    }

    /// `sOptionMenuPtr->option[item]` -- the menu's *working* value for a row,
    /// not yet written back to the save block; `CloseAndSaveOptionMenu` does
    /// that on exit (`decompiled/src/option_menu.c:508`).
    pub fn option_menu_setting(&self, emu: &mut Emu, item: u32) -> Option<u16> {
        let ptr = self.option_menu(emu)?;
        Some(emu.read16(ptr + option_menu_off::OPTIONS + item * 2))
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

    /// True while the battle is waiting for action selection:
    /// `gBattleMainFunc` points inside `HandleTurnActionSelectionState`
    /// (`decompiled/src/battle_main.c:3097`), which `BattleTurnPassed` re-arms
    /// at the top of every turn (`battle_main.c:2998`). One visit to this
    /// state is one turn, which is what makes it a per-turn decision point
    /// for the battle search.
    pub fn battle_choosing_actions(&self, emu: &mut Emu) -> bool {
        let func = emu.read32(self.g_battle_main_func);
        matches!(self.syms.covering(func), Some((sym, _)) if sym == "HandleTurnActionSelectionState")
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

    /// `FlagGet(id)` for ordinary save-block flags:
    /// `gSaveBlock1Ptr->flags[id / 8] & (1 << (id & 7))`
    /// (`decompiled/src/event_data.c:257-309`, flags array at
    /// `include/global.h:790`).
    pub fn flag(&self, emu: &mut Emu, id: u16) -> Option<bool> {
        let sb1 = self.save_block1(emu)?;
        let byte = emu.read8(sb1 + sb1_off::FLAGS + (id as u32) / 8);
        Some(byte & (1 << (id & 7)) != 0)
    }

    /// True when `gBattlerControllerFuncs[battler]` points inside the named
    /// function -- "is this battler's controller waiting in state X", e.g.
    /// `HandleInputChooseMove` for the move menu
    /// (`decompiled/src/battle_controller_player.c`).
    pub fn battle_controller_is(&self, emu: &mut Emu, battler: u32, name: &str) -> bool {
        let func = emu.read32(self.g_battler_controller_funcs + battler * 4);
        matches!(self.syms.covering(func), Some((sym, _)) if sym == name)
    }

    /// `gMoveSelectionCursor[battler]` -- which move slot the move menu's
    /// cursor is on. Persists across turns within one battle.
    pub fn move_cursor(&self, emu: &mut Emu, battler: u32) -> u8 {
        emu.read8(self.g_move_selection_cursor + battler)
    }

    /// `gActionSelectionCursor[battler]` -- FIGHT/BAG/POKEMON/RUN, 0-3.
    pub fn action_cursor(&self, emu: &mut Emu, battler: u32) -> u8 {
        emu.read8(self.g_action_selection_cursor + battler)
    }

    /// The lead party mon's `(hp, maxHP)` -- the computed stats at the tail
    /// of `struct Pokemon` are not encrypted (`include/pokemon.h:126-138`:
    /// box 80 bytes, status u32, level, mail, then `u16 hp` at 0x56 and
    /// `u16 maxHP` at 0x58).
    pub fn party_lead_hp(&self, emu: &mut Emu) -> (u16, u16) {
        (
            emu.read16(self.g_player_party + 0x56),
            emu.read16(self.g_player_party + 0x58),
        )
    }

    /// `sLockFieldControls` (`decompiled/src/script.c:34,199-209`): true
    /// while a script owns field input (`lockall`..`releaseall`). The
    /// avatar can read as free mid-scene between forced moves, so
    /// [`Observer::player_can_step`] alone does *not* prove a scene is over
    /// -- measured on the parcel scene, whose reward text waits for a press
    /// while the player stands "free" at the counter.
    pub fn field_controls_locked(&self, emu: &mut Emu) -> bool {
        emu.read8(self.s_lock_field_controls) != 0
    }

    /// `sWildEncounterData`'s decision-relevant fields, folded into one value:
    /// rngState (u32), encounterRateBuff and prevMetatileBehavior (u16 each),
    /// stepsSinceLastEncounter (u8) (`decompiled/src/wild_encounter.c:24-34`).
    /// Two field states with the same key make identical encounter decisions
    /// on identical tiles, which is what lets a path search treat "same tile,
    /// different rate-test index" as different nodes.
    pub fn wild_key(&self, emu: &mut Emu) -> u64 {
        let base = self.s_wild_encounter_data;
        let rng_state = emu.read32(base) as u64;
        // Behavior is a 9-bit attribute (`src/fieldmap.c:63-83`); steps
        // saturates at the largest minSteps, 8 (`wild_encounter.c:749`), so
        // 4 bits; the buff gets the remaining 19 (it grows by the map rate
        // per failed test and resets on success/map load, so 2^19 is far
        // beyond anything a route sees).
        let prev_behavior = (emu.read16(base + 4) as u64) & 0x1FF;
        let buff = (emu.read16(base + 6) as u64) & 0x7FFFF;
        let steps = (emu.read8(base + 8) as u64).min(15);
        rng_state | (prev_behavior << 32) | (steps << 41) | (buff << 45)
    }

    /// `sWildEncounterData`'s raw fields (`decompiled/src/wild_encounter.c:24-34`),
    /// for the model-driven path planner: the same data [`Observer::wild_key`]
    /// folds, unfolded.
    pub fn wild_data(&self, emu: &mut Emu) -> WildData {
        let base = self.s_wild_encounter_data;
        WildData {
            rng_state: emu.read32(base),
            prev_behavior: emu.read16(base + 4),
            rate_buff: emu.read16(base + 6),
            steps_since: emu.read8(base + 8),
        }
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

/// `struct WildEncounterData`'s decision-relevant fields
/// (`decompiled/src/wild_encounter.c:24-34`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WildData {
    pub rng_state: u32,
    pub prev_behavior: u16,
    pub rate_buff: u16,
    pub steps_since: u8,
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
