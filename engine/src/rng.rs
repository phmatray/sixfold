//! A tiny seedable PRNG (PCG32, O'Neill). Deterministic: same seed, same game.

#[derive(Debug, Clone)]
pub(crate) struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        xorshifted.rotate_right((old >> 59) as u32)
    }

    /// Uniform in `0..n` (multiply-shift; the bias is negligible for card counts).
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }

    /// R-EFFECT-03: 50/50.
    pub fn coin(&mut self) -> bool {
        self.next_u32() >> 31 == 1
    }

    /// Fisher–Yates (R-ZONE-03).
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        assert!((0..100).all(|_| a.next_u32() == b.next_u32()));
        let heads = (0..1000).filter(|_| a.coin()).count();
        assert!((400..600).contains(&heads));
    }
}
