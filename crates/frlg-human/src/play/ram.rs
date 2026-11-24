//! RAM readers the controller needs beyond `frlg_route::Observer`: the party (decrypted), the
//! bag, money, and the live object events. Every offset cites the decomp header it comes from.
use frlg_emu::Emu;
use frlg_route::Observer;

/// `sizeof(struct Pokemon)` = 100: `struct BoxPokemon` (80 bytes) then `u32 status`, `u8 level`,
/// `u8 mail`, `u16 hp`, `u16 maxHP`, five `u16` stats (`decompiled/include/pokemon.h:105-141`).
pub const MON_SIZE: u32 = 100;

/// One decrypted party mon.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mon {
    pub species: u16,
    pub held: u16,
    pub exp: u32,
    pub moves: [u16; 4],
    pub pp: [u8; 4],
    /// `ppBonuses` (growth substruct byte 8, two bits per move: `decompiled/include/pokemon.h`).
    pub pp_bonuses: u8,
    pub status: u32,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    /// atk, def, spe, spa, spd.
    pub stats: [u16; 5],
    /// The decrypted block sums to the stored checksum (`CalculateBoxMonChecksum`,
    /// `decompiled/src/pokemon.c`; checksum at 0x1C). False on a mon read mid-update.
    pub valid: bool,
}

impl Mon {
    pub fn knows(&self, mv: u16) -> Option<usize> {
        self.moves.iter().position(|&m| m == mv)
    }
}

/// The position of substruct type `t` (0 growth, 1 attacks, 2 EVs, 3 misc) in the encrypted
/// block for `personality % 24`: `SUBSTRUCT_CASE(n, v1, v2, v3, v4)` puts type 0 at `v1` and so on
/// (`decompiled/src/pokemon.c:2817-2895`).
const ORDER: [[u8; 4]; 24] = [
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

/// Decrypt one `struct Pokemon` at `addr`. The 48-byte `secure` block (at 0x20) is XORed word by
/// word with `personality ^ otId` (`DecryptBoxMon`, `decompiled/src/pokemon.c`); growth holds
/// `u16 species, u16 heldItem, u32 experience`, attacks `u16 moves[4], u8 pp[4]`
/// (`decompiled/include/pokemon.h`, `struct PokemonSubstruct0/1`).
pub fn read_mon(emu: &mut Emu, addr: u32) -> Mon {
    let raw = emu.read_bytes(addr, MON_SIZE);
    let u32_at = |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
    let u16_at = |o: usize| u16::from_le_bytes([raw[o], raw[o + 1]]);
    let personality = u32_at(0);
    let key = personality ^ u32_at(4);
    let mut sec = [0u8; 48];
    for w in 0..12 {
        let v = u32_at(0x20 + w * 4) ^ key;
        sec[w * 4..w * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    let sum = (0..24).fold(0u16, |a, i| {
        a.wrapping_add(u16::from_le_bytes([sec[2 * i], sec[2 * i + 1]]))
    });
    let valid = sum == u16_at(0x1C);
    let ord = ORDER[(personality % 24) as usize];
    let sub = |t: usize| ord[t] as usize * 12;
    let s16 = |o: usize| u16::from_le_bytes([sec[o], sec[o + 1]]);
    let g = sub(0);
    let a = sub(1);
    Mon {
        species: s16(g),
        held: s16(g + 2),
        exp: u32::from_le_bytes([sec[g + 4], sec[g + 5], sec[g + 6], sec[g + 7]]),
        moves: std::array::from_fn(|i| s16(a + 2 * i)),
        pp: std::array::from_fn(|i| sec[a + 8 + i]),
        pp_bonuses: sec[g + 8],
        status: u32_at(80),
        level: raw[84],
        hp: u16_at(86),
        max_hp: u16_at(88),
        stats: std::array::from_fn(|i| u16_at(90 + 2 * i)),
        valid,
    }
}

/// `struct SaveBlock1` offsets (`decompiled/include/global.h:774-782`).
mod sb1 {
    pub const MONEY: u32 = 0x0290;
    pub const PC_ITEMS: u32 = 0x0298;
    pub const ITEMS: u32 = 0x0310;
    pub const KEY_ITEMS: u32 = 0x03b8;
    pub const BALLS: u32 = 0x0430;
    pub const TMHM: u32 = 0x0464;
    pub const BERRIES: u32 = 0x054c;
}
/// `gSaveBlock2Ptr->encryptionKey` at 0xF20 (`decompiled/include/global.h:358`).
const SB2_ENCRYPTION_KEY: u32 = 0xF20;

/// Bag pockets in `POCKET_*` order (`decompiled/include/constants/item.h`: items, key items, balls,
/// TMs/HMs, berries) with their slot counts (the gaps between the SaveBlock1 offsets above / 4).
pub const POCKETS: [(u32, u32); 5] = [
    (sb1::ITEMS, 42),
    (sb1::KEY_ITEMS, 30),
    (sb1::BALLS, 13),
    (sb1::TMHM, 58),
    (sb1::BERRIES, 43),
];

pub struct Ram<'a> {
    pub obs: &'a Observer,
    g_player_party: u32,
    g_object_events: u32,
}

/// `struct ObjectEvent` (`decompiled/include/global.fieldmap.h:212-268`): 0x24 bytes, `active` is
/// bit 0 of byte 0, `isPlayer` bit 16, `localId` at 0x08, `currentCoords` (s16 x, y) at 0x10, in
/// map coordinates plus `MAP_OFFSET` 7 (`decompiled/include/fieldmap.h`).
const OBJ_SIZE: u32 = 0x24;
const OBJ_COUNT: u32 = 16;
pub const MAP_OFFSET: i16 = 7;

#[derive(Clone, Copy, Debug)]
pub struct LiveObject {
    pub local_id: u8,
    pub x: i16,
    pub y: i16,
    pub graphics: u8,
    /// `previousCoords`: the tile a walking object is leaving. Collision checks both it and
    /// `currentCoords` (`DoesObjectCollideWithObjectAt`, `decompiled/src/event_object_movement.c:4899-4915`),
    /// and `currentCoords` is already the destination while a step is under way.
    pub px: i16,
    pub py: i16,
    /// `facingDirection` (`DIR_SOUTH` 1, `DIR_NORTH` 2, `DIR_WEST` 3, `DIR_EAST` 4).
    pub facing: u8,
    /// `movementType` (`MOVEMENT_TYPE_*`, `decompiled/include/constants/event_object_movement.h`).
    pub movement: u8,
    /// `trainerType` and `trainerRange_berryTreeId` (the sight range of a trainer).
    pub trainer_type: u8,
    pub range: u8,
    /// `mapNum` / `mapGroup` the object belongs to (an object of the previous map can stay
    /// loaded for a while after a connection is crossed).
    pub map: (u8, u8),
    pub elevation: u8,
    /// `invisible` (bit 13): hidden objects still collide, but nobody sees them.
    pub invisible: bool,
}

impl<'a> Ram<'a> {
    pub fn new(obs: &'a Observer) -> Self {
        let s = obs.symbols();
        Ram {
            obs,
            g_player_party: s.get("gPlayerParty").expect("gPlayerParty").addr,
            g_object_events: s.get("gObjectEvents").expect("gObjectEvents").addr,
        }
    }

    pub fn party(&self, emu: &mut Emu) -> Vec<Mon> {
        let n = self.obs.party_count(emu).min(6) as u32;
        (0..n)
            .map(|i| read_mon(emu, self.g_player_party + i * MON_SIZE))
            .collect()
    }

    fn key(&self, emu: &mut Emu) -> u32 {
        self.obs
            .save_block2(emu)
            .map(|sb2| emu.read32(sb2 + SB2_ENCRYPTION_KEY))
            .unwrap_or(0)
    }

    /// Money: `gSaveBlock1Ptr->money ^ encryptionKey` (`GetMoney`, `decompiled/src/money.c`).
    pub fn money(&self, emu: &mut Emu) -> u32 {
        let Some(sb) = self.obs.save_block1(emu) else {
            return 0;
        };
        emu.read32(sb + sb1::MONEY) ^ self.key(emu)
    }

    /// One pocket's `(item, quantity)` slots; quantities are XORed with the low half of the
    /// encryption key (`GetBagItemQuantity`, `decompiled/src/item.c`).
    pub fn pocket(&self, emu: &mut Emu, pocket: usize) -> Vec<(u16, u16)> {
        let Some(sb) = self.obs.save_block1(emu) else {
            return vec![];
        };
        let k = self.key(emu) as u16;
        let (off, n) = POCKETS[pocket];
        (0..n)
            .map(|i| {
                let a = sb + off + i * 4;
                (emu.read16(a), emu.read16(a + 2) ^ k)
            })
            .filter(|&(id, _)| id != 0)
            .collect()
    }

    /// Experiment aid (`battle-test --give`, honesty 4): put `qty` of `item` into the first empty slot of
    /// `pocket` (quantity XORed with the key's low half, as `pocket` reads it). Never used by a played run.
    pub fn give(&self, emu: &mut Emu, pocket: usize, item: u16, qty: u16) -> bool {
        let Some(sb) = self.obs.save_block1(emu) else {
            return false;
        };
        let k = self.key(emu) as u16;
        let (off, n) = POCKETS[pocket];
        for i in 0..n {
            let a = sb + off + i * 4;
            if emu.read16(a) == 0 {
                let q = qty ^ k;
                for (j, b) in item
                    .to_le_bytes()
                    .into_iter()
                    .chain(q.to_le_bytes())
                    .enumerate()
                {
                    emu.write8(a + j as u32, b);
                }
                return true;
            }
        }
        false
    }

    /// Quantity of `item` anywhere in the bag.
    pub fn count(&self, emu: &mut Emu, item: u16) -> u16 {
        (0..POCKETS.len())
            .flat_map(|p| self.pocket(emu, p))
            .filter(|&(id, _)| id == item)
            .map(|(_, q)| q)
            .sum()
    }

    /// PC item slots (not encrypted: `pcItems` quantities are stored plain, `decompiled/src/item.c`
    /// `GetPcItemQuantity`).
    pub fn pc_items(&self, emu: &mut Emu) -> Vec<(u16, u16)> {
        let Some(sb) = self.obs.save_block1(emu) else {
            return vec![];
        };
        (0..30)
            .map(|i| {
                let a = sb + sb1::PC_ITEMS + i * 4;
                (emu.read16(a), emu.read16(a + 2))
            })
            .filter(|&(id, _)| id != 0)
            .collect()
    }

    /// The player object's `currentElevation` (low nibble of byte 0x0B of `struct ObjectEvent`,
    /// `decompiled/include/global.fieldmap.h`; the player has local id `OBJ_EVENT_ID_PLAYER` 0xFF).
    pub fn player_elevation(&self, emu: &mut Emu) -> u8 {
        (0..OBJ_COUNT)
            .map(|i| self.g_object_events + i * OBJ_SIZE)
            .find(|&b| emu.read32(b) & 1 != 0 && emu.read8(b + 0x08) == 0xFF)
            .map_or(0, |b| emu.read8(b + 0x0B) & 0xF)
    }

    /// Active non-player object events, in map coordinates.
    pub fn objects(&self, emu: &mut Emu) -> Vec<LiveObject> {
        (0..OBJ_COUNT)
            .filter_map(|i| {
                let b = self.g_object_events + i * OBJ_SIZE;
                let flags = emu.read32(b);
                if flags & 1 == 0 || flags & (1 << 16) != 0 {
                    return None;
                }
                Some(LiveObject {
                    local_id: emu.read8(b + 0x08),
                    graphics: emu.read8(b + 0x05),
                    x: emu.read16(b + 0x10) as i16 - MAP_OFFSET,
                    y: emu.read16(b + 0x12) as i16 - MAP_OFFSET,
                    px: emu.read16(b + 0x14) as i16 - MAP_OFFSET,
                    py: emu.read16(b + 0x16) as i16 - MAP_OFFSET,
                    facing: emu.read8(b + 0x18) & 0x0F,
                    movement: emu.read8(b + 0x06),
                    trainer_type: emu.read8(b + 0x07),
                    range: emu.read8(b + 0x1D),
                    map: (emu.read8(b + 0x0A), emu.read8(b + 0x09)),
                    elevation: emu.read8(b + 0x0B) & 0x0F,
                    invisible: flags & (1 << 13) != 0,
                })
            })
            .collect()
    }

    /// The player's facing direction (`facingDirection`, low nibble of byte 0x18 of the player's
    /// object event; `DIR_SOUTH` 1, `DIR_NORTH` 2, `DIR_WEST` 3, `DIR_EAST` 4,
    /// `decompiled/include/constants/event_object_movement.h`).
    pub fn player_facing(&self, emu: &mut Emu) -> u8 {
        for i in 0..OBJ_COUNT {
            let b = self.g_object_events + i * OBJ_SIZE;
            let flags = emu.read32(b);
            if flags & 1 != 0 && flags & (1 << 16) != 0 {
                return emu.read8(b + 0x18) & 0x0F;
            }
        }
        0
    }
}
