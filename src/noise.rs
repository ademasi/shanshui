//! Perlin noise, ported from the original, which embeds p5.js's `noise()`
//! (src/math/noise.js, LGPL-2.1; adapted there from Processing, toxi and
//! farbrausch). The table is filled once from the world seed, so `noise()` is
//! a pure function.

use crate::rng::Rng;

const YWRAPB: i64 = 4;
const YWRAP: i64 = 1 << YWRAPB;
const ZWRAPB: i64 = 8;
const ZWRAP: i64 = 1 << ZWRAPB;
const SIZE: i64 = 4095;
const OCTAVES: usize = 4;
const AMP_FALLOFF: f64 = 0.5;

pub struct Noise {
    perlin: Box<[f64; SIZE as usize + 1]>,
}

fn scaled_cosine(i: f64) -> f64 {
    0.5 * (1.0 - (i * std::f64::consts::PI).cos())
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ 0x6E6F_6973_6500_0000);
        let mut perlin = Box::new([0.0; SIZE as usize + 1]);
        for v in perlin.iter_mut() {
            *v = rng.random();
        }
        Noise { perlin }
    }

    #[inline]
    fn p(&self, i: i64) -> f64 {
        self.perlin[(i & SIZE) as usize]
    }

    /// `Noise.noise(x, y, z)`; result roughly in [0, 1].
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        let (x, y, z) = (x.abs(), y.abs(), z.abs());
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            return 0.0;
        }
        let (mut xi, mut yi, mut zi) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
        let (mut xf, mut yf, mut zf) = (x - xi as f64, y - yi as f64, z - zi as f64);
        let mut r = 0.0;
        let mut ampl = 0.5;
        for _ in 0..OCTAVES {
            let mut of = xi.wrapping_add(yi << YWRAPB).wrapping_add(zi << ZWRAPB);
            let rxf = scaled_cosine(xf);
            let ryf = scaled_cosine(yf);
            let mut n1 = self.p(of);
            n1 += rxf * (self.p(of + 1) - n1);
            let mut n2 = self.p(of + YWRAP);
            n2 += rxf * (self.p(of + YWRAP + 1) - n2);
            n1 += ryf * (n2 - n1);
            of += ZWRAP;
            n2 = self.p(of);
            n2 += rxf * (self.p(of + 1) - n2);
            let mut n3 = self.p(of + YWRAP);
            n3 += rxf * (self.p(of + YWRAP + 1) - n3);
            n2 += ryf * (n3 - n2);
            n1 += scaled_cosine(zf) * (n2 - n1);
            r += n1 * ampl;
            ampl *= AMP_FALLOFF;
            xi <<= 1;
            xf *= 2.0;
            yi <<= 1;
            yf *= 2.0;
            zi <<= 1;
            zf *= 2.0;
            if xf >= 1.0 {
                xi += 1;
                xf -= 1.0;
            }
            if yf >= 1.0 {
                yi += 1;
                yf -= 1.0;
            }
            if zf >= 1.0 {
                zi += 1;
                zf -= 1.0;
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_and_bounded() {
        let n = Noise::new(7);
        let m = Noise::new(7);
        for i in 0..500 {
            let x = i as f64 * 0.37;
            let v = n.noise(x, 1.5, 0.0);
            assert_eq!(v, m.noise(x, 1.5, 0.0));
            assert!((0.0..=1.0).contains(&v));
        }
        assert_eq!(n.noise(-3.2, 0.0, 0.0), n.noise(3.2, 0.0, 0.0));
    }
}
