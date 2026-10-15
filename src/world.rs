//! The endless landscape: plans which elements exist where, and draws them.
//!
//! STUB: placeholder hills so the painter and daemon can be exercised. The
//! real port of `mountplanner`/`chunkloader`/`water` replaces this file but
//! keeps the public API: `BAND_HEIGHT`, `Element`, `World::{new, plan, draw}`.

use crate::brush::{StrokeArgs, stroke};
use crate::geom::{Canv, rgba};
use crate::noise::Noise;
use crate::rng::{Cx, hash};

/// Visible height of the painting in world units (the original's viewBox:
/// 800 / zoom 1.142). Each output scales this band to its pixel height.
pub const BAND_HEIGHT: f64 = 800.0 / 1.142;

/// One planned thing in the world (a mountain, a boat, ...).
#[derive(Clone, Debug)]
pub struct Element {
    pub x: f64,
    pub y: f64,
    pub seed: u64,
}

pub struct World {
    pub seed: u64,
    pub noise: Noise,
}

impl World {
    pub fn new(seed: u64) -> Self {
        World { seed, noise: Noise::new(seed) }
    }

    /// Every element that may draw inside world x range `[x0, x1)`, in paint
    /// order. Must be deterministic: the same element always comes back with
    /// the same seed and the same relative order, whatever the range.
    pub fn plan(&self, x0: f64, x1: f64) -> Vec<Element> {
        let step = 200.0;
        let first = ((x0 - 400.0) / step).floor() as i64;
        let last = ((x1 + 400.0) / step).ceil() as i64;
        (first..=last)
            .map(|k| Element {
                x: k as f64 * step,
                y: 300.0 + (k.rem_euclid(5) as f64) * 60.0,
                seed: hash(&[self.seed, k as u64]),
            })
            .collect()
    }

    /// The element's shapes, in world coordinates.
    pub fn draw(&self, e: &Element) -> Canv {
        let mut cx = Cx::new(&self.noise, e.seed);
        let mut canv = Vec::new();
        for layer in 0..8 {
            let pts: Vec<_> = (0..=40)
                .map(|i| {
                    let u = i as f64 / 40.0;
                    let x = e.x - 200.0 + u * 400.0;
                    let y = e.y - (u * std::f64::consts::PI).sin() * (120.0 - layer as f64 * 12.0)
                        + layer as f64 * 6.0;
                    [x, y]
                })
                .collect();
            canv.extend(stroke(
                &mut cx,
                &pts,
                StrokeArgs { col: rgba(100.0, 100.0, 100.0, 0.25), wid: 4.0, ..Default::default() },
            ));
        }
        canv
    }
}
