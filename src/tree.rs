//! Trees (`Tree.tree01` .. `Tree.tree08` in the original).
//!
//! Every JS arg key is a field with the same name; JS numbers are f64;
//! colours are `Rgba`.

use crate::brush::{BlobArgs, StrokeArgs, blob, blob_pts, stroke};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, distance, div, mid_pt, poly, rgba, triangulate};
use crate::rng::Cx;
use std::f64::consts::PI;

fn grey(a: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, a)
}

/// JS `x.toFixed(n)` followed by re-parsing.
fn fixed(v: f64, n: i32) -> f64 {
    let m = 10f64.powi(n);
    (v * m).round() / m
}

/// The leaf colour the original rebuilds from the `col` string, with a new alpha.
fn leaf(col: Rgba, a: f64) -> Rgba {
    rgba(col.r as f64, col.g as f64, col.b as f64, a)
}

/// Leaf profile shared by `tree02` and `twig`.
fn leaf_fun(x: f64) -> f64 {
    if x <= 1.0 {
        ((x * PI).sin() * x).powf(0.5)
    } else {
        -((x - 2.0) * PI * (x - 2.0)).sin().powf(0.5)
    }
}

/// `Math.sin(1)`: constant width profile used for trunk outlines.
fn const_fun(_x: f64) -> f64 {
    1f64.sin()
}

fn shift(pts: &[Pt], x: f64, y: f64) -> Vec<Pt> {
    pts.iter().map(|p| [p[0] + x, p[1] + y]).collect()
}

/// `trlist[0].concat(trlist[1].reverse())`
fn join(t0: &[Pt], t1: &[Pt]) -> Vec<Pt> {
    t0.iter().chain(t1.iter().rev()).copied().collect()
}

fn noise_list(cx: &Cx, reso: usize) -> Vec<[f64; 2]> {
    (0..reso)
        .map(|i| [cx.noise(i as f64 * 0.5, 0.0, 0.0), cx.noise(i as f64 * 0.5, 0.5, 0.0)])
        .collect()
}

pub struct Tree01Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree01Args {
    fn default() -> Self { Tree01Args { hei: 50.0, wid: 3.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree01(cx: &mut Cx, x: f64, y: f64, a: Tree01Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let reso = 10usize;
    let nslist = noise_list(cx, reso);
    let mut canv = Vec::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    for i in 0..reso {
        let nx = x;
        let ny = y - (i as f64 * hei) / reso as f64;
        let ri = (reso - i) as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let mut j = 0.0;
            while j < ri / 5.0 {
                let bx = nx + (cx.random() - 0.5) * wid * 1.2 * ri;
                let by = ny + (cx.random() - 0.5) * wid;
                let len = cx.random() * 20.0 * ri * 0.2 + 10.0;
                let bw = cx.random() * 6.0 + 3.0;
                let ang = ((cx.random() - 0.5) * PI) / 6.0;
                let c = leaf(col, fixed(cx.random() * 0.2 + col.a as f64, 1));
                canv.extend(blob(cx, bx, by, BlobArgs { len, wid: bw, ang, col: c, ..Default::default() }));
                j += 1.0;
            }
        }
        line1.push([nx + (nslist[i][0] - 0.5) * wid - wid / 2.0, ny]);
        line2.push([nx + (nslist[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    let la = |c: Rgba| PolyArgs { fil: Rgba::NONE, str: Some(c), wid: 1.5, ..Default::default() };
    canv.extend(poly(&line1, la(col)));
    canv.extend(poly(&line2, la(col)));
    canv
}

pub struct Tree02Args { pub hei: f64, pub wid: f64, pub clu: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree02Args {
    fn default() -> Self { Tree02Args { hei: 16.0, wid: 8.0, clu: 5.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree02(cx: &mut Cx, x: f64, y: f64, a: Tree02Args) -> Canv {
    let (hei, wid, clu, col) = (a.hei, a.wid, a.clu, a.col);
    let mut canv = Vec::new();
    let mut i = 0.0;
    while i < clu {
        let bx = x + cx.rand_gaussian() * clu * 4.0;
        let by = y + cx.rand_gaussian() * clu * 4.0;
        let bw = cx.random() * wid * 0.75 + wid * 0.5;
        let len = cx.random() * hei * 0.75 + hei * 0.5;
        canv.extend(blob(
            cx,
            bx,
            by,
            BlobArgs { ang: PI / 2.0, fun: Some(&leaf_fun), wid: bw, len, col, ..Default::default() },
        ));
        i += 1.0;
    }
    canv
}

pub struct Tree03Args<'a> {
    pub hei: f64,
    pub wid: f64,
    /// bend profile; `None` = the original's default function
    pub ben: Option<&'a dyn Fn(f64) -> f64>,
    pub col: Rgba,
    pub noi: f64,
}
impl Default for Tree03Args<'_> {
    fn default() -> Self { Tree03Args { hei: 50.0, wid: 5.0, ben: None, col: grey(0.5), noi: 0.5 } }
}
pub fn tree03(cx: &mut Cx, x: f64, y: f64, a: Tree03Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let default_ben = |_x: f64| 0.0;
    let ben: &dyn Fn(f64) -> f64 = a.ben.unwrap_or(&default_ben);
    let reso = 10usize;
    let nslist = noise_list(cx, reso);
    let shape = |x: f64| (50.0 * x + 1.0).ln() / 3.95;
    let mut canv = Vec::new();
    let mut blobs = Vec::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        let ri = (reso - i) as f64;
        if i as f64 >= reso as f64 / 5.0 {
            for _ in 0..(reso - i) * 2 {
                let ox = cx.random() * wid * 2.0 * shape(ri / reso as f64);
                let bx = nx + ox * cx.rand_choice(&[-1.0, 1.0]);
                let by = ny + (cx.random() - 0.5) * wid * 2.0;
                let bw = cx.random() * 6.0 + 3.0;
                let ang = ((cx.random() - 0.5) * PI) / 6.0;
                let c = leaf(col, fixed(cx.random() * 0.2 + col.a as f64, 3));
                blobs.extend(blob(cx, bx, by, BlobArgs { len: ox * 2.0, wid: bw, ang, col: c, ..Default::default() }));
            }
        }
        line1.push([nx + ((nslist[i][0] - 0.5) * wid - wid / 2.0) * ri / reso as f64, ny]);
        line2.push([nx + ((nslist[i][1] - 0.5) * wid + wid / 2.0) * ri / reso as f64, ny]);
    }
    let lc = join(&line1, &line2);
    canv.extend(poly(&lc, PolyArgs { fil: Rgba::WHITE, str: Some(col), wid: 1.5, ..Default::default() }));
    canv.extend(blobs);
    canv
}

/// `branch(args)`: the two outlines of a bent, tapering limb from the origin.
fn branch(cx: &mut Cx, hei: f64, wid: f64, ang: f64, det: f64, ben: f64) -> (Vec<Pt>, Vec<Pt>) {
    let (mut nx, mut ny) = (0.0, 0.0);
    let mut tlist: Vec<Pt> = vec![[nx, ny]];
    let mut a0 = 0.0;
    let g = 3;
    for _ in 0..g {
        a0 += (ben / 2.0 + (cx.random() * ben) / 2.0) * cx.rand_choice(&[-1.0, 1.0]);
        nx += (a0.cos() * hei) / g as f64;
        ny -= (a0.sin() * hei) / g as f64;
        tlist.push([nx, ny]);
    }
    let last = tlist[tlist.len() - 1];
    let ta = last[1].atan2(last[0]);
    for p in tlist.iter_mut() {
        let a = p[1].atan2(p[0]);
        let d = (p[0] * p[0] + p[1] * p[1]).sqrt();
        *p = [d * (a - ta + ang).cos(), d * (a - ta + ang).sin()];
    }

    let mut trlist1 = Vec::new();
    let mut trlist2 = Vec::new();
    let span = det;
    let tl = (tlist.len() - 1) as f64 * span;
    let (mut lx, mut ly) = (0.0, 0.0);
    let mut i = 0.0;
    while i < tl {
        let lastp = tlist[(i / span).floor() as usize];
        let nextp = tlist[((i / span).ceil() as usize).min(tlist.len() - 1)];
        let p = (i % span) / span;
        let nx = lastp[0] * (1.0 - p) + nextp[0] * p;
        let ny = lastp[1] * (1.0 - p) + nextp[1] * p;
        let ang = (ny - ly).atan2(nx - lx);
        let woff = ((cx.noise(i * 0.3, 0.0, 0.0) - 0.5) * wid * hei) / 80.0;
        let b = if p == 0.0 { cx.random() * wid } else { 0.0 };
        let nw = wid * (((tl - i) / tl) * 0.5 + 0.5);
        let (c1, s1) = ((ang + PI / 2.0).cos(), (ang + PI / 2.0).sin());
        let (c2, s2) = ((ang - PI / 2.0).cos(), (ang - PI / 2.0).sin());
        trlist1.push([nx + c1 * (nw + woff + b), ny + s1 * (nw + woff + b)]);
        trlist2.push([nx + c2 * (nw - woff + b), ny + s2 * (nw - woff + b)]);
        lx = nx;
        ly = ny;
        i += 1.0;
    }
    (trlist1, trlist2)
}

struct TwigArgs {
    dir: f64,
    sca: f64,
    wid: f64,
    ang: f64,
    lea: (bool, f64),
}
impl Default for TwigArgs {
    fn default() -> Self { TwigArgs { dir: 1.0, sca: 1.0, wid: 1.0, ang: 0.0, lea: (true, 12.0) } }
}

/// `twig(tx, ty, dep, args)`: a recursive twig, optionally with leaf clusters.
fn twig(cx: &mut Cx, tx: f64, ty: f64, dep: i32, a: TwigArgs) -> Canv {
    let TwigArgs { dir, sca, wid, ang, lea } = a;
    let mut canv = Vec::new();
    let mut twlist = Vec::new();
    let tl = 10;
    let hs = cx.random() * 0.5 + 0.5;
    let fun2 = |x: f64| -1.0 / (x + 1.0).powi(5) + 1.0;
    let _ = cx.rand_choice(&[0]); // randChoice([fun2])
    let a0 = ((cx.random() * PI) / 6.0) * dir + ang;
    for i in 0..tl {
        let mx = dir * fun2(i as f64 / tl as f64) * 50.0 * sca * hs;
        let my = -(i as f64) * 5.0 * sca;
        let a = my.atan2(mx);
        let d = (mx * mx + my * my).sqrt();
        let nx = (a + a0).cos() * d;
        let ny = (a + a0).sin() * d;
        twlist.push([nx + tx, ny + ty]);
        if (i == tl / 3 || i == tl * 2 / 3) && dep > 0 {
            let ndir = dir * cx.rand_choice(&[-1.0, 1.0]);
            canv.extend(twig(
                cx,
                nx + tx,
                ny + ty,
                dep - 1,
                TwigArgs { ang, sca: sca * 0.8, wid, dir: ndir, lea },
            ));
        }
        if i == tl - 1 && lea.0 {
            for j in 0..5 {
                let dj = (j as f64 - 2.5) * 5.0;
                let bx = nx + tx + ang.cos() * dj * wid;
                let by = ny + ty + (ang.sin() * dj - lea.1 / (dep + 1) as f64) * wid;
                let bw = (6.0 + 3.0 * cx.random()) * wid;
                let len = (15.0 + 12.0 * cx.random()) * wid;
                let bang = ang / 2.0 + PI / 2.0 + PI * 0.2 * (cx.random() - 0.5);
                let col = grey(fixed(0.5 + dep as f64 * 0.2, 3));
                canv.extend(blob(
                    cx,
                    bx,
                    by,
                    BlobArgs { wid: bw, len, ang: bang, col, fun: Some(&leaf_fun), ..Default::default() },
                ));
            }
        }
    }
    let tfun = |x: f64| (x * PI / 2.0).cos();
    canv.extend(stroke(cx, &twlist, StrokeArgs { wid: 1.0, fun: Some(&tfun), col: grey(0.5), ..Default::default() }));
    canv
}

/// `bark(x, y, wid, ang)` inside `barkify`.
fn bark(cx: &mut Cx, x: f64, y: f64, wid: f64, ang: f64) -> Canv {
    let len = 10.0 + 10.0 * cx.random();
    let fun = |x: f64| {
        if x <= 1.0 {
            (x * PI).sin().powf(0.5)
        } else {
            -((x + 1.0) * PI).sin().powf(0.5)
        }
    };
    // Same outline as `blob(.., {ret: 1})` with this profile and noi 0.5.
    let brklist = blob_pts(cx, x, y, &BlobArgs { len, wid, ang, noi: 0.5, fun: Some(&fun), ..Default::default() });
    let fr = cx.random();
    let sfun = move |x: f64| ((x + fr) * PI * 3.0).sin();
    stroke(
        cx,
        &brklist,
        StrokeArgs { wid: 0.8, noi: 0.0, col: grey(0.4), out: 0.0, fun: Some(&sfun), ..Default::default() },
    )
}

/// `barkify(x, y, trlist)`: bark marks and rough contour strokes along a limb.
///
/// The original mutates the endpoints of the contour runs in place, and those
/// points are shared with the caller's outline (JS array aliasing), which
/// roughens the trunk silhouette. That side effect is reproduced on `t0`/`t1`.
fn barkify(cx: &mut Cx, x: f64, y: f64, t0: &mut [Pt], t1: &mut [Pt]) -> Canv {
    let mut canv = Vec::new();
    let n = t0.len().min(t1.len());
    for i in 2..n.saturating_sub(1) {
        let a0 = (t0[i][1] - t0[i - 1][1]).atan2(t0[i][0] - t0[i - 1][0]);
        let a1 = (t1[i][1] - t1[i - 1][1]).atan2(t1[i][0] - t1[i - 1][0]);
        let p = cx.random();
        let nx = t0[i][0] * (1.0 - p) + t1[i][0] * p;
        let ny = t0[i][1] * (1.0 - p) + t1[i][1] * p;
        if cx.random() < 0.2 {
            canv.extend(blob(
                cx,
                nx + x,
                ny + y,
                BlobArgs {
                    noi: 1.0,
                    len: 15.0,
                    wid: 6.0 - (p - 0.5).abs() * 10.0,
                    ang: (a0 + a1) / 2.0,
                    col: grey(0.6),
                    ..Default::default()
                },
            ));
        } else {
            canv.extend(bark(cx, nx + x, ny + y, 5.0 - (p - 0.5).abs() * 10.0, (a0 + a1) / 2.0));
        }

        if cx.random() < 0.05 {
            let jl = cx.random() * 2.0 + 2.0;
            let xya = cx.rand_choice(&[[t0[i][0], t0[i][1], a0], [t1[i][0], t1[i][1], a1]]);
            let mut j = 0.0;
            while j < jl {
                let bx = xya[0] + x + xya[2].cos() * (j - jl / 2.0) * 4.0;
                let by = xya[1] + y + xya[2].sin() * (j - jl / 2.0) * 4.0;
                let len = 4.0 + 6.0 * cx.random();
                canv.extend(blob(
                    cx,
                    bx,
                    by,
                    BlobArgs { wid: 4.0, len, ang: a0 + PI / 2.0, col: grey(0.6), ..Default::default() },
                ));
                j += 1.0;
            }
        }
    }

    // trflist = t0 ++ reverse(t1); index k -> source point.
    let (n0, n1) = (t0.len(), t1.len());
    let src = |k: usize| -> Pt { if k < n0 { t0[k] } else { t1[n1 - 1 - (k - n0)] } };
    // runs of consecutive points, each with the trflist index of its last point
    let mut rglist: Vec<(Vec<Pt>, Option<usize>)> = vec![(Vec::new(), None)];
    for k in 0..n0 + n1 {
        if cx.random() < 0.5 {
            rglist.push((Vec::new(), None));
        } else {
            let last = rglist.last_mut().unwrap();
            last.0.push(src(k));
            last.1 = Some(k);
        }
    }

    let mut writeback = Vec::new();
    for (i, (run, last_k)) in rglist.iter().enumerate() {
        let mut pts = div(run, 4.0);
        for (j, pt) in pts.iter_mut().enumerate() {
            pt[0] += (cx.noise(i as f64, j as f64 * 0.1, 1.0) - 0.5) * (15.0 + 5.0 * cx.rand_gaussian());
            pt[1] += (cx.noise(i as f64, j as f64 * 0.1, 2.0) - 0.5) * (15.0 + 5.0 * cx.rand_gaussian());
        }
        if let (Some(k), Some(&p)) = (last_k, pts.last()) {
            writeback.push((*k, p));
        }
        canv.extend(stroke(
            cx,
            &shift(&pts, x, y),
            StrokeArgs { wid: 1.5, col: grey(0.7), out: 0.0, ..Default::default() },
        ));
    }
    for (k, p) in writeback {
        if k < n0 {
            t0[k] = p;
        } else {
            t1[n1 - 1 - (k - n0)] = p;
        }
    }
    canv
}

/// The trunk silhouette (white fill) and its constant-width ink outline,
/// shared by tree04/05/06. `trim` drops the first and last outline points.
fn trunk_outline(cx: &mut Cx, x: f64, y: f64, trmlist: &[Pt], col: Rgba, a0: f64, trim: bool) -> Canv {
    let mut canv = poly(trmlist, PolyArgs { xof: x, yof: y, fil: Rgba::WHITE, str: Some(col), wid: 0.0 });
    let pts = if trim && trmlist.len() >= 2 { &trmlist[1..trmlist.len() - 1] } else { trmlist };
    let sc = grey(fixed(a0 + cx.random() * 0.1, 3));
    canv.extend(stroke(
        cx,
        &shift(pts, x, y),
        StrokeArgs { col: sc, wid: 2.5, fun: Some(&const_fun), noi: 0.9, out: 0.0, ..Default::default() },
    ));
    canv
}

pub struct Tree04Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree04Args {
    fn default() -> Self { Tree04Args { hei: 300.0, wid: 6.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree04(cx: &mut Cx, x: f64, y: f64, a: Tree04Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let mut txcanv = Vec::new();
    let mut twcanv = Vec::new();

    let (mut t0, mut t1) = branch(cx, hei, wid, -PI / 2.0, 10.0, PI * 0.2);
    txcanv.extend(barkify(cx, x, y, &mut t0, &mut t1));
    let trlist = join(&t0, &t1);
    let l = trlist.len() as f64;

    let mut trmlist = Vec::new();
    for i in 0..trlist.len() {
        let fi = i as f64;
        if (fi >= l * 0.3 && fi <= l * 0.7 && cx.random() < 0.1) || fi == l / 2.0 - 1.0 {
            let ba = PI * 0.2 - PI * 1.4 * ((fi > l / 2.0) as i32 as f64);
            let bh = hei * (cx.random() + 1.0) * 0.3;
            let (mut b0, mut b1) = branch(cx, bh, wid * 0.5, ba, 10.0, PI * 0.2);
            if !b0.is_empty() {
                b0.remove(0);
            }
            if !b1.is_empty() {
                b1.remove(0);
            }
            let o = trlist[i];
            let mut f0 = shift(&b0, o[0], o[1]);
            let mut f1 = shift(&b1, o[0], o[1]);
            txcanv.extend(barkify(cx, x, y, &mut f0, &mut f1));

            for j in 0..b0.len() {
                if cx.random() < 0.2 || j == b0.len() - 1 {
                    twcanv.extend(twig(
                        cx,
                        b0[j][0] + o[0] + x,
                        b0[j][1] + o[1] + y,
                        1,
                        TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: (0.5 * hei) / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            ..Default::default()
                        },
                    ));
                }
            }
            trmlist.extend(shift(&join(&b0, &b1), o[0], o[1]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    let mut canv = trunk_outline(cx, x, y, &trmlist, col, 0.4, true);
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

pub struct Tree05Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree05Args {
    fn default() -> Self { Tree05Args { hei: 300.0, wid: 5.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree05(cx: &mut Cx, x: f64, y: f64, a: Tree05Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let mut txcanv = Vec::new();
    let mut twcanv = Vec::new();

    let (mut t0, mut t1) = branch(cx, hei, wid, -PI / 2.0, 10.0, 0.0);
    txcanv.extend(barkify(cx, x, y, &mut t0, &mut t1));
    let trlist = join(&t0, &t1);
    let l = trlist.len() as f64;

    let mut trmlist = Vec::new();
    for i in 0..trlist.len() {
        let fi = i as f64;
        let p = (fi - l * 0.5).abs() / (l * 0.5);
        if (fi >= l * 0.2 && fi <= l * 0.8 && i % 3 == 0 && cx.random() > p) || fi == l / 2.0 - 1.0 {
            let bar = cx.random() * 0.2;
            let ba = -bar * PI - (1.0 - bar * 2.0) * PI * ((fi > l / 2.0) as i32 as f64);
            let bh = hei * (0.3 * p - cx.random() * 0.05);
            let (mut b0, mut b1) = branch(cx, bh, wid * 0.5, ba, 10.0, 0.5);
            if !b0.is_empty() {
                b0.remove(0);
            }
            if !b1.is_empty() {
                b1.remove(0);
            }
            let o = trlist[i];
            for j in 0..b0.len() {
                if j % 20 == 0 || j == b0.len() - 1 {
                    twcanv.extend(twig(
                        cx,
                        b0[j][0] + o[0] + x,
                        b0[j][1] + o[1] + y,
                        0,
                        TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: (0.2 * hei) / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            lea: (true, 5.0),
                        },
                    ));
                }
            }
            trmlist.extend(shift(&join(&b0, &b1), o[0], o[1]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    let mut canv = trunk_outline(cx, x, y, &trmlist, col, 0.4, true);
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

/// `fracTree` inside `tree06`: returns the outline relative to (xoff, yoff).
#[allow(clippy::too_many_arguments)]
fn frac_tree06(
    cx: &mut Cx,
    xoff: f64,
    yoff: f64,
    dep: i32,
    hei: f64,
    wid: f64,
    ang: f64,
    ben: f64,
    txcanv: &mut Canv,
    twcanv: &mut Canv,
) -> Vec<Pt> {
    let (mut t0, mut t1) = branch(cx, hei, wid, ang, hei / 20.0, ben);
    txcanv.extend(barkify(cx, xoff, yoff, &mut t0, &mut t1));
    let trlist = join(&t0, &t1);
    let l = trlist.len() as f64;
    let half = (trlist.len() / 2) as i64;

    let mut trmlist = Vec::new();
    for i in 0..trlist.len() {
        let fi = i as f64;
        let ii = i as i64;
        let pick = (cx.random() < 0.025 && fi >= l * 0.2 && fi <= l * 0.8) || ii == half - 1 || ii == half + 1;
        if pick && dep > 0 {
            let bar = 0.02 + cx.random() * 0.08;
            let ba = bar * PI - bar * 2.0 * PI * ((fi > l / 2.0) as i32 as f64);
            let o = trlist[i];
            let bh = hei * (0.7 + cx.random() * 0.2);
            let brlist =
                frac_tree06(cx, o[0] + xoff, o[1] + yoff, dep - 1, bh, wid * 0.6, ang + ba, 0.55, txcanv, twcanv);
            for b in &brlist {
                if cx.random() < 0.03 {
                    let tang = ba * (cx.random() * 0.5 + 0.75);
                    twcanv.extend(twig(
                        cx,
                        b[0] + o[0] + xoff,
                        b[1] + o[1] + yoff,
                        2,
                        TwigArgs {
                            ang: tang,
                            sca: 0.3,
                            dir: if ba > 0.0 { 1.0 } else { -1.0 },
                            lea: (false, 0.0),
                            ..Default::default()
                        },
                    ));
                }
            }
            trmlist.extend(shift(&brlist, o[0], o[1]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    trmlist
}

pub struct Tree06Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree06Args {
    fn default() -> Self { Tree06Args { hei: 100.0, wid: 6.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree06(cx: &mut Cx, x: f64, y: f64, a: Tree06Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let mut txcanv = Vec::new();
    let mut twcanv = Vec::new();
    let trmlist = frac_tree06(cx, x, y, 3, hei, wid, -PI / 2.0, 0.0, &mut txcanv, &mut twcanv);
    let mut canv = trunk_outline(cx, x, y, &trmlist, col, 0.4, true);
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

pub struct Tree07Args<'a> {
    pub hei: f64,
    pub wid: f64,
    pub ben: Option<&'a dyn Fn(f64) -> f64>,
    pub col: Rgba,
    pub noi: f64,
}
impl Default for Tree07Args<'_> {
    fn default() -> Self { Tree07Args { hei: 60.0, wid: 4.0, ben: None, col: grey(1.0), noi: 0.5 } }
}
pub fn tree07(cx: &mut Cx, x: f64, y: f64, a: Tree07Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let default_ben = |x: f64| x.sqrt() * 0.2;
    let ben: &dyn Fn(f64) -> f64 = a.ben.unwrap_or(&default_ben);
    let reso = 10usize;
    let nslist = noise_list(cx, reso);
    let bfun = |x: f64| {
        if x <= 1.0 {
            2.75 * x * (1.0 - x).powf(1.0 / 1.8)
        } else {
            2.75 * (x - 2.0) * (x - 1.0).powf(1.0 / 1.8)
        }
    };
    let mut canv = Vec::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    let mut tris: Vec<Vec<Pt>> = Vec::new();
    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let bx = nx + (cx.random() - 0.5) * wid * 1.2 * (reso - i) as f64 * 0.5;
            let by = ny + (cx.random() - 0.5) * wid * 0.5;
            let len = cx.random() * 50.0 + 20.0;
            let bw = cx.random() * 12.0 + 12.0;
            let ang = (-cx.random() * PI) / 6.0;
            let bpl = blob_pts(
                cx,
                bx,
                by,
                &BlobArgs { len, wid: bw, ang, col: leaf(col, fixed(col.a as f64, 3)), fun: Some(&bfun), ..Default::default() },
            );
            tris.extend(triangulate(&bpl, 50.0, true, false));
        }
        line1.push([nx + (nslist[i][0] - 0.5) * wid - wid / 2.0, ny]);
        line2.push([nx + (nslist[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    let mut all = triangulate(&join(&line1, &line2), 50.0, true, true);
    all.extend(tris);
    for t in &all {
        let m = mid_pt(t);
        let c = (cx.noise(m[0] * 0.02, m[1] * 0.02, 0.0) * 200.0 + 50.0).trunc();
        let co = rgba(c, c, c, 0.8);
        canv.extend(poly(t, PolyArgs { fil: co, str: Some(co), wid: 0.0, ..Default::default() }));
    }
    canv
}

/// `fracTree` inside `tree08`: a recursive bare twig, drawn directly.
fn frac_tree08(cx: &mut Cx, xoff: f64, yoff: f64, dep: i32, ang: f64, len: f64, ben: f64) -> Canv {
    let tip = |x: f64| (0.5 * PI * x).cos();
    let flat = |_x: f64| 1.0;
    let fun: &dyn Fn(f64) -> f64 = if dep == 0 { &tip } else { &flat };
    let spt = [xoff, yoff];
    let ept = [xoff + ang.cos() * len, yoff + ang.sin() * len];

    let sign = cx.rand_choice(&[1.0, -1.0]);
    let mut trmlist = div(&[[xoff, yoff], [xoff + len, yoff]], 10.0);
    let n = trmlist.len();
    for (i, p) in trmlist.iter_mut().enumerate() {
        p[1] += sign * (i as f64 / n as f64 * PI).sin() * 2.0;
    }
    for p in trmlist.iter_mut() {
        let d = distance(*p, spt);
        let a = (p[1] - spt[1]).atan2(p[0] - spt[0]);
        *p = [spt[0] + d * (a + ang).cos(), spt[1] + d * (a + ang).sin()];
    }

    let mut tcanv = stroke(cx, &trmlist, StrokeArgs { fun: Some(fun), wid: 0.8, col: grey(0.5), ..Default::default() });
    if dep != 0 {
        let nben = ben + cx.rand_choice(&[-1.0, 1.0]) * PI * 0.001 * (dep * dep) as f64;
        if cx.random() < 0.5 {
            let c1 = [cx.norm_rand(-1.0, 0.5), cx.norm_rand(0.5, 1.0)];
            let a1 = ang + ben + PI * cx.rand_choice(&c1) * 0.2;
            let l1 = len * cx.norm_rand(0.8, 0.9);
            tcanv.extend(frac_tree08(cx, ept[0], ept[1], dep - 1, a1, l1, nben));
            let c2 = [cx.norm_rand(-1.0, -0.5), cx.norm_rand(0.5, 1.0)];
            let a2 = ang + ben + PI * cx.rand_choice(&c2) * 0.2;
            let l2 = len * cx.norm_rand(0.8, 0.9);
            tcanv.extend(frac_tree08(cx, ept[0], ept[1], dep - 1, a2, l2, nben));
        } else {
            let l1 = len * cx.norm_rand(0.8, 0.9);
            tcanv.extend(frac_tree08(cx, ept[0], ept[1], dep - 1, ang + ben, l1, nben));
        }
    }
    tcanv
}

pub struct Tree08Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree08Args {
    fn default() -> Self { Tree08Args { hei: 80.0, wid: 1.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree08(cx: &mut Cx, x: f64, y: f64, a: Tree08Args) -> Canv {
    let (hei, wid, col) = (a.hei, a.wid, a.col);
    let mut twcanv = Vec::new();
    let ang = cx.norm_rand(-1.0, 1.0) * PI * 0.2;

    let (t0, t1) = branch(cx, hei, wid, -PI / 2.0 + ang, hei / 20.0, PI * 0.2);
    let trlist = join(&t0, &t1);

    for i in 0..trlist.len() {
        if cx.random() < 0.2 {
            let dep = (4.0 * cx.random()).floor() as i32;
            let fa = -PI / 2.0 - ang * cx.random();
            twcanv.extend(frac_tree08(cx, x + trlist[i][0], y + trlist[i][1], dep, fa, 15.0, 0.0));
        } else if i == trlist.len() / 2 {
            twcanv.extend(frac_tree08(cx, x + trlist[i][0], y + trlist[i][1], 3, -PI / 2.0 + ang, 15.0, 0.0));
        }
    }

    let mut canv = trunk_outline(cx, x, y, &trlist, col, 0.6, false);
    canv.extend(twcanv);
    canv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::Noise;

    fn finite(canv: &Canv) -> bool {
        canv.iter().all(|s| s.pts.iter().all(|p| p[0].is_finite() && p[1].is_finite()))
    }

    #[test]
    fn trees_nonempty_and_finite() {
        let nz = Noise::new(7);
        for seed in 0..6u64 {
            let mut cx = Cx::new(&nz, seed);
            let all: Vec<Canv> = vec![
                tree01(&mut cx, 0.0, 500.0, Tree01Args::default()),
                tree02(&mut cx, 0.0, 500.0, Tree02Args::default()),
                tree03(&mut cx, 0.0, 500.0, Tree03Args::default()),
                tree04(&mut cx, 0.0, 500.0, Tree04Args::default()),
                tree05(&mut cx, 0.0, 500.0, Tree05Args::default()),
                tree06(&mut cx, 0.0, 500.0, Tree06Args::default()),
                tree07(&mut cx, 0.0, 500.0, Tree07Args::default()),
                tree08(&mut cx, 0.0, 500.0, Tree08Args::default()),
            ];
            for (k, c) in all.iter().enumerate() {
                assert!(!c.is_empty(), "tree0{} empty", k + 1);
                assert!(finite(c), "tree0{} has non-finite points (seed {seed})", k + 1);
            }
        }
    }

    /// Visual check: one column per tree type, written to the scratchpad.
    #[test]
    #[ignore]
    fn preview_trees() {
        let nz = Noise::new(3);
        let mut cx = Cx::new(&nz, 11);
        let mut canv = Vec::new();
        let g = 620.0;
        let col = 300.0;
        // small trees: three instances per column, mountain-style args
        for k in 0..3 {
            let dx = k as f64 * 80.0 - 80.0;
            let hei = 40.0 + k as f64 * 20.0;
            canv.extend(tree01(&mut cx, col * 0.5 + dx, g, Tree01Args { hei, wid: 2.5, ..Default::default() }));
            canv.extend(tree02(&mut cx, col * 1.5 + dx, g - 20.0, Tree02Args { hei: 16.0, wid: 8.0, clu: 5.0, ..Default::default() }));
            let ben = |x: f64| 0.5 * x * x * if k % 2 == 0 { 1.0 } else { -1.0 } * 0.3;
            canv.extend(tree03(&mut cx, col * 2.5 + dx, g, Tree03Args { hei: hei + 20.0, ben: Some(&ben), ..Default::default() }));
            canv.extend(tree07(&mut cx, col * 6.5 + dx, g, Tree07Args { hei: 50.0 + k as f64 * 15.0, ..Default::default() }));
            canv.extend(tree08(&mut cx, col * 7.5 + dx, g, Tree08Args { hei: 60.0 + k as f64 * 20.0, ..Default::default() }));
        }
        canv.extend(tree04(&mut cx, col * 3.5, g, Tree04Args::default()));
        canv.extend(tree05(&mut cx, col * 4.5, g, Tree05Args { hei: 250.0, ..Default::default() }));
        canv.extend(tree06(&mut cx, col * 5.5, g, Tree06Args { hei: 110.0, ..Default::default() }));
        assert!(finite(&canv));
        let path = "/tmp/claude-1000/-home-alex-git/01e62bf5-6418-42f5-9f02-d05c54456047/scratchpad/trees.png";
        crate::paint::preview_png(&canv, 0.0, 2400, 700, path).unwrap();
    }
}
