//! The human's own randomness: splitmix64 seeding a xoshiro256++ stream, keyed by
//! `(seed, attempt, stream)` so attempts are reproducible and plans can be paired on
//! common random numbers. This is *not* the game's RNG (that is `frlg-rng`).

pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a, used to key a stream by segment name.
pub fn hash_str(s: &str) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

#[derive(Clone)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64, attempt: u64, stream: u64) -> Self {
        let mut sm = seed
            ^ attempt.wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ stream.wrapping_mul(0xA076_1D64_78BD_642F);
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        Rng { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[0]
            .wrapping_add(self.s[3])
            .rotate_left(23)
            .wrapping_add(self.s[0]);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * n as u64) >> 32) as u32
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }

    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// Failures before the first success, success probability `p` per trial.
    pub fn geometric(&mut self, p: f64) -> u64 {
        if p >= 1.0 {
            return 0;
        }
        if p <= 0.0 {
            return u64::MAX / 4;
        }
        let u = 1.0 - self.f64();
        (u.ln() / (1.0 - p).ln()).floor() as u64
    }
}

/// The game's own RNG stream: the LCG of `decompiled/src/random.c` (via `frlg-rng`,
/// emulator-verified there), advanced once per frame by the VBlank handler
/// (`decompiled/src/main.c:412`) and consumed by every game roll.
///
/// Within a segment the stream is serial and real: frame time advances it (`frames`),
/// battle and encounter rolls consume it in order, so rolls have exactly the game's
/// 16-bit arithmetic and in-stream correlation. Across segments it is re-keyed from the
/// human stream so plans stay pairable on common random numbers — statistically
/// indistinguishable from continuity because the real seed is a free-running 16-bit
/// timer (`SeedRngAndSetTrainerId`, `decompiled/src/main.c:264`) and human input timing
/// jitters the stream position anyway.
#[derive(Clone, Copy)]
pub struct GameRng(pub frlg_rng::Rng);

impl GameRng {
    /// Re-keyed from 32 bits of the human stream (any u32 is a reachable LCG state).
    pub fn from_u32(v: u32) -> Self {
        GameRng(frlg_rng::Rng(v))
    }

    /// `Random()`: one step, top 16 bits (`decompiled/src/random.c:10-14`).
    pub fn roll(&mut self) -> u16 {
        self.0.random()
    }

    /// `n` frames pass: the VBlank advance (`decompiled/src/main.c:412`), O(log n).
    pub fn frames(&mut self, n: u32) {
        self.0 = self.0.jump(n);
    }

    /// `Random() % 100 < p`: the secondary-effect and AI percent roll shape.
    pub fn pct(&mut self, p: u32) -> bool {
        (self.roll() as u32) % 100 < p
    }

    /// `Random() % n`.
    pub fn modn(&mut self, n: u32) -> u32 {
        (self.roll() as u32) % n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_rng_matches_the_lcg() {
        let mut g = GameRng::from_u32(0);
        // First Random() after SeedRng(0): state = 24691, top 16 bits = 0.
        assert_eq!(g.roll(), (24691u32 >> 16) as u16);
        let mut a = GameRng::from_u32(12345);
        let mut b = GameRng::from_u32(12345);
        a.frames(1000);
        for _ in 0..1000 {
            b.roll();
        }
        assert_eq!(a.0 .0, b.0 .0);
    }

    #[test]
    fn streams_are_reproducible_and_distinct() {
        let mut a = Rng::new(1, 2, 3);
        let mut b = Rng::new(1, 2, 3);
        let mut c = Rng::new(1, 2, 4);
        let (x, y, z) = (a.next_u64(), b.next_u64(), c.next_u64());
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn below_is_in_range_and_roughly_uniform() {
        let mut r = Rng::new(9, 9, 9);
        let mut counts = [0u32; 5];
        for _ in 0..50_000 {
            counts[r.below(5) as usize] += 1;
        }
        for c in counts {
            assert!((9_000..11_000).contains(&c), "{counts:?}");
        }
    }
}
