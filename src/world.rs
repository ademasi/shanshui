//! The endless landscape: plans which elements exist where, and draws them.
//!
//! Port of `mountplanner`, `chunkloader`/`chunkrender` and `water` from the
//! original, made stateless. The world is cut into chunks of `CWID` units;
//! chunk `c` covers `[CWID*c, CWID*(c+1))` and is planned from its own RNG
//! (seeded from the world seed and `c`), so any stretch of the world can be
//! regenerated identically whatever range is asked for and in what order.
//!
//! The planner runs in phases on one stream per chunk, as the JS does:
//! mountains (+ the odd distant range), flat mountains where no mountain is
//! near, boats, then the per-element arguments `chunkloader` draws. The
//! original's global `MEM.planmtx` coverage is recomputed from the mountain
//! phase of the neighbouring chunks (a prefix of their stream).

use crate::arch::{Boat01Args, boat01};
use crate::brush::{StrokeArgs, stroke};
use crate::geom::{Canv, Pt, rgba};
use crate::mount::{DistMountArgs, FlatMountArgs, MountainArgs, dist_mount, flat_mount, mountain};
use crate::noise::Noise;
use crate::rng::{Cx, Rng, hash};
use std::cmp::Ordering;
use std::f64::consts::PI;

/// Visible height of the painting in world units (the original's viewBox:
/// 800 / zoom 1.142). Each output scales this band to its pixel height.
pub const BAND_HEIGHT: f64 = 800.0 / 1.142;

/// Chunk width (`MEM.cwid`).
const CWID: f64 = 512.0;
/// Planner sampling step (`xstep`).
const XSTEP: f64 = 5.0;
/// Half-width of the flat-mountain exclusion around a mountain (`mwid`).
const MWID: f64 = 200.0;
/// Farthest an element's drawing reaches beyond its own chunk: flatMount is
/// jittered up to 700 from its sample point and draws up to ~900 either side
/// (half-width 500, ground wobble, plateau trees and buildings); distMount
/// starts at its sample point and runs right up to 1500.
const CHUNK_REACH: f64 = 1600.0;
/// Mountains are jittered up to this far from their sample point.
const MOUNT_JITTER: f64 = 500.0;
/// Chunks whose mountains can cover a given chunk: jitter + `MWID` < 2 chunks.
const COVER_CHUNKS: i64 = ((MOUNT_JITTER + MWID) / CWID) as i64 + 1;

const TAG_PLAN: u64 = 0x504C_414E;
const TAG_ELEM: u64 = 0x454C_454D;

/// What an element is, with the arguments `chunkloader` drew for it.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Mount { nseed: f64 },
    Water,
    FlatMount { nseed: f64, wid: f64, cho: f64 },
    DistMount { nseed: f64, len: f64 },
    Boat { nseed: f64, fli: bool },
}

/// One planned thing in the world (a mountain, a boat, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: Kind,
    /// anchor, in world units (as the original's `xoff`, `yoff`)
    pub x: f64,
    pub y: f64,
    /// paint-order key: the original's `y`, `y - 10000` for water
    pub depth: f64,
    /// chunk, index in the chunk's plan, sub-element (mountain 0, its water 1)
    pub chunk: i64,
    pub index: u32,
    pub sub: u32,
    /// seed of the element's own random stream
    pub seed: u64,
}

impl Element {
    /// Conservative horizontal extent of what `draw` can paint.
    pub fn extent(&self) -> (f64, f64) {
        let x = self.x;
        match self.kind {
            // half-width <= 300, texture noise, edge trees, rocks, buildings
            Kind::Mount { .. } => (x - 650.0, x + 650.0),
            // xk <= 50, lk <= 400
            Kind::Water => (x - 500.0, x + 500.0),
            Kind::FlatMount { .. } => (x - 900.0, x + 900.0),
            Kind::DistMount { len, .. } => (x - 20.0, x + len + 20.0),
            Kind::Boat { .. } => (x - 200.0, x + 200.0),
        }
    }

    fn order(&self, o: &Element) -> Ordering {
        self.depth
            .total_cmp(&o.depth)
            .then(self.x.total_cmp(&o.x))
            .then(self.chunk.cmp(&o.chunk))
            .then(self.index.cmp(&o.index))
            .then(self.sub.cmp(&o.sub))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tag {
    Mount,
    DistMount,
    FlatMount,
    Boat,
}

#[derive(Clone, Copy, Debug)]
struct Reg {
    tag: Tag,
    x: f64,
    y: f64,
}

/// `chadd(r, mind)`: keep `r` unless something already planned in this chunk
/// is within `mind` horizontally.
fn chadd(reg: &mut Vec<Reg>, r: Reg, mind: f64) -> bool {
    if reg.iter().any(|q| (q.x - r.x).abs() < mind) {
        return false;
    }
    reg.push(r);
    true
}

/// Sample points `xmin, xmin + 5, ...` below `xmax`, as the JS loops.
fn samples(c: i64) -> impl Iterator<Item = f64> {
    let xmin = c as f64 * CWID;
    let xmax = xmin + CWID;
    (0..).map(move |k| xmin + k as f64 * XSTEP).take_while(move |&i| i < xmax)
}

pub struct World {
    pub seed: u64,
    pub noise: Noise,
}

impl World {
    pub fn new(seed: u64) -> Self {
        World { seed, noise: Noise::new(seed) }
    }

    fn ns(&self, x: f64) -> f64 {
        (self.noise.noise(x * 0.03, 0.0, 0.0) - 0.55).max(0.0) * 2.0
    }

    /// `locmax(x, y, ns, 2)`; `ns` ignores y, so only x matters.
    fn locmax(&self, x: f64) -> bool {
        let z0 = self.ns(x);
        if z0 <= 0.3 {
            return false;
        }
        (-2..2).all(|d| self.ns(x + d as f64) <= z0)
    }

    /// Mountain phase of chunk `c`: mountains and distant ranges, in plan
    /// order. Consumes the start of the chunk's stream.
    fn mount_phase(&self, c: i64, rng: &mut Rng) -> Vec<Reg> {
        let mut reg = Vec::new();
        for i in samples(c) {
            let yr = self.noise.noise(i * 0.01, PI, 0.0);
            let lm = self.locmax(i);
            let mut j = 0.0;
            while j < yr * 480.0 {
                if lm {
                    let x = i + 2.0 * (rng.random() - 0.5) * MOUNT_JITTER;
                    chadd(&mut reg, Reg { tag: Tag::Mount, x, y: j + 300.0 }, 10.0);
                }
                j += 30.0;
            }
            if i.abs() % 1000.0 < (XSTEP - 1.0).max(1.0) {
                let y = 280.0 - rng.random() * 50.0;
                chadd(&mut reg, Reg { tag: Tag::DistMount, x: i, y }, 10.0);
            }
        }
        reg
    }

    fn mount_xs(&self, c: i64) -> Vec<f64> {
        let mut rng = Rng::new(hash(&[self.seed, c as u64, TAG_PLAN]));
        self.mount_phase(c, &mut rng)
            .into_iter()
            .filter(|r| r.tag == Tag::Mount)
            .map(|r| r.x)
            .collect()
    }

    /// The full plan of chunk `c`. `near` holds the mountain x positions of
    /// chunks `c - COVER_CHUNKS ..= c + COVER_CHUNKS` (for `planmtx`).
    fn plan_chunk(&self, c: i64, near: &[f64]) -> Vec<Element> {
        let mut rng = Rng::new(hash(&[self.seed, c as u64, TAG_PLAN]));
        let mut reg = self.mount_phase(c, &mut rng);

        // planmtx[k] for k = floor(i / xstep): number of mountains whose
        // [x - mwid, x + mwid) footprint covers that cell.
        let covered = |i: f64| {
            let k = (i / XSTEP).floor();
            near.iter().any(|&m| k >= ((m - MWID) / XSTEP).floor() && k < (m + MWID) / XSTEP)
        };
        for i in samples(c) {
            if covered(i) || rng.random() >= 0.01 {
                continue;
            }
            let mut j = 0.0;
            while j < 4.0 * rng.random() {
                let x = i + 2.0 * (rng.random() - 0.5) * 700.0;
                chadd(&mut reg, Reg { tag: Tag::FlatMount, x, y: 700.0 - j * 50.0 }, 10.0);
                j += 1.0;
            }
        }
        for i in samples(c) {
            if rng.random() < 0.2 {
                let y = 300.0 + rng.random() * 390.0;
                chadd(&mut reg, Reg { tag: Tag::Boat, x: i, y }, 400.0);
            }
        }

        // chunkloader: the arguments it draws, from the same stream.
        let mut out = Vec::with_capacity(reg.len() + 8);
        for (i, r) in reg.iter().enumerate() {
            let fi = i as f64;
            let el = |kind, depth, sub: u32| Element {
                kind,
                x: r.x,
                y: r.y,
                depth,
                chunk: c,
                index: i as u32,
                sub,
                seed: hash(&[self.seed, c as u64, i as u64, sub as u64, TAG_ELEM]),
            };
            match r.tag {
                Tag::Mount => {
                    let nseed = fi * 2.0 * rng.random();
                    out.push(el(Kind::Mount { nseed }, r.y, 0));
                    out.push(el(Kind::Water, r.y - 10000.0, 1));
                }
                Tag::FlatMount => {
                    let nseed = 2.0 * rng.random() * PI;
                    let wid = 600.0 + rng.random() * 400.0;
                    let cho = 0.5 + rng.random() * 0.2;
                    out.push(el(Kind::FlatMount { nseed, wid, cho }, r.y, 0));
                }
                Tag::DistMount => {
                    let nseed = rng.random() * 100.0;
                    let len = [500.0, 1000.0, 1500.0][(rng.random() * 3.0) as usize % 3];
                    out.push(el(Kind::DistMount { nseed, len }, r.y, 0));
                }
                Tag::Boat => {
                    let nseed = rng.random();
                    let fli = rng.random() < 0.5;
                    out.push(el(Kind::Boat { nseed, fli }, r.y, 0));
                }
            }
        }
        out
    }

    /// Every element that may draw inside world x range `[x0, x1)`, in paint
    /// order. Deterministic: the same element always comes back with the
    /// same seed and the same relative order, whatever the range.
    pub fn plan(&self, x0: f64, x1: f64) -> Vec<Element> {
        let c0 = ((x0 - CHUNK_REACH) / CWID).floor() as i64 - 1;
        let c1 = ((x1 + CHUNK_REACH) / CWID).floor() as i64;
        let mounts: Vec<Vec<f64>> =
            (c0 - COVER_CHUNKS..=c1 + COVER_CHUNKS).map(|c| self.mount_xs(c)).collect();
        let mut out = Vec::new();
        for c in c0..=c1 {
            let k = (c - c0) as usize;
            let near: Vec<f64> = mounts[k..=k + 2 * COVER_CHUNKS as usize].concat();
            out.extend(self.plan_chunk(c, &near).into_iter().filter(|e| {
                let (l, r) = e.extent();
                r > x0 && l < x1
            }));
        }
        out.sort_by(|a, b| a.order(b));
        out
    }

    /// The element's shapes, in world coordinates.
    pub fn draw(&self, e: &Element) -> Canv {
        let mut cx = Cx::new(&self.noise, e.seed);
        let cx = &mut cx;
        match e.kind {
            Kind::Mount { nseed } => mountain(cx, e.x, e.y, nseed, MountainArgs::default()),
            Kind::Water => water(cx, e.x, e.y, WaterArgs::default()),
            Kind::FlatMount { nseed, wid, cho } => flat_mount(
                cx,
                e.x,
                e.y,
                nseed,
                FlatMountArgs { wid: Some(wid), hei: Some(100.0), cho, ..Default::default() },
            ),
            Kind::DistMount { nseed, len } => {
                dist_mount(cx, e.x, e.y, nseed, DistMountArgs { hei: 150.0, len, ..Default::default() })
            }
            Kind::Boat { nseed, fli } => {
                boat01(cx, e.x, e.y, nseed, Boat01Args { sca: e.y / 800.0, fli, ..Default::default() })
            }
        }
    }
}

pub struct WaterArgs {
    pub hei: f64,
    pub len: f64,
    pub clu: usize,
}

impl Default for WaterArgs {
    fn default() -> Self {
        WaterArgs { hei: 2.0, len: 800.0, clu: 10 }
    }
}

/// `water(xoff, yoff, seed, args)`: ripple lines below a mountain. (The
/// original's `seed` argument is unused.)
pub fn water(cx: &mut Cx, xoff: f64, yoff: f64, a: WaterArgs) -> Canv {
    let (hei, len) = (a.hei, a.len);
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(a.clu);
    let mut yk = 0.0;
    for _ in 0..a.clu {
        let xk = (cx.random() - 0.5) * (len / 8.0);
        yk += cx.random() * 5.0;
        let lk = len / 4.0 + cx.random() * (len / 4.0);
        let mut row = Vec::new();
        let mut j = -lk;
        while j < lk {
            row.push([j + xk, (j * 0.2).sin() * hei * cx.noise(j * 0.1, 0.0, 0.0) - 20.0 + yk]);
            j += 5.0;
        }
        ptlist.push(row);
    }
    let mut canv = Vec::new();
    for row in ptlist.iter().skip(1) {
        let col = rgba(100.0, 100.0, 100.0, 0.3 + cx.random() * 0.3);
        let pts: Vec<Pt> = row.iter().map(|p| [p[0] + xoff, p[1] + yoff]).collect();
        canv.extend(stroke(cx, &pts, StrokeArgs { col, wid: 1.0, ..Default::default() }));
    }
    canv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{Palette, PaletteKind, PixelFormat, render_strip, scale_for_height};

    #[test]
    fn plan_is_reproducible() {
        let (a, b) = (World::new(11), World::new(11));
        for &(x0, x1) in &[(0.0, 1200.0), (-5000.0, -3000.0), (123456.0, 125000.0)] {
            let pa = a.plan(x0, x1);
            assert!(!pa.is_empty());
            assert_eq!(pa, b.plan(x0, x1));
        }
    }

    #[test]
    fn plan_order_is_range_independent() {
        let w = World::new(5);
        let wide = w.plan(-2000.0, 4000.0);
        let narrow = w.plan(500.0, 1500.0);
        let key = |e: &Element| (e.chunk, e.index, e.sub);
        let sub: Vec<_> =
            wide.iter().filter(|e| narrow.iter().any(|n| key(n) == key(e))).cloned().collect();
        assert_eq!(sub, narrow);
    }

    #[test]
    fn extents_cover_drawing() {
        // Drawn bounds must stay inside the planner's extents (trees and
        // buildings are not counted while their modules are stubs).
        let w = World::new(9);
        for e in w.plan(-20000.0, 20000.0) {
            let (l, r) = e.extent();
            for s in w.draw(&e) {
                for p in &s.pts {
                    assert!(p[0] >= l && p[0] <= r, "{:?} draws at {} outside {l}..{r}", e.kind, p[0]);
                }
            }
        }
    }

    fn strip(w: &World, x0_px: i64, scale: f64, width: u32, h: u32) -> Vec<u8> {
        let mut buf = vec![0u8; width as usize * h as usize * 4];
        let pal = Palette::new(PaletteKind::Paper);
        render_strip(w, x0_px, scale, width, h, &mut buf, PixelFormat::Rgba, &pal);
        buf
    }

    #[test]
    fn strips_are_seamless() {
        // A 600 px strip at P against the same columns of a 1400 px strip at
        // P - 400. The painter's leftmost few columns are skipped: tiny-skia's
        // anti-aliasing at the pixmap's left clip edge differs from the same
        // pixels drawn mid-strip (a paint.rs matter, not a planning one).
        // A handful of isolated pixels may differ by a step more than the
        // tolerance where f32 rounding flips a miter join.
        const EDGE: usize = 8;
        let world = World::new(3);
        let h = 360;
        let scale = scale_for_height(h);
        for &p in &[0i64, 1777, -2950, 40_000] {
            let a = strip(&world, p, scale, 600, h);
            let b = strip(&world, p - 400, scale, 1400, h);
            let (mut worst, mut over) = (0, 0);
            for y in 0..h as usize {
                for x in EDGE..600usize {
                    for ch in 0..3 {
                        let va = a[(y * 600 + x) * 4 + ch] as i32;
                        let vb = b[(y * 1400 + x + 400) * 4 + ch] as i32;
                        let d = (va - vb).abs();
                        worst = worst.max(d);
                        over += (d > 2) as usize;
                    }
                }
            }
            assert!(worst <= 4 && over <= 12, "strip at {p}: max diff {worst}, {over} values > 2");
        }
    }
}
