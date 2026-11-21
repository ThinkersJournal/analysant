// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gate B sampling (SPEC-STEP1 5.2): the language-independent shuffle, the stop rule, census handling, the second-judge
//! selection and the rule-of-three bound. Nothing here judges a finding; it fixes *which* findings are read and *when* to stop,
//! so that two people with the same findings and the same seed read the same ones in the same order.
//!
//! The algorithm is written down, not borrowed from a library: SplitMix64 (Vigna's public-domain `splitmix64.c`), a rejection-sampled
//! bounded draw, and a backwards Fisher-Yates. `docs/GATE-B-PROTOCOL.md` restates it for people who do not read Rust.

/// SplitMix64's increment, the odd constant 2^64 / golden ratio.
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// Salt XORed into the report seed to pick the second judge's items, so that choice is independent of the shuffle but still
/// reproducible from the one seed the report states.
pub const SECOND_JUDGE_SALT: u64 = 0x5EC0_D0DE_0A11_CE55;

/// How many findings are judged per kind when the kind has enough of them.
pub const TARGET_JUDGED: usize = 30;

/// How many of those a second judge re-judges, blind.
pub const SECOND_JUDGE_COUNT: usize = 10;

/// SplitMix64: a 64-bit state advanced by [`GAMMA`] and scrambled by two xor-shift-multiply rounds.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose first output is a function of `seed` alone.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`, without modulo bias: draws below `2^64 mod bound` are rejected and redrawn.
    ///
    /// # Panics
    /// If `bound` is zero.
    pub fn bounded(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "bound must be positive");
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u64();
            if r >= threshold {
                return r % bound;
            }
        }
    }
}

/// Fisher-Yates, from the back: for `i` from `len - 1` down to 1, swap position `i` with a position drawn from `0..=i`.
pub fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut rng = SplitMix64::new(seed);
    for i in (1..items.len()).rev() {
        let j = usize::try_from(rng.bounded(i as u64 + 1)).expect("index fits usize");
        items.swap(i, j);
    }
}

/// Where a finding lives; the derived order is the spec's sort key `(project, file, line)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingRef {
    pub project: String,
    pub file: String,
    pub line: u32,
}

/// The order in which a kind's findings are judged: sorted by `(project, file, line)`, then shuffled with `seed`.
/// The sort comes first so the result does not depend on the order the lens happened to emit findings in.
#[must_use]
pub fn judging_order(mut findings: Vec<FindingRef>, seed: u64) -> Vec<FindingRef> {
    findings.sort();
    shuffle(&mut findings, seed);
    findings
}

/// How a kind is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Fewer than [`TARGET_JUDGED`] findings: all of them are read, and the "30 sampled" clause is not satisfied.
    Census { findings: usize },
    /// Judge in shuffled order until [`TARGET_JUDGED`] are judged or the pass line becomes unreachable.
    Sample,
}

/// Census or sample, by population.
#[must_use]
pub const fn reading(findings: usize) -> Reading {
    if findings < TARGET_JUDGED {
        Reading::Census { findings }
    } else {
        Reading::Sample
    }
}

/// The false-positive count at which a pass line of `pass_percent` (e.g. 80) is out of reach over 30 judged:
/// `floor(30 * (1 - p)) + 1`. Integer arithmetic on purpose: `30.0 * (1.0 - 0.8)` is 5.999..., which floors to 5.
///
/// # Panics
/// If `pass_percent` is over 100.
#[must_use]
pub const fn false_positive_stop(pass_percent: u32) -> usize {
    assert!(pass_percent <= 100, "a percentage");
    (TARGET_JUDGED * (100 - pass_percent as usize)) / 100 + 1
}

/// Why judging stopped, or that it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Continue,
    /// 30 judged without reaching the false-positive count.
    ThirtyJudged,
    /// The false-positive count was reached: the pass line cannot be met whatever the rest say.
    PassLineUnreachable,
    /// A census: every finding has been read.
    CensusComplete,
}

/// Where a kind's judging stands. `false_count` includes UNJUDGEABLE findings, which count FALSE (a lower bound).
/// Unreachability is checked first: the 7th false positive on the 7th judged stops for that reason, not for "30 judged".
#[must_use]
pub fn stop_state(findings: usize, judged: usize, false_count: usize, pass_percent: u32) -> Stop {
    if false_count >= false_positive_stop(pass_percent) {
        Stop::PassLineUnreachable
    } else if judged >= findings {
        match reading(findings) {
            Reading::Census { .. } => Stop::CensusComplete,
            Reading::Sample => Stop::ThirtyJudged,
        }
    } else if judged >= TARGET_JUDGED {
        Stop::ThirtyJudged
    } else {
        Stop::Continue
    }
}

/// Which of `judged` items (0-based positions in judging order) a second judge re-judges blind: [`SECOND_JUDGE_COUNT`] of them,
/// or all when fewer, chosen by a shuffle seeded with `seed ^ SECOND_JUDGE_SALT`, returned in ascending order.
#[must_use]
pub fn second_judge_selection(judged: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..judged).collect();
    shuffle(&mut idx, seed ^ SECOND_JUDGE_SALT);
    idx.truncate(SECOND_JUDGE_COUNT);
    idx.sort_unstable();
    idx
}

/// One-sided 95% upper bound on the true rate when none of `n` judged findings is true: the rule of three, `3 / n`
/// (CodeRipper's "about 7% (3/42)"). `None` for `n == 0`.
#[must_use]
pub fn rule_of_three_upper_bound(n: usize) -> Option<f64> {
    (n > 0).then(|| 3.0 / n as f64)
}

/// How many of two judges' verdicts agree, over the items both judged. The threshold below which the figure is void is
/// not set (it is CireSnave's, D1); this reports the number.
///
/// # Panics
/// If the slices differ in length.
#[must_use]
pub fn agreement(first: &[bool], second: &[bool]) -> (usize, usize) {
    assert_eq!(first.len(), second.len(), "same items");
    (
        first.iter().zip(second).filter(|(a, b)| a == b).count(),
        first.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values: Vigna's splitmix64.c with the first state 0 gives 0xE220A8397B1DCDAF first; seed 1234567 gives
    // 6457827717110365317, 3203168211198807973, ... (the vectors published in the rand_xoshiro crate's tests). Both agree with an
    // independent implementation of the algorithm in Python, run when this module was written.
    #[test]
    fn splitmix64_seed_0() {
        let mut g = SplitMix64::new(0);
        let got: Vec<u64> = (0..5).map(|_| g.next_u64()).collect();
        assert_eq!(
            got,
            [
                16_294_208_416_658_607_535,
                7_960_286_522_194_355_700,
                487_617_019_471_545_679,
                17_909_611_376_780_542_444,
                1_961_750_202_426_094_747
            ]
        );
        assert_eq!(got[0], 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn splitmix64_seed_1234567() {
        let mut g = SplitMix64::new(1_234_567);
        let got: Vec<u64> = (0..5).map(|_| g.next_u64()).collect();
        assert_eq!(
            got,
            [
                6_457_827_717_110_365_317,
                3_203_168_211_198_807_973,
                9_817_491_932_198_370_423,
                4_593_380_528_125_082_431,
                16_408_922_859_458_223_821
            ]
        );
    }

    #[test]
    fn shuffle_fixed_vectors() {
        let mut a: Vec<u32> = (0..10).collect();
        shuffle(&mut a, 2026);
        assert_eq!(a, [5, 0, 9, 2, 4, 7, 3, 6, 8, 1]);
        let mut b: Vec<u32> = (0..10).collect();
        shuffle(&mut b, 0);
        assert_eq!(b, [6, 3, 2, 9, 8, 1, 4, 7, 0, 5]);
        let mut c: Vec<u32> = (0..30).collect();
        shuffle(&mut c, 2026);
        assert_eq!(&c[..10], [8, 25, 7, 12, 21, 5, 0, 4, 29, 17]);
    }

    #[test]
    fn a_different_seed_gives_a_different_order_and_the_same_members() {
        let mut a: Vec<u32> = (0..10).collect();
        let mut b: Vec<u32> = (0..10).collect();
        shuffle(&mut a, 2026);
        shuffle(&mut b, 2027);
        assert_eq!(b, [3, 2, 4, 8, 0, 5, 7, 1, 9, 6]);
        assert_ne!(a, b);
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
    }

    #[test]
    fn bounded_is_in_range_and_rejects_the_biased_zone() {
        let mut g = SplitMix64::new(7);
        for bound in [1_u64, 2, 3, 7, 1000, u64::MAX] {
            for _ in 0..200 {
                assert!(g.bounded(bound) < bound);
            }
        }
        // 2^64 mod 3 = 1: only a draw of 0 is rejected; 2^64 mod 2^63 + 1 rejects nearly half the space.
        assert_eq!(3_u64.wrapping_neg() % 3, 1);
    }

    fn fr(project: &str, file: &str, line: u32) -> FindingRef {
        FindingRef {
            project: project.into(),
            file: file.into(),
            line,
        }
    }

    #[test]
    fn judging_order_sorts_first_so_input_order_does_not_matter() {
        let a = vec![
            fr("kiss", "b.md", 3),
            fr("humboldt", "x.rs", 9),
            fr("kiss", "a.md", 40),
            fr("kiss", "a.md", 2),
        ];
        let mut b = a.clone();
        b.reverse();
        let oa = judging_order(a, 2026);
        assert_eq!(oa, judging_order(b, 2026));
        let mut sorted = oa.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            [
                fr("humboldt", "x.rs", 9),
                fr("kiss", "a.md", 2),
                fr("kiss", "a.md", 40),
                fr("kiss", "b.md", 3)
            ]
        );
        // The order is the shuffle of that sorted list: 4 items, seed 2026.
        let mut expect = sorted;
        shuffle(&mut expect, 2026);
        assert_eq!(oa, expect);
    }

    #[test]
    fn stop_rule_is_the_seventh_false_positive_at_80_percent() {
        assert_eq!(false_positive_stop(80), 7);
        assert_eq!(false_positive_stop(90), 4);
        assert_eq!(false_positive_stop(100), 1);
        assert_eq!(false_positive_stop(0), 31);
        // The float form is wrong: this is why the function takes integer percent.
        assert_eq!((30.0_f64 * (1.0 - 0.8)).floor() as usize + 1, 6);
    }

    #[test]
    fn census_below_thirty_and_sample_from_thirty() {
        assert_eq!(reading(29), Reading::Census { findings: 29 });
        assert_eq!(reading(0), Reading::Census { findings: 0 });
        assert_eq!(reading(30), Reading::Sample);
    }

    #[test]
    fn stop_states() {
        assert_eq!(stop_state(100, 10, 6, 80), Stop::Continue);
        assert_eq!(stop_state(100, 10, 7, 80), Stop::PassLineUnreachable);
        assert_eq!(stop_state(100, 29, 6, 80), Stop::Continue);
        assert_eq!(stop_state(100, 30, 6, 80), Stop::ThirtyJudged);
        assert_eq!(stop_state(100, 30, 7, 80), Stop::PassLineUnreachable);
        assert_eq!(stop_state(12, 11, 0, 80), Stop::Continue);
        assert_eq!(stop_state(12, 12, 3, 80), Stop::CensusComplete);
        assert_eq!(stop_state(12, 7, 7, 80), Stop::PassLineUnreachable);
    }

    #[test]
    fn second_judge_takes_ten_of_thirty_reproducibly() {
        assert_eq!(
            second_judge_selection(30, 2026),
            [0, 5, 6, 9, 16, 18, 19, 23, 25, 27]
        );
        assert_eq!(
            second_judge_selection(30, 2026),
            second_judge_selection(30, 2026)
        );
        assert_ne!(
            second_judge_selection(30, 2026),
            second_judge_selection(30, 2027)
        );
        assert_eq!(second_judge_selection(4, 2026).len(), 4);
        assert_eq!(second_judge_selection(0, 2026), Vec::<usize>::new());
    }

    #[test]
    fn rule_of_three_and_agreement() {
        assert!((rule_of_three_upper_bound(42).unwrap() - 3.0 / 42.0).abs() < 1e-12);
        assert_eq!(rule_of_three_upper_bound(0), None);
        assert_eq!(
            agreement(&[true, false, true, true], &[true, true, true, false]),
            (2, 4)
        );
    }
}
