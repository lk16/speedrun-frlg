//! The game's RNG, reimplemented so a search can ask "what will the stream do"
//! without paying for an emulator frame.
//!
//! The game's `Random()` is an LCG over the u32 `gRngValue`
//! (`decompiled/src/random.c`):
//!
//! ```c
//! gRngValue = ISO_RANDOMIZE1(gRngValue);   // RAND_MULT * val + 24691
//! return gRngValue >> 16;
//! ```
//!
//! with `RAND_MULT 1103515245` (`decompiled/include/random.h:18-19`). The
//! increment 24691 is odd and `RAND_MULT ≡ 1 (mod 4)`, so the LCG is
//! full-period over 2^32: every state occurs exactly once per 2^32 steps,
//! which is what makes [`Rng::distance_to`] total.
//!
//! The overworld advances the stream once per frame from the VBlank
//! interrupt (`Random()` in `EnableVCountIntrAtLine150`'s registered VBlank
//! path, `decompiled/src/main.c:412`), so "frames" and "steps" coincide
//! whenever nothing else consumes the stream; every extra consumer shows up
//! as [`Rng::distance_to`] > 1 between two consecutive frames' values.
//!
//! Correctness against the real thing is not argued, it is tested:
//! `tests/emulator.rs` replays the committed route and checks this model
//! against `gRngValue` read out of libmgba on every frame.

/// `RAND_MULT`, `decompiled/include/random.h:18`.
pub const MULT: u32 = 1_103_515_245;
/// The `ISO_RANDOMIZE1` increment, `decompiled/include/random.h:19`.
pub const INC: u32 = 24_691;

/// Multiplicative inverse of [`MULT`] mod 2^32 (`MULT` is odd, so it exists);
/// verified by a test, used to step the stream backwards.
const MULT_INV: u32 = 4_005_161_829;

/// The affine maps for 2^k steps, `k = 0..32`: `POW2[k] = (a, c)` with
/// `jump(2^k)(x) = a·x + c`. Computed at compile time by squaring, so `jump`
/// and `distance_to` pay no setup per call -- they are the inner loop of any
/// stream search.
const POW2: [(u32, u32); 32] = {
    let mut table = [(0u32, 0u32); 32];
    let (mut a, mut c) = (MULT, INC);
    let mut k = 0;
    while k < 32 {
        table[k] = (a, c);
        c = a.wrapping_mul(c).wrapping_add(c);
        a = a.wrapping_mul(a);
        k += 1;
    }
    table
};

/// One `gRngValue` state. Copy-cheap on purpose: a search forks these by the
/// thousand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rng(pub u32);

impl Rng {
    /// The state after `SeedRng(seed)` (`decompiled/src/random.c:16-19`):
    /// the u16 seed zero-extended.
    pub fn seeded(seed: u16) -> Self {
        Rng(seed as u32)
    }

    /// `Random()`: advance one step, return the top 16 bits of the new state.
    pub fn random(&mut self) -> u16 {
        self.0 = self.0.wrapping_mul(MULT).wrapping_add(INC);
        (self.0 >> 16) as u16
    }

    /// The state one step ahead, without mutating.
    #[must_use]
    pub fn next(self) -> Self {
        Rng(self.0.wrapping_mul(MULT).wrapping_add(INC))
    }

    /// The state one step back: `x = (x' - INC) * MULT⁻¹`.
    #[must_use]
    pub fn prev(self) -> Self {
        Rng(self.0.wrapping_sub(INC).wrapping_mul(MULT_INV))
    }

    /// The state `n` steps ahead in O(log n), applying the precomputed
    /// power-of-two maps for the set bits of `n`.
    #[must_use]
    pub fn jump(self, n: u32) -> Self {
        let mut x = self.0;
        let mut n = n;
        while n != 0 {
            let k = n.trailing_zeros();
            let (a, c) = POW2[k as usize];
            x = a.wrapping_mul(x).wrapping_add(c);
            n &= n - 1;
        }
        Rng(x)
    }

    /// Fills `out` with successive `Random()` returns, advancing `self` by
    /// `out.len()` steps: the bulk form for scans over candidate streams.
    pub fn fill(&mut self, out: &mut [u16]) {
        let mut x = self.0;
        for slot in out {
            x = x.wrapping_mul(MULT).wrapping_add(INC);
            *slot = (x >> 16) as u16;
        }
        self.0 = x;
    }

    /// How many steps ahead `target` is: the unique `n` in `[0, 2^32)` with
    /// `self.jump(n) == target`. Total because the LCG is full-period.
    ///
    /// Determined bit by bit: an LCG over 2^32 with odd increment and
    /// `a ≡ 1 (mod 4)` is full-period modulo every 2^k, so `n mod 2^k` is
    /// pinned down by the low k bits of the states alone.
    pub fn distance_to(self, target: Rng) -> u32 {
        let mut n: u32 = 0;
        let mut cur = self.0; // == self.jump(n)
        for (k, &(a, c)) in POW2.iter().enumerate() {
            if (cur ^ target.0) & (1 << k) != 0 {
                // Low k bits already match and bit k does not: step 2^k.
                cur = a.wrapping_mul(cur).wrapping_add(c);
                n |= 1 << k;
            }
        }
        debug_assert_eq!(cur, target.0);
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mult_inv_is_the_inverse() {
        assert_eq!(MULT.wrapping_mul(MULT_INV), 1);
    }

    #[test]
    fn random_matches_the_iso_formula() {
        // ISO_RANDOMIZE1 written out longhand, straight from random.h.
        let mut rng = Rng::seeded(0);
        let first = rng.random();
        assert_eq!(rng.0, 24_691);
        assert_eq!(first, 0);
        let second = rng.random();
        assert_eq!(
            rng.0,
            1_103_515_245u32.wrapping_mul(24_691).wrapping_add(24_691)
        );
        assert_eq!(second, (rng.0 >> 16) as u16);
    }

    #[test]
    fn prev_undoes_next() {
        let mut state = Rng(0xDEAD_BEEF);
        for _ in 0..1000 {
            assert_eq!(state.next().prev(), state);
            assert_eq!(state.prev().next(), state);
            state = state.next();
        }
    }

    #[test]
    fn jump_matches_naive_stepping() {
        let start = Rng(0x1234_5678);
        let mut naive = start;
        for n in 0..=4096u32 {
            assert_eq!(start.jump(n), naive, "jump({n})");
            naive = naive.next();
        }
        // A few large ones against jump-composition rather than 2^31 steps.
        for &(a, b) in &[(1u32 << 20, 3), (0xFFFF_0000, 0xFFFF), (7, 1 << 30)] {
            assert_eq!(start.jump(a).jump(b), start.jump(a.wrapping_add(b)));
        }
    }

    #[test]
    fn distance_inverts_jump() {
        let start = Rng(0xCAFE_F00D);
        for n in [0u32, 1, 2, 3, 15, 100, 65_535, 1 << 20, u32::MAX] {
            assert_eq!(start.distance_to(start.jump(n)), n, "distance for jump {n}");
        }
        // Every state pair has a distance, and jumping it lands exactly.
        let a = Rng(0x0000_0001);
        let b = Rng(0x8765_4321);
        let d = a.distance_to(b);
        assert_eq!(a.jump(d), b);
    }

    #[test]
    fn fill_equals_repeated_random() {
        let mut bulk = Rng(0xBEEF);
        let mut single = Rng(0xBEEF);
        let mut buf = [0u16; 257];
        bulk.fill(&mut buf);
        for (index, &value) in buf.iter().enumerate() {
            assert_eq!(value, single.random(), "output {index}");
        }
        assert_eq!(bulk, single);
    }

    /// Not a benchmark, a floor: the ops a search leans on must stay orders
    /// of magnitude under an emulator frame (~1 ms). Run with
    /// `cargo test --release -- --ignored --nocapture`.
    #[test]
    #[ignore = "timing floor, run explicitly in release"]
    fn ops_are_fast_enough_to_ignore() {
        let rounds = 10_000_000u32;

        let start = std::time::Instant::now();
        let mut rng = Rng(1);
        let mut acc = 0u16;
        for _ in 0..rounds {
            acc = acc.wrapping_add(rng.random());
        }
        let per_step = start.elapsed() / rounds;

        let start = std::time::Instant::now();
        let mut d = 0u32;
        for n in 0..rounds {
            d = d.wrapping_add(Rng(n).distance_to(Rng(acc as u32 ^ n)));
        }
        let per_distance = start.elapsed() / rounds;

        println!("random(): {per_step:?}/op, distance_to(): {per_distance:?}/op (acc {acc} {d})");
        assert!(per_step < std::time::Duration::from_nanos(50));
        assert!(per_distance < std::time::Duration::from_nanos(1_000));
    }

    #[test]
    fn full_period_low_bits() {
        // The low k bits cycle with period 2^k -- the property distance_to
        // rests on. Check k = 8 exhaustively.
        let mut state = Rng(0);
        let mut seen = [false; 256];
        for _ in 0..256 {
            let low = (state.0 & 0xFF) as usize;
            assert!(!seen[low], "low byte repeated before 256 steps");
            seen[low] = true;
            state = state.next();
        }
        assert_eq!(state.0 & 0xFF, 0, "period of low byte is exactly 256");
    }
}
