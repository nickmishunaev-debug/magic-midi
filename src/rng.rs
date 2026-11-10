//! A tiny, dependency-free random number generator (xorshift64*).

#[derive(Clone, Copy)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
        r.next_u64();
        r.next_u64();
        r
    }

    /// Seeded from the system clock, so every instance starts differently.
    pub fn from_clock() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self::new(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in [0, 1)
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }

    /// True with probability `p`
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }

    /// Uniform integer in [0, n). `n` must be > 0.
    pub fn range(&mut self, n: usize) -> usize {
        ((self.next_u64() >> 11) % n as u64) as usize
    }
}
