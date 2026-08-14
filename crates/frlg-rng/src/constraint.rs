//! A fixed roll timeline as constraints on the state it starts from.
//!
//! A battle whose strategy (inputs, and therefore frame pacing) is decided
//! consumes `Random()` at *known offsets* after its start state `s`: the
//! n-th call returns `s.jump(n) >> 16`, and `jump(n)` is affine
//! ([`Rng::jump_coeffs`]), so "the roll n calls after `s`" costs one
//! multiply-add from `s` directly -- no stepping. A requirement like "the
//! crit roll at call 214 must be a multiple of 16" is then a predicate on an
//! affine image of `s`, and a whole battle is a conjunction of them: a
//! [`ConstraintSet`].
//!
//! That inversion turns two questions into arithmetic:
//!
//! - **"how long must I wait so the battle plays out this way?"** --
//!   [`ConstraintSet::first_wait`]: scan the *reachable* states
//!   `anchor.jump(stride·w)` (the stream advances a fixed stride per waiting
//!   frame; overworld 1, battle 2 -- `decompiled/src/main.c:412`,
//!   `src/battle_main.c:1650`) and return the first satisfying `w`. One
//!   multiply-add per waited frame plus one per checked constraint.
//! - **"which start states satisfy it at all?"** -- [`ConstraintSet::count_all`]
//!   / [`ConstraintSet::scan_states`]: the full 2^32 space, brute force. The
//!   first (most selective) constraint is evaluated incrementally (its
//!   affine image steps by `a` as `s` steps by 1), the rest only on a pass,
//!   so the whole space costs about `2^32 · (1 + density₁·k)` multiply-adds.
//!
//! What this module deliberately does not know: *where* the offsets come
//! from. Offsets are frame pacing, and pacing is measured per fight
//! (`frlg-battle::pacing`) -- a constraint set is only as real as the
//! pacing model that produced its offsets. Extraction lives with the fight
//! model; this is the solver.

use crate::Rng;

/// A predicate on one 16-bit roll. The two shapes cover every decisive roll
/// the pre-Brock game makes -- crit (`% 16 == 0`,
/// `decompiled/src/battle_script_commands.c:1199`), accuracy (`% 100`,
/// `:1093`), damage variance (`% 16`, `:1560-1568`), AI viability (`% 256`,
/// `data/battle_ai_scripts.s:1137,1149`), the tie-break (`% 2`,
/// `src/battle_ai_script_commands.c:408`), escapes (`% 256`) -- because the
/// game only ever consumes a roll through `%` and compares the residue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pred {
    /// The roll is exactly this value.
    Exact(u16),
    /// `lo <= roll % m <= hi` (inclusive both ends). `m` must be nonzero
    /// and `lo <= hi < m`; a "not in range" wish is the complement range
    /// where contiguous (crit-must-miss is `1..=15 mod 16`), or two
    /// constraints on the same offset where not.
    ModRange { m: u16, lo: u16, hi: u16 },
}

impl Pred {
    /// Does `roll` pass?
    #[inline]
    pub fn passes(self, roll: u16) -> bool {
        match self {
            Pred::Exact(v) => roll == v,
            Pred::ModRange { m, lo, hi } => {
                let r = roll % m;
                lo <= r && r <= hi
            }
        }
    }

    /// How many of the 65536 possible rolls pass -- exact, used to order a
    /// set most-selective-first so the conjunction short-circuits early.
    pub fn pass_count(self) -> u32 {
        match self {
            Pred::Exact(_) => 1,
            Pred::ModRange { m, lo, hi } => {
                let m = m as u32;
                let width = (hi - lo + 1) as u32;
                let full_cycles = 65536 / m;
                let tail = 65536 % m; // residues 0..tail occur once more
                let lo = lo as u32;
                let hi = hi as u32;
                full_cycles * width + tail.clamp(lo, hi + 1).saturating_sub(lo)
            }
        }
    }
}

/// One requirement: the `offset`-th `Random()` call after the start state
/// (1-based: offset 1 is the first call) must pass `pred`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraint {
    pub offset: u32,
    pub pred: Pred,
}

/// A constraint with its offset compiled to the affine map of `jump(offset)`.
/// Public fields so a caller can build its own specialised checker (the
/// benchmarked "dedicated" shape) from the same compilation.
#[derive(Debug, Clone, Copy)]
pub struct Compiled {
    /// `roll = ((a·s + c) >> 16) as u16` for start state `s`.
    pub a: u32,
    pub c: u32,
    pub pred: Pred,
    /// The original call offset, kept for shifting and diagnostics.
    pub offset: u32,
}

impl Compiled {
    #[inline]
    fn roll(&self, s: u32) -> u16 {
        (self.a.wrapping_mul(s).wrapping_add(self.c) >> 16) as u16
    }
}

/// A conjunction of compiled constraints, ordered most-selective-first.
#[derive(Debug, Clone)]
pub struct ConstraintSet {
    cs: Vec<Compiled>,
}

impl ConstraintSet {
    /// Compile a set. Panics on an offset of 0 (the start state itself is
    /// not a roll) or a malformed `ModRange`.
    pub fn new(constraints: &[Constraint]) -> Self {
        let mut cs: Vec<Compiled> = constraints
            .iter()
            .map(|k| {
                assert!(k.offset >= 1, "offset 0 is the start state, not a roll");
                if let Pred::ModRange { m, lo, hi } = k.pred {
                    assert!(m > 0 && lo <= hi && hi < m, "malformed {:?}", k.pred);
                }
                let (a, c) = Rng::jump_coeffs(k.offset);
                Compiled {
                    a,
                    c,
                    pred: k.pred,
                    offset: k.offset,
                }
            })
            .collect();
        cs.sort_by_key(|k| k.pred.pass_count());
        ConstraintSet { cs }
    }

    /// The compiled constraints, most selective first.
    pub fn compiled(&self) -> &[Compiled] {
        &self.cs
    }

    /// Estimated fraction of random states that satisfy the set (product of
    /// per-constraint densities; exact only when constraints are
    /// independent, which distinct offsets nearly are).
    pub fn density(&self) -> f64 {
        self.cs
            .iter()
            .map(|k| k.pred.pass_count() as f64 / 65536.0)
            .product()
    }

    /// Does start state `s` satisfy every constraint?
    #[inline]
    pub fn satisfied(&self, s: Rng) -> bool {
        self.cs.iter().all(|k| k.pred.passes(k.roll(s.0)))
    }

    /// The satisfying waits in `0..max_wait`: `w` such that
    /// `anchor.jump(stride * w)` satisfies the set. `stride` is how many
    /// stream steps one waited frame costs where the wait is spent.
    pub fn wait_hits(&self, anchor: Rng, stride: u32, max_wait: u32) -> Vec<u32> {
        let mut hits = Vec::new();
        self.scan_waits(anchor, stride, max_wait, |w| {
            hits.push(w);
            true
        });
        hits
    }

    /// The first satisfying wait in `0..max_wait`, if any.
    pub fn first_wait(&self, anchor: Rng, stride: u32, max_wait: u32) -> Option<u32> {
        let mut found = None;
        self.scan_waits(anchor, stride, max_wait, |w| {
            found = Some(w);
            false
        });
        found
    }

    /// Walk waits `0..max_wait` in order, calling `visit` on each satisfying
    /// one until it returns `false`. The anchor state advances by the affine
    /// map of `stride` once per wait -- one multiply-add -- rather than by
    /// stepping.
    pub fn scan_waits(
        &self,
        anchor: Rng,
        stride: u32,
        max_wait: u32,
        mut visit: impl FnMut(u32) -> bool,
    ) {
        let (sa, sc) = Rng::jump_coeffs(stride);
        let mut s = anchor.0;
        for w in 0..max_wait {
            if self.satisfied(Rng(s)) && !visit(w) {
                return;
            }
            s = sa.wrapping_mul(s).wrapping_add(sc);
        }
    }

    /// Scan start states `start`, `start+1`, ... (`count` of them, wrapping
    /// mod 2^32), calling `emit` for each satisfying state. The first
    /// constraint's affine image is maintained incrementally (it moves by
    /// `a` when `s` moves by 1), so a non-passing state costs one add and
    /// one compare.
    pub fn scan_states(&self, start: u32, count: u64, emit: &mut impl FnMut(u32)) {
        let Some((first, rest)) = self.cs.split_first() else {
            // An empty set is satisfied by every state.
            let mut s = start;
            for _ in 0..count {
                emit(s);
                s = s.wrapping_add(1);
            }
            return;
        };
        let mut s = start;
        let mut r0 = first.a.wrapping_mul(s).wrapping_add(first.c);
        for _ in 0..count {
            if first.pred.passes((r0 >> 16) as u16) && rest.iter().all(|k| k.pred.passes(k.roll(s)))
            {
                emit(s);
            }
            s = s.wrapping_add(1);
            r0 = r0.wrapping_add(first.a);
        }
    }

    /// How many states in `start..start+count` (wrapping) satisfy the set,
    /// `threads`-wide.
    pub fn count_range(&self, start: u32, count: u64, threads: usize) -> u64 {
        let threads = (threads.max(1) as u64).min(count.max(1));
        let chunk = count.div_ceil(threads);
        let mut total = 0u64;
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let lo = start.wrapping_add((t * chunk) as u32);
                    let count = chunk.min(count - t * chunk);
                    scope.spawn(move || {
                        let mut n = 0u64;
                        self.scan_states(lo, count, &mut |_| n += 1);
                        n
                    })
                })
                .collect();
            for h in handles {
                total += h.join().expect("solver thread");
            }
        });
        total
    }

    /// How many of all 2^32 start states satisfy the set, `threads`-wide.
    /// Full-space brute force -- release-build territory (~2 s at 16
    /// threads for a selective set).
    pub fn count_all(&self, threads: usize) -> u64 {
        self.count_range(0, 1 << 32, threads)
    }

    /// A copy of the set with every offset moved by `delta` calls -- the
    /// same fight entered `delta` stream steps later (positive) or earlier.
    /// What "the same fight" means is the caller's problem: pacing must be
    /// unchanged for the shifted offsets to describe anything real.
    pub fn shifted(&self, delta: i64) -> Self {
        let constraints: Vec<Constraint> = self
            .cs
            .iter()
            .map(|k| Constraint {
                offset: (k.offset as i64 + delta)
                    .try_into()
                    .expect("shifted offset must stay in 1..2^32"),
                pred: k.pred,
            })
            .collect();
        ConstraintSet::new(&constraints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constraints copied off an actual stream must accept their anchor.
    fn set_from_stream(anchor: Rng, offsets: &[u32]) -> ConstraintSet {
        let constraints: Vec<Constraint> = offsets
            .iter()
            .map(|&offset| Constraint {
                offset,
                pred: Pred::Exact((anchor.jump(offset).0 >> 16) as u16),
            })
            .collect();
        ConstraintSet::new(&constraints)
    }

    #[test]
    fn compiled_rolls_match_stepping() {
        let anchor = Rng(0x1357_9BDF);
        let set = ConstraintSet::new(&[
            Constraint {
                offset: 1,
                pred: Pred::Exact(0),
            },
            Constraint {
                offset: 4242,
                pred: Pred::Exact(0),
            },
        ]);
        let mut stream = anchor;
        let mut rolls = vec![0u16; 4242];
        stream.fill(&mut rolls);
        for k in set.compiled() {
            assert_eq!(k.roll(anchor.0), rolls[k.offset as usize - 1]);
        }
    }

    #[test]
    fn stream_derived_set_accepts_its_anchor() {
        let anchor = Rng(0xed94_271d);
        let set = set_from_stream(anchor, &[1, 7, 100, 2500, 5000]);
        assert!(set.satisfied(anchor));
        assert!(!set.satisfied(anchor.next()));
    }

    #[test]
    fn pass_count_is_exact() {
        for pred in [
            Pred::Exact(1234),
            Pred::ModRange {
                m: 16,
                lo: 0,
                hi: 0,
            },
            Pred::ModRange {
                m: 16,
                lo: 1,
                hi: 15,
            },
            Pred::ModRange {
                m: 100,
                lo: 0,
                hi: 94,
            },
            Pred::ModRange {
                m: 256,
                lo: 50,
                hi: 255,
            },
            Pred::ModRange { m: 3, lo: 2, hi: 2 },
            Pred::ModRange { m: 1, lo: 0, hi: 0 },
        ] {
            let naive = (0..=u16::MAX).filter(|&r| pred.passes(r)).count() as u32;
            assert_eq!(pred.pass_count(), naive, "{pred:?}");
        }
    }

    #[test]
    fn wait_scan_finds_planted_states() {
        let anchor = Rng(0xCAFE_F00D);
        for stride in [1u32, 2] {
            for wait in [0u32, 1, 17, 999] {
                let target = anchor.jump(stride * wait);
                let set = set_from_stream(target, &[3, 11, 250]);
                assert_eq!(
                    set.first_wait(anchor, stride, wait + 1),
                    Some(wait),
                    "stride {stride}, wait {wait}"
                );
                assert_eq!(set.wait_hits(anchor, stride, wait + 1), vec![wait]);
            }
        }
    }

    #[test]
    fn state_scan_agrees_with_naive_check() {
        // Dense enough that the window has real hits: 1/16 · 1/2.
        let set = ConstraintSet::new(&[
            Constraint {
                offset: 5,
                pred: Pred::ModRange {
                    m: 16,
                    lo: 0,
                    hi: 0,
                },
            },
            Constraint {
                offset: 60,
                pred: Pred::ModRange { m: 2, lo: 1, hi: 1 },
            },
        ]);
        let start = 0xFFFF_FF00u32; // crosses the wrap on purpose
        let count = 1u64 << 12;
        let mut scanned = Vec::new();
        set.scan_states(start, count, &mut |s| scanned.push(s));
        let mut naive = Vec::new();
        let mut s = start;
        for _ in 0..count {
            if set.satisfied(Rng(s)) {
                naive.push(s);
            }
            s = s.wrapping_add(1);
        }
        assert!(!naive.is_empty(), "test window has no hits to compare");
        assert_eq!(scanned, naive);
    }

    #[test]
    fn count_range_matches_naive_across_threads_and_the_wrap() {
        let set = ConstraintSet::new(&[
            Constraint {
                offset: 5,
                pred: Pred::ModRange {
                    m: 16,
                    lo: 0,
                    hi: 3,
                },
            },
            Constraint {
                offset: 60,
                pred: Pred::ModRange {
                    m: 100,
                    lo: 0,
                    hi: 49,
                },
            },
        ]);
        let start = 0xFFFF_8000u32;
        let count = 1u64 << 18;
        let mut naive = 0u64;
        let mut s = start;
        for _ in 0..count {
            naive += set.satisfied(Rng(s)) as u64;
            s = s.wrapping_add(1);
        }
        assert!(naive > 0);
        for threads in [1, 2, 7, 16] {
            assert_eq!(set.count_range(start, count, threads), naive, "{threads}");
        }
    }

    /// The exact full-space count of a single-constraint set: the affine
    /// image of `s` is a bijection on 2^32, so each of the 65536 roll values
    /// is produced by exactly 2^16 states, and the count is
    /// `pass_count * 2^16`. Run in release: 2^32 states.
    #[test]
    #[ignore = "full 2^32 sweep, run explicitly in release"]
    fn count_all_is_exact_for_one_constraint() {
        let set = ConstraintSet::new(&[Constraint {
            offset: 123,
            pred: Pred::ModRange {
                m: 16,
                lo: 3,
                hi: 3,
            },
        }]);
        let expected = set.compiled()[0].pred.pass_count() as u64 * (1 << 16);
        assert_eq!(set.count_all(16), expected);
    }

    #[test]
    fn shifted_set_accepts_the_shifted_anchor() {
        let anchor = Rng(0x0BAD_F00D);
        let set = set_from_stream(anchor, &[10, 20, 300]);
        let earlier = anchor.prev().prev().prev();
        assert!(set.shifted(3).satisfied(earlier));
        let later = anchor.jump(5);
        assert!(set.shifted(-5).satisfied(later));
    }
}
