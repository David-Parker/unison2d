//! Deterministic pseudo-random number generator (xorshift32).
//!
//! A simple, fast, no-dependency PRNG suitable for procedural content,
//! particle effects, and any gameplay that needs reproducible randomness.

/// Deterministic xorshift32 PRNG.
///
/// ```
/// use unison_math::Rng;
///
/// let mut rng = Rng::new(42);
/// let x = rng.range_f32(0.0, 1.0);
/// let n = rng.range_u32(1, 7); // 1..6 inclusive
/// ```
pub struct Rng(u32);

impl Rng {
    /// Create a new RNG with the given seed.
    /// If zero is passed, it is replaced with 1 (xorshift has a fixed point at 0).
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    /// Advance the state and return the next raw u32 value.
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Random f32 in [lo, hi).
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let t = (self.next() % 10000) as f32 / 10000.0;
        lo + t * (hi - lo)
    }

    /// Random u32 in [lo, hi) (exclusive upper bound).
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next() % (hi - lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next(), b.next());
        }
    }

    #[test]
    fn range_f32_in_bounds() {
        let mut rng = Rng::new(12345);
        for _ in 0..1000 {
            let v = rng.range_f32(2.0, 5.0);
            assert!(v >= 2.0 && v < 5.0, "out of bounds: {}", v);
        }
    }

    #[test]
    fn range_u32_in_bounds() {
        let mut rng = Rng::new(12345);
        for _ in 0..1000 {
            let v = rng.range_u32(10, 20);
            assert!(v >= 10 && v < 20, "out of bounds: {}", v);
        }
    }

    #[test]
    fn zero_seed_does_not_stick() {
        let mut rng = Rng::new(0);
        let first = rng.next();
        let second = rng.next();
        assert_ne!(first, 0);
        assert_ne!(first, second);
    }
}
