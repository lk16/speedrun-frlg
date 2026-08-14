//! Mon creation as `Random()` consumption.
//!
//! Two paths matter before Brock, and they consume differently:
//!
//! - **The starter** (`givemon PLAYER_STARTER_SPECIES, 5`,
//!   `decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc:1122` →
//!   `ScriptGiveMon`, `src/script_pokemon_util.c:55` →
//!   `CreateMon(mon, species, level, 32, 0, 0, OT_ID_PLAYER_ID, 0)`):
//!   PID is a bare `Random32()` (`src/pokemon.c:1778`), no reroll loop --
//!   the anti-shiny `do/while` at `src/pokemon.c:1786-1792` only runs for
//!   `OT_ID_RANDOM_NO_SHINY` -- then two IV rolls. **Exactly 4 calls.**
//! - **A wild mon** (`GenerateWildMon`, `src/wild_encounter.c:226-241`):
//!   `CreateMonWithNature(…, Random() % 25)` -- one nature roll, then
//!   `pid = Random32()` rerolled until `pid % 25 == nature`
//!   (`src/pokemon.c:1864-1875`), then the same two IV rolls.
//!
//! Nature is `PID % 25` in both (`GetNatureFromPersonality`,
//! `src/pokemon.c:5020-5023`); gender and ability derive from the PID with
//! no further rolls (`src/pokemon.c:2743-2746`, `:1855-1859`).

use frlg_rng::Rng;

use crate::stats::Ivs;

/// `Random32()` = `(Random() | (Random() << 16))`
/// (`decompiled/include/random.h:14`). Which call becomes the low half is
/// C evaluation order -- unspecified in the source, decided by the original
/// compiler. Measured on the ROM (2026-08-12, the committed route's starter:
/// PID `0xbfbc8df4` from consecutive outputs `8df4`, `bfbc`): the **first**
/// call is the low half. `tests/emulator.rs` re-checks this on every run and
/// fails if the halves ever read the other way.
pub fn random32(rng: &mut Rng) -> u32 {
    let first = rng.random() as u32;
    let second = rng.random() as u32;
    first | (second << 16)
}

/// What creation rolled: everything about a mon that is not deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Genome {
    pub pid: u32,
    pub ivs: Ivs,
}

impl Genome {
    /// `GetNatureFromPersonality` (`decompiled/src/pokemon.c:5020-5023`).
    pub fn nature(&self) -> u8 {
        (self.pid % 25) as u8
    }
}

/// The gift-mon path: PID then IVs, 4 calls, no loops
/// (`CreateBoxMon`, `decompiled/src/pokemon.c:1778,1836-1852`, reached with
/// `fixedIV = USE_RANDOM_IVS`, no fixed personality, `OT_ID_PLAYER_ID`).
pub fn gift_mon(rng: &mut Rng) -> Genome {
    let pid = random32(rng);
    let ivs = Ivs::unpack(rng.random(), rng.random());
    Genome { pid, ivs }
}

/// The wild path *after* the nature roll: `CreateMonWithNature`'s reroll
/// loop (`decompiled/src/pokemon.c:1864-1875`), then IVs. Returns the genome
/// and how many `Random()` calls it consumed (2 per PID iteration + 2).
pub fn wild_mon_from_nature(rng: &mut Rng, nature: u8) -> (Genome, u32) {
    let mut calls = 0u32;
    let pid = loop {
        let pid = random32(rng);
        calls += 2;
        if pid % 25 == nature as u32 {
            break pid;
        }
    };
    let ivs = Ivs::unpack(rng.random(), rng.random());
    (Genome { pid, ivs }, calls + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gift_mon_consumes_exactly_four() {
        let mut rng = Rng(0x1234_5678);
        let start = rng;
        gift_mon(&mut rng);
        assert_eq!(start.distance_to(rng), 4);
    }

    #[test]
    fn wild_mon_pid_has_the_asked_nature() {
        for seed in [0u32, 1, 0xDEAD_BEEF, 0x8000_0000] {
            for nature in 0..25u8 {
                let mut rng = Rng(seed);
                let start = rng;
                let (genome, calls) = wild_mon_from_nature(&mut rng, nature);
                assert_eq!(genome.nature(), nature);
                assert_eq!(start.distance_to(rng), calls);
            }
        }
    }
}
