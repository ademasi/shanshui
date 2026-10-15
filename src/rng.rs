//! Seeded randomness. Replaces the original's global `Math.random()` stream:
//! every element gets its own `Cx` seeded from (world seed, chunk, index), so
//! any stretch of landscape can be regenerated identically.

use crate::noise::Noise;

/// SplitMix64 finaliser, used to derive independent seeds.
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Combine several values into one seed.
pub fn hash(parts: &[u64]) -> u64 {
    parts.iter().fold(0x5348_414E_5348_5549, |acc, &p| mix(acc ^ mix(p)))
}

/// xoshiro256** generator.
#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut s = [0u64; 4];
        for v in &mut s {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            *v = mix(z);
        }
        Rng { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform in [0, 1), like `Math.random()`.
    pub fn random(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Drawing context handed to every generator function: the element's own
/// random stream plus the world's shared (pure) Perlin noise.
pub struct Cx<'a> {
    pub rng: Rng,
    pub nz: &'a Noise,
}

impl<'a> Cx<'a> {
    pub fn new(nz: &'a Noise, seed: u64) -> Self {
        Cx { rng: Rng::new(seed), nz }
    }

    /// `Math.random()`
    pub fn random(&mut self) -> f64 {
        self.rng.random()
    }

    /// `Noise.noise(x, y, z)`; pass 0.0 for omitted arguments.
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        self.nz.noise(x, y, z)
    }

    /// `normRand(m, M)`
    pub fn norm_rand(&mut self, m: f64, big_m: f64) -> f64 {
        mapval(self.random(), 0.0, 1.0, m, big_m)
    }

    /// `randChoice(arr)`
    pub fn rand_choice<T: Copy>(&mut self, arr: &[T]) -> T {
        let i = (arr.len() as f64 * self.random()).floor() as usize;
        arr[i.min(arr.len() - 1)]
    }

    /// `wtrand(func)`: rejection sampling, iterative instead of recursive.
    pub fn wtrand(&mut self, func: impl Fn(f64) -> f64) -> f64 {
        loop {
            let x = self.random();
            let y = self.random();
            if y < func(x) {
                return x;
            }
        }
    }

    /// `randGaussian()`
    pub fn rand_gaussian(&mut self) -> f64 {
        self.wtrand(|x| (-24.0 * (x - 0.5).powi(2)).exp()) * 2.0 - 1.0
    }
}

/// `mapval(value, istart, istop, ostart, ostop)`
pub fn mapval(value: f64, istart: f64, istop: f64, ostart: f64, ostop: f64) -> f64 {
    ostart + (ostop - ostart) * ((value - istart) / (istop - istart))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let x = a.random();
            assert_eq!(x, b.random());
            assert!((0.0..1.0).contains(&x));
        }
        assert_ne!(Rng::new(1).random(), Rng::new(2).random());
    }
}
