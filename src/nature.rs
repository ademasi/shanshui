//! Nature that is not in the original: mist banks, birds, the moon, reeds
//! along the shore and waterfalls. Same conventions as the ported modules:
//! world coordinates in, a `Canv` in paint order out, randomness from `cx`.

use crate::brush::{BlobArgs, StrokeArgs, blob, stroke};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, haze, poly, rgba};
use crate::rng::Cx;
use std::f64::consts::PI;

fn grey(a: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, a)
}

fn mist_white(a: f64) -> Rgba {
    rgba(255.0, 255.0, 255.0, a)
}

/// Farthest a mist haze reaches beyond the bank's `[x, x + len]`.
pub const MIST_REACH: f64 = 270.0;

/// A bank of mist lying on the ground or water at height `y` along
/// `[x, x + len]`, thinning upwards over about `thick` units and towards both
/// ends, so what stands in it rises out of the fog: overlapping soft white
/// ellipses, plus a few faint long wisps above it.
pub fn mist(cx: &mut Cx, x: f64, y: f64, len: f64, thick: f64) -> Canv {
    let mut canv = Vec::new();
    let n = (len / 70.0).ceil().max(3.0) as usize;
    for k in 0..n {
        let t = (k as f64 + cx.random()) / n as f64;
        let fade = (PI * t).sin().sqrt();
        let rx = 120.0 + cx.random() * 140.0;
        let ry = thick * (0.7 + cx.random() * 0.5) * fade.max(0.4);
        let cy = y + (cx.random() - 0.3) * thick * 0.3;
        let a = (0.65 + cx.random() * 0.25) * fade;
        canv.push(haze([x + t * len, cy], [rx, ry], mist_white(a)));
    }
    let wisps = (len / 300.0).ceil() as usize;
    for _ in 0..wisps {
        let t = 0.2 + cx.random() * 0.6;
        let rx = 120.0 + cx.random() * 120.0;
        let ry = 6.0 + cx.random() * 8.0;
        let cy = y - thick * (0.9 + cx.random() * 0.8);
        canv.push(haze([x + t * len, cy], [rx, ry], mist_white(0.35 + cx.random() * 0.2)));
    }
    canv
}

/// Farthest a flock reaches from its anchor.
pub const BIRDS_REACH: f64 = 260.0;

pub struct BirdsArgs {
    pub n: usize,
    /// geese in a V, else a loose flock
    pub vee: bool,
    /// flying left
    pub fli: bool,
    pub sca: f64,
}

/// One bird, two tapered wing strokes meeting at the body `(x, y)`. `up` is
/// the wing beat, -0.2 (wings low) to 1 (raised, gull-like).
fn bird(cx: &mut Cx, x: f64, y: f64, s: f64, up: f64, col: Rgba) -> Canv {
    let mut canv = Vec::new();
    let fun = |t: f64| (PI * t).sin().powf(0.7);
    for side in [-1.0, 1.0] {
        let tip = [x + side * s, y - s * (0.25 * up - 0.3 * (1.0 - up))];
        let elbow = [x + side * s * 0.45, y - s * (0.15 + 0.4 * up)];
        // quadratic Bezier from the tip, through the elbow, to the body
        let pts: Vec<Pt> = (0..=6)
            .map(|k| {
                let t = k as f64 / 6.0;
                let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t);
                [a * tip[0] + b * elbow[0] + c * x, a * tip[1] + b * elbow[1] + c * y]
            })
            .collect();
        canv.extend(stroke(cx, &pts, StrokeArgs { wid: s * 0.16, col, noi: 0.3, fun: Some(&fun), ..Default::default() }));
    }
    canv
}

/// A flock of birds around `(xoff, yoff)`.
pub fn birds(cx: &mut Cx, xoff: f64, yoff: f64, a: BirdsArgs) -> Canv {
    let dir = if a.fli { -1.0 } else { 1.0 };
    let sp = 15.0 * a.sca;
    let mut canv = Vec::new();
    for k in 0..a.n {
        let (dx, dy) = if a.vee {
            // leader in front, two arms trailing back
            let rank = k.div_ceil(2) as f64;
            let side = if k % 2 == 1 { 1.0 } else { -1.0 };
            (-dir * rank * sp * (0.9 + cx.random() * 0.2), side * rank * sp * 0.5 + (cx.random() - 0.5) * 3.0)
        } else {
            let spread = (a.n as f64).sqrt();
            (cx.norm_rand(-1.0, 1.0) * sp * spread * 1.4, cx.norm_rand(-1.0, 1.0) * sp * spread * 0.5)
        };
        let s = (5.0 + cx.random() * 3.0) * a.sca;
        let up = cx.random() * 1.2 - 0.2;
        let col = grey(0.55 + cx.random() * 0.3);
        canv.extend(bird(cx, xoff + dx, yoff + dy, s, up, col));
    }
    canv
}

/// How far the moon's halo reaches, in moon radii.
pub const MOON_HALO: f64 = 2.8;

/// The moon, as ink painters leave it: bare paper inside a faint grey wash,
/// with a light broken rim.
pub fn moon(cx: &mut Cx, x: f64, y: f64, r: f64) -> Canv {
    let mut canv = vec![haze([x, y], [r * MOON_HALO, r * MOON_HALO], grey(0.13))];
    let circle = |rr: f64, a0: f64, a1: f64, n: usize| -> Vec<Pt> {
        (0..=n)
            .map(|k| {
                let a = a0 + (a1 - a0) * k as f64 / n as f64;
                [x + rr * a.cos(), y + rr * a.sin()]
            })
            .collect()
    };
    canv.extend(poly(
        &circle(r, 0.0, 2.0 * PI, 64),
        PolyArgs { fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() },
    ));
    let mut a0 = cx.random() * 2.0 * PI;
    for _ in 0..3 {
        let span = 1.2 + cx.random() * 0.9;
        let rim = circle(r, a0, a0 + span, 20);
        let col = grey(0.18 + cx.random() * 0.12);
        canv.extend(stroke(cx, &rim, StrokeArgs { wid: 0.9, col, noi: 0.4, ..Default::default() }));
        a0 += span + 0.3 + cx.random() * 0.4;
    }
    canv
}

/// A clump of reeds standing at `(x, y)`, blades up to `40 * sca` high.
pub fn reeds(cx: &mut Cx, x: f64, y: f64, sca: f64) -> Canv {
    let mut canv = Vec::new();
    let n = 5 + (cx.random() * 9.0) as usize;
    let spread = (8.0 + cx.random() * 16.0) * sca;
    let wind = (cx.random() - 0.5) * 0.5;
    let fun = |t: f64| (1.0 - t).powf(0.5) * (t * 6.0).min(1.0);
    for _ in 0..n {
        let bx = x + cx.norm_rand(-1.0, 1.0) * spread;
        let by = y + cx.random() * 3.0 * sca;
        let hgt = (14.0 + cx.random() * 26.0) * sca;
        let lean = wind + (cx.random() - 0.5) * 0.5;
        let pts: Vec<Pt> = (0..=8)
            .map(|k| {
                let t = k as f64 / 8.0;
                [bx + lean * hgt * t * t, by - hgt * t]
            })
            .collect();
        let col = grey(0.3 + cx.random() * 0.3);
        let top = pts[8];
        let wid = (0.8 + cx.random() * 0.6) * sca;
        canv.extend(stroke(cx, &pts, StrokeArgs { wid, col, noi: 0.3, fun: Some(&fun), ..Default::default() }));
        if cx.random() < 0.3 {
            // a reed-mace head just below the tip
            let ang = -PI / 2.0 + (lean * 2.0).atan() * 0.5;
            let hx = top[0] - lean * hgt * 0.15;
            let hy = top[1] + hgt * 0.12;
            let col = grey(0.5 + cx.random() * 0.2);
            canv.extend(blob(
                cx,
                hx,
                hy,
                BlobArgs { len: 6.0 * sca, wid: 1.6 * sca, ang, col, noi: 0.2, ..Default::default() },
            ));
        }
    }
    canv
}

/// 0 to 3 clumps of reeds along a shore from `x - half` to `x + half` at
/// height `y`.
pub fn shore_reeds(cx: &mut Cx, x: f64, y: f64, half: f64, sca: f64) -> Canv {
    let mut canv = Vec::new();
    let n = cx.rand_choice(&[0, 0, 1, 1, 2, 3]);
    for _ in 0..n {
        let rx = x + cx.norm_rand(-0.9, 0.9) * half;
        let ry = y + cx.random() * 8.0;
        canv.extend(reeds(cx, rx, ry, sca));
    }
    canv
}

/// A waterfall from `(x, y0)` down to `y1`, about `wid` wide at the bottom,
/// in one drop or two (the second offset below a rock ledge). As ink
/// painters do it: bare paper for the water, the rock darkened on both
/// sides, broken edge lines, faint streaks, and spray where it lands.
pub fn waterfall(cx: &mut Cx, x: f64, y0: f64, y1: f64, wid: f64) -> Canv {
    if y1 - y0 < 120.0 || cx.random() < 0.5 {
        return fall(cx, x, y0, y1, wid, true);
    }
    let ym = y0 + (y1 - y0) * (0.35 + cx.random() * 0.25);
    let dx = (cx.random() - 0.5) * wid * 1.5;
    let mut canv = fall(cx, x, y0, ym, wid * 0.75, false);
    canv.extend(fall(cx, x + dx, ym + 3.0, y1, wid, true));
    canv
}

fn fall(cx: &mut Cx, x: f64, y0: f64, y1: f64, wid: f64, spray: bool) -> Canv {
    let len = y1 - y0;
    let n = ((len / 5.0).ceil() as usize).max(4);
    let s = cx.random() * 100.0;
    let mut mid = Vec::with_capacity(n + 1);
    let mut left = Vec::with_capacity(n + 1);
    let mut right = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let t = k as f64 / n as f64;
        let y = y0 + t * len;
        let c = x + (cx.noise(t * 2.0, s, 0.0) - 0.5) * wid * 0.8;
        let w = wid * (0.6 + 0.4 * t.powf(0.7)) / 2.0;
        mid.push([c, y]);
        left.push([c - w * (0.8 + 0.4 * cx.noise(t * 9.0, s + 1.0, 0.0)), y]);
        right.push([c + w * (0.8 + 0.4 * cx.noise(t * 9.0, s + 2.0, 0.0)), y]);
    }
    let mut canv = Vec::new();

    // the rock on either side, darkened by a soft wash that the water's
    // white then cuts cleanly
    for (edge, side) in [(&left, -1.0), (&right, 1.0)] {
        for (k, p) in edge.iter().enumerate() {
            let t = k as f64 / n as f64;
            let dark = cx.noise(t * 4.0, s + side, 1.0);
            let a = 0.25 * dark * (PI * t).sin().powf(0.3);
            let r = [wid * (0.6 + 0.5 * dark), 7.0];
            canv.push(haze([p[0] + side * wid * 0.15, p[1]], r, grey(a)));
        }
    }

    let mut outline = left.clone();
    outline.extend(right.iter().rev());
    canv.extend(poly(&outline, PolyArgs { fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() }));

    // edges in broken runs, like the rock contours around them
    for edge in [&left, &right] {
        let mut k = 0;
        while k + 2 < edge.len() {
            let run = n / 4 + 3 + (cx.random() * (n as f64 / 2.0)) as usize;
            let end = (k + run).min(edge.len());
            if cx.random() < 0.85 {
                let col = grey(0.45 + cx.random() * 0.25);
                canv.extend(stroke(cx, &edge[k..end], StrokeArgs { wid: 1.4, col, noi: 0.5, ..Default::default() }));
            }
            k = end + (cx.random() * 3.0) as usize;
        }
    }
    // streaks of falling water
    for _ in 0..3 + (cx.random() * 4.0) as usize {
        let u = cx.random() - 0.5;
        let a = (cx.random() * 0.5 * n as f64) as usize;
        let b = (a + 3 + (cx.random() * 0.6 * n as f64) as usize).min(n);
        let streak: Vec<Pt> = (a..=b)
            .map(|k| {
                let w = right[k][0] - left[k][0];
                [mid[k][0] + u * w * 0.7, mid[k][1]]
            })
            .collect();
        let col = grey(0.15 + cx.random() * 0.15);
        canv.extend(stroke(cx, &streak, StrokeArgs { wid: 0.6, col, noi: 0.3, ..Default::default() }));
    }
    // the lip it pours over
    let lip: Vec<Pt> = (0..=8)
        .map(|k| {
            let t = k as f64 / 8.0 - 0.5;
            [mid[0][0] + t * wid * 2.2, y0 + 1.5 - 3.0 * (1.0 - 4.0 * t * t) + (cx.random() - 0.5)]
        })
        .collect();
    let col = grey(0.45 + cx.random() * 0.2);
    canv.extend(stroke(cx, &lip, StrokeArgs { wid: 1.8, col, noi: 0.5, ..Default::default() }));
    if spray {
        let foot = mid[n];
        canv.push(haze([foot[0], foot[1] - 2.0], [wid * 3.5, wid * 1.3], mist_white(0.95)));
    }
    canv
}
