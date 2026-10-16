//! Nature that is not in the original: mist banks, birds, the moon, reeds
//! along the shore, waterfalls, near shores along the bottom edge, and
//! people out in it (travellers, a scholar under a pine, anglers). Same conventions as the ported modules: world
//! coordinates in, a `Canv` in paint order out, randomness from `cx`.

use crate::brush::{BlobArgs, StrokeArgs, TextureArgs, blob, stroke, texture};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, haze, poly, rgba};
use crate::man::{Hat, Item, ManArgs, man};
use crate::mount::{RockArgs, rock};
use crate::rng::Cx;
use crate::tree::{
    Tree02Args, Tree04Args, Tree05Args, Tree06Args, Tree07Args, Tree08Args, tree02, tree04, tree05,
    tree06, tree07, tree08,
};
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

/// Figure scale for something standing at depth `y` (nearer is larger).
pub fn figure_sca(y: f64) -> f64 {
    0.45 * (y / 700.0).clamp(0.5, 1.1)
}

/// `man` joint angles from absolute directions (radians, 0 = forward,
/// pi/2 = down) of torso, head, thigh, shin, the arms and forearms.
fn pose(torso: f64, head: f64, legs: [f64; 2], front: [f64; 2], back: [f64; 2]) -> [f64; 9] {
    [
        0.0,
        torso,
        head - torso,
        legs[0],
        legs[1] - legs[0],
        front[0] - torso,
        front[1] - front[0],
        back[0] - torso,
        back[1] - back[0],
    ]
}

/// Where `man` puts the hand of the front arm, relative to the feet.
fn front_hand(a: &[f64; 9], len: &[f64; 9], sca: f64) -> Pt {
    let dir = |t: f64| [t.cos(), t.sin()];
    let (t1, t3, t4) = (a[1], a[3], a[3] + a[4]);
    let (t5, t6) = (a[1] + a[5], a[1] + a[5] + a[6]);
    let shoulder = dir(t1).map(|v| v * len[1]);
    let foot = [len[3] * dir(t3)[0] + len[4] * dir(t4)[0], len[3] * dir(t3)[1] + len[4] * dir(t4)[1]];
    let hand = [
        shoulder[0] + len[5] * dir(t5)[0] + len[6] * dir(t6)[0],
        shoulder[1] + len[5] * dir(t5)[1] + len[6] * dir(t6)[1],
    ];
    [(hand[0] - foot[0]) * sca, (hand[1] - foot[1]) * sca]
}

/// A thin, slightly bowed line from `a` to `b`: a staff or a fishing rod.
fn pole(cx: &mut Cx, a: Pt, b: Pt, bow: f64, col: Rgba, wid: f64) -> Canv {
    let s = cx.random() * 10.0;
    let pts: Vec<Pt> = (0..=10)
        .map(|k| {
            let t = k as f64 / 10.0;
            let (nx, ny) = (-(b[1] - a[1]), b[0] - a[0]);
            let off = bow * (PI * t).sin() + (cx.noise(t * 3.0, s, 0.0) - 0.5) * 0.6;
            let l = (nx * nx + ny * ny).sqrt().max(1e-9);
            [a[0] + (b[0] - a[0]) * t + nx / l * off, a[1] + (b[1] - a[1]) * t + ny / l * off]
        })
        .collect();
    poly(&pts, PolyArgs { str: Some(col), wid, ..Default::default() })
}

const FIG_LEN: [f64; 9] = [0.0, 30.0, 20.0, 24.0, 22.0, 20.0, 18.0, 20.0, 16.0];

/// A traveller walking with a staff, at `(x, y)` (feet), facing left if
/// `fli`; now and then followed by a servant boy.
pub fn traveller(cx: &mut Cx, x: f64, y: f64, sca: f64, fli: bool) -> Canv {
    let dir = if fli { -1.0 } else { 1.0 };
    let ang = pose(-PI / 2.0 + 0.08, -PI / 2.0 + 0.15, [PI / 2.0 - 0.3, PI / 2.0 + 0.1], [PI / 2.0 - 0.9, -0.25], [PI / 2.0 + 0.35, PI / 2.0 + 0.1]);
    let hand = front_hand(&ang, &FIG_LEN, sca);
    let hat = cx.rand_choice(&[Hat::Hat01, Hat::Hat02, Hat::None]);
    let mut canv = Vec::new();
    if cx.random() < 0.3 {
        // the servant, smaller, a few steps behind with the luggage pole
        let bx = x - dir * 28.0 * sca;
        let bang = pose(-PI / 2.0, -PI / 2.0 + 0.1, [PI / 2.0 - 0.2, PI / 2.0], [PI / 2.0 - 0.6, -0.6], [PI / 2.0 + 0.3, PI / 2.0]);
        canv.extend(man(cx, bx, y, ManArgs { sca: sca * 0.75, hat: Hat::None, ite: Item::Stick01, fli, ang: Some(bang), len: FIG_LEN }));
    }
    let top = [x + dir * hand[0], y + hand[1] - 9.0 * sca];
    let foot = [x + dir * (hand[0] + 4.0 * sca), y];
    canv.extend(pole(cx, top, foot, 0.8, grey(0.7), 1.3 * sca / 0.45));
    canv.extend(man(cx, x, y, ManArgs { sca, hat, ite: Item::None, fli, ang: Some(ang), len: FIG_LEN }));
    canv
}

/// A scholar sitting cross-legged at `(x, y)`, facing left if `fli`.
pub fn sitter(cx: &mut Cx, x: f64, y: f64, sca: f64, fli: bool) -> Canv {
    let ang = pose(-PI / 2.0 - 0.05, -PI / 2.0 + 0.2, [0.1, PI / 2.0 + 0.9], [PI / 2.0 - 0.4, 0.25], [PI / 2.0 + 0.2, 0.2]);
    let len = [0.0, 30.0, 20.0, 20.0, 9.0, 18.0, 16.0, 18.0, 14.0];
    let hat = cx.rand_choice(&[Hat::Hat01, Hat::None]);
    man(cx, x, y, ManArgs { sca, hat, ite: Item::None, fli, ang: Some(ang), len })
}

/// An angler sitting on a rock at the water's edge at `(x, y)`, his rod and
/// line out over the water on the left if `fli`.
pub fn angler(cx: &mut Cx, x: f64, y: f64, sca: f64, fli: bool) -> Canv {
    let dir = if fli { -1.0 } else { 1.0 };
    let k = sca / 0.45;
    let (rw, rh) = ((18.0 + cx.random() * 10.0) * k, (10.0 + cx.random() * 6.0) * k);
    let seed = cx.random() * 100.0;
    let mut canv = rock(cx, x, y, seed, RockArgs { wid: rw, hei: rh, tex: 20, sha: 2.0 });
    let sy = y - rh * 0.75;
    let ang = pose(-PI / 2.0 - 0.1, -PI / 2.0 + 0.3, [0.25, PI / 2.0 + 0.2], [PI / 2.0 - 0.7, -0.2], [PI / 2.0 + 0.3, 0.1]);
    let len = [0.0, 30.0, 20.0, 20.0, 18.0, 18.0, 16.0, 18.0, 14.0];
    let hand = front_hand(&ang, &len, sca);
    let hx = x + dir * hand[0];
    let hy = sy + hand[1];
    let rl = (45.0 + cx.random() * 20.0) * k;
    let tip = [hx + dir * rl * 0.85, hy - rl * 0.5];
    canv.extend(man(cx, x, sy, ManArgs { sca, hat: Hat::Hat02, ite: Item::None, fli, ang: Some(ang), len }));
    canv.extend(pole(cx, [hx - dir * 6.0 * k, hy + 3.0 * k], tip, -2.0 * k, grey(0.7), 1.1 * k));
    canv.extend(pole(cx, tip, [tip[0] + dir * 3.0 * k, y + 6.0 * k], 0.5, grey(0.35), 0.6 * k));
    canv
}

/// Rarely, someone out at the foot of a slope or on a shore spanning
/// `x - half .. x + half` at height `y`: an angler (facing the water at the
/// nearer end) or a traveller on the path.
pub fn shore_figure(cx: &mut Cx, x: f64, y: f64, half: f64, p: f64) -> Canv {
    if cx.random() >= p {
        return Vec::new();
    }
    let side = if cx.random() < 0.5 { -1.0 } else { 1.0 };
    let fx = x + side * half * (0.55 + cx.random() * 0.35);
    let sca = figure_sca(y);
    if cx.random() < 0.6 {
        angler(cx, fx, y + 4.0, sca, side < 0.0)
    } else {
        let fli = cx.random() < 0.5;
        traveller(cx, fx, y + 2.0, sca, fli)
    }
}

/// Farthest a near shore draws beyond its `[x, x + len]` (rods, reeds).
pub const SHORE_REACH: f64 = 220.0;

/// A near shore: low ground rising from below the bottom edge of the
/// painting, with its water line at `y`, `len` long and up to about `hei`
/// high, sloping into the water at both ends. On it: rocks, reeds at the
/// water's edge, one kind of trees (big pines, a grove, bare trees or
/// bushes), and now and then someone.
pub fn near_shore(cx: &mut Cx, x: f64, y: f64, len: f64, hei: f64, seed: f64) -> Canv {
    let (n, rows) = (50usize, 5usize);
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(rows);
    for r in 0..rows {
        let p = 1.0 - r as f64 / rows as f64 * 0.7;
        let row = (0..n)
            .map(|i| {
                let u = (i as f64 / (n - 1) as f64 - 0.5) * p + 0.5;
                let rise = (PI * u).sin().powf(0.6);
                let ns = cx.noise(u * len * 0.008, seed, r as f64 * 0.3);
                [x + u * len, y - hei * rise * (0.55 + 0.6 * ns) * p + r as f64 * 6.0]
            })
            .collect();
        ptlist.push(row);
    }
    let top = ptlist[0].clone();
    let top_at = |t: f64| {
        let f = (t.clamp(0.0, 1.0) * (n - 1) as f64).min((n - 1) as f64 - 1e-9);
        let (i, u) = (f as usize, f.fract());
        top[i][1] * (1.0 - u) + top[i + 1][1] * u
    };

    // the ground, white over the water behind, outlined and textured
    let mut ground = top.clone();
    ground.push([x + len, y + 90.0]);
    ground.push([x, y + 90.0]);
    let mut canv = poly(&ground, PolyArgs { fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() });
    canv.extend(stroke(cx, &top, StrokeArgs { col: grey(0.3), noi: 1.0, wid: 3.0, ..Default::default() }));
    let dis = |cx: &mut Cx| if cx.random() > 0.5 { 0.1 + 0.4 * cx.random() } else { 0.9 - 0.4 * cx.random() };
    let tex = (len * 0.2) as usize;
    canv.extend(texture(cx, &ptlist, TextureArgs { tex, wid: 2.0, dis: Some(&dis), ..Default::default() }));

    // one kind of trees
    let on = |t: f64| [x + t * len, top_at(t) + 4.0];
    let kind = cx.rand_choice(&[0, 0, 0, 1, 1, 2, 2, 3]);
    let mut pine = None;
    match kind {
        0 => {
            for _ in 0..cx.rand_choice(&[1, 1, 2]) {
                let t = 0.25 + cx.random() * 0.5;
                let [px, py] = on(t);
                let hei = 140.0 + cx.random() * 100.0;
                canv.extend(tree04(cx, px, py, Tree04Args { hei, wid: 5.0, ..Default::default() }));
                pine = Some((t, px, py));
            }
        }
        1 => {
            let t0 = 0.2 + cx.random() * 0.4;
            for k in 0..2 + (cx.random() * 3.0) as usize {
                let [px, py] = on(t0 + k as f64 * 30.0 / len);
                let hei = 120.0 + cx.random() * 80.0;
                let px = px + cx.norm_rand(-8.0, 8.0);
                canv.extend(tree05(cx, px, py, Tree05Args { hei, ..Default::default() }));
            }
        }
        2 => {
            for _ in 0..1 + (cx.random() * 2.0) as usize {
                let [px, py] = on(0.2 + cx.random() * 0.6);
                let hei = 70.0 + cx.random() * 50.0;
                if cx.random() < 0.5 {
                    canv.extend(tree06(cx, px, py, Tree06Args { hei, ..Default::default() }));
                } else {
                    canv.extend(tree08(cx, px, py, Tree08Args { hei, ..Default::default() }));
                }
            }
        }
        _ => {
            let (t0, t1) = (0.15 + cx.random() * 0.3, 0.55 + cx.random() * 0.3);
            let mut t = t0;
            while t < t1 {
                let [px, py] = on(t);
                let hei = cx.norm_rand(40.0, 80.0);
                let py = py + cx.norm_rand(-1.0, 1.0);
                canv.extend(tree07(cx, px, py, Tree07Args { hei, ..Default::default() }));
                t += 20.0 / len;
            }
        }
    }
    for _ in 0..(len / 40.0 * cx.random()) as usize {
        let [px, py] = on(0.1 + cx.random() * 0.8);
        let py = py + cx.random() * 10.0;
        canv.extend(tree02(cx, px, py, Tree02Args::default()));
    }

    // rocks, and reeds where the ground meets the water
    for _ in 0..1 + (cx.random() * 3.0) as usize {
        let [rx, ry] = on(0.1 + cx.random() * 0.8);
        let (wid, hei) = (25.0 + cx.random() * 25.0, 18.0 + cx.random() * 17.0);
        let s = cx.random() * 100.0;
        canv.extend(rock(cx, rx, ry + 8.0, s, RockArgs { wid, hei, sha: 2.0, tex: 40 }));
    }
    for t in [0.06, 0.94] {
        if cx.random() < 0.6 {
            let [rx, ry] = on(t + (cx.random() - 0.5) * 0.06);
            canv.extend(reeds(cx, rx, ry.max(y - 12.0), 1.1));
        }
    }

    // maybe someone: a scholar under the pine (40% when there is one), an
    // angler at the water (25%) or a traveller (20%)
    let sca = figure_sca(y);
    let who = cx.random();
    if who < 0.4 {
        if let Some((t, px, py)) = pine {
            let side = if t < 0.5 { 1.0 } else { -1.0 };
            let sx = px + side * (14.0 + cx.random() * 14.0);
            canv.extend(sitter(cx, sx, py.max(top_at((sx - x) / len) + 4.0), sca, side < 0.0));
        }
    } else if who < 0.65 {
        let left = cx.random() < 0.5;
        let [ax, ay] = on(if left { 0.12 } else { 0.88 });
        canv.extend(angler(cx, ax, ay, sca, left));
    } else if who < 0.85 {
        let [tx, ty] = on(0.3 + cx.random() * 0.4);
        let fli = cx.random() < 0.5;
        canv.extend(traveller(cx, tx, ty, sca, fli));
    }
    canv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::Noise;

    fn finite(c: &Canv) -> bool {
        c.iter().all(|s| s.pts.iter().all(|p| p[0].is_finite() && p[1].is_finite()))
    }

    #[test]
    fn figures_are_finite() {
        let nz = Noise::new(2);
        for seed in 0..20 {
            let mut cx = Cx::new(&nz, seed);
            for fli in [false, true] {
                assert!(finite(&traveller(&mut cx, 100.0, 500.0, 0.45, fli)));
                assert!(finite(&sitter(&mut cx, 100.0, 500.0, 0.45, fli)));
                assert!(finite(&angler(&mut cx, 100.0, 500.0, 0.45, fli)));
            }
        }
    }

    /// Visual check: the figures large, both ways round, written to the temp dir.
    #[test]
    #[ignore]
    fn preview_figures() {
        let nz = Noise::new(5);
        let mut cx = Cx::new(&nz, 3);
        let mut canv = Vec::new();
        for (row, fli) in [(0.0, false), (1.0, true)] {
            let y = 300.0 + row * 300.0;
            canv.extend(traveller(&mut cx, 120.0, y, 1.6, fli));
            canv.extend(traveller(&mut cx, 330.0, y, 1.6, fli));
            canv.extend(sitter(&mut cx, 560.0, y, 1.6, fli));
            canv.extend(angler(&mut cx, 820.0, y, 1.6, fli));
        }
        let path = std::env::temp_dir().join("shanshui-figures.png");
        crate::paint::preview_png(&canv, 0.0, 1100, 700, path.to_str().unwrap()).unwrap();
    }
}
