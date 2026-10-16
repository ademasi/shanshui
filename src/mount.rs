//! Mountains, flat mountains, distant mountains and rocks (`Mount.*` in the
//! original).
//!
//! Every function draws in world coordinates and returns a `Canv` in paint
//! order. `seed` is the original's numeric seed argument (a noise offset),
//! not an RNG seed: randomness comes from `cx`.

use crate::arch::{
    Arch01Args, Arch02Args, Arch03Args, Arch04Args, arch01, arch02, arch03, arch04,
};
use crate::brush::{StrokeArgs, TextureArgs, stroke, texture};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, div, mid_pt, poly, rgb, rgba, triangulate};
use crate::nature::{shore_reeds, waterfall};
use crate::rng::Cx;
use crate::tree::{
    Tree01Args, Tree02Args, Tree03Args, Tree04Args, Tree05Args, Tree06Args, Tree07Args,
    Tree08Args, tree01, tree02, tree03, tree04, tree05, tree06, tree07, tree08,
};
use std::f64::consts::PI;

fn grey(a: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, a)
}

fn shifted(pts: &[Pt], xof: f64, yof: f64) -> Vec<Pt> {
    pts.iter().map(|p| [p[0] + xof, p[1] + yof]).collect()
}

/// White silhouette `ptlist[0] + [extra]` then the grey outline stroke, as
/// mountain, flatMount and rock all begin.
fn silhouette(cx: &mut Cx, top: &[Pt], extra: Pt, xof: f64, yof: f64) -> Canv {
    let mut bg = top.to_vec();
    bg.push(extra);
    let mut canv = poly(
        &bg,
        PolyArgs { xof, yof, fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() },
    );
    canv.extend(stroke(
        cx,
        &shifted(top, xof, yof),
        StrokeArgs { col: grey(0.3), noi: 1.0, wid: 3.0, ..Default::default() },
    ));
    canv
}

/// Private `foot`: the white skirts and strokes along the mountain base.
fn foot(cx: &mut Cx, ptlist: &[Vec<Pt>], xof: f64, yof: f64) -> Canv {
    let mut ftlist: Vec<Vec<Pt>> = Vec::new();
    let span = 10;
    let mut ni = 0usize;
    let n = ptlist.len();
    for i in 0..n.saturating_sub(2) {
        if i != ni {
            continue;
        }
        ni = (ni + cx.rand_choice(&[1usize, 2])).min(n - 1);
        let row = &ptlist[i];
        let rl = row.len();
        let mut a = Vec::new();
        let mut b = Vec::new();
        let lim = (rl as f64 / 8.0).min(10.0);
        let mut j = 0usize;
        while (j as f64) < lim {
            let ns = cx.noise(j as f64 * 0.1, i as f64, 0.0) * 10.0;
            a.push([row[j][0] + ns, row[j][1]]);
            b.push([row[rl - 1 - j][0] - ns, row[rl - 1 - j][1]]);
            j += 1;
        }
        a.reverse();
        b.reverse();
        for j in 0..span {
            let p = j as f64 / span as f64;
            let x1 = row[0][0] * (1.0 - p) + ptlist[ni][0][0] * p;
            let mut y1 = row[0][1] * (1.0 - p) + ptlist[ni][0][1] * p;
            let x2 = row[rl - 1][0] * (1.0 - p) + ptlist[ni][rl - 1][0] * p;
            let mut y2 = row[rl - 1][1] * (1.0 - p) + ptlist[ni][rl - 1][1] * p;
            let vib = -1.7 * (p - 1.0) * p.powf(1.0 / 5.0);
            let d = vib * 5.0 + cx.noise(xof * 0.05, i as f64, 0.0) * 5.0;
            y1 += d;
            y2 += d;
            a.push([x1, y1]);
            b.push([x2, y2]);
        }
        ftlist.push(a);
        ftlist.push(b);
    }
    let mut canv = Vec::new();
    for f in &ftlist {
        canv.extend(poly(
            f,
            PolyArgs { xof, yof, fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() },
        ));
    }
    for f in &ftlist {
        let col = grey(0.1 + cx.random() * 0.1);
        canv.extend(stroke(
            cx,
            &shifted(f, xof, yof),
            StrokeArgs { col, wid: 1.0, ..Default::default() },
        ));
    }
    canv
}

/// Private `vegetate`: collect grid points passing `growth`, then draw `tree`
/// at each one passing `proof` (all growth tests run before any drawing).
fn vegetate(
    cx: &mut Cx,
    ptlist: &[Vec<Pt>],
    canv: &mut Canv,
    mut tree: impl FnMut(&mut Cx, f64, f64) -> Canv,
    mut growth: impl FnMut(&mut Cx, usize, usize) -> bool,
    proof: impl Fn(&[Pt], usize) -> bool,
) {
    let mut veglist = Vec::new();
    for (i, row) in ptlist.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            if growth(cx, i, j) {
                veglist.push(*p);
            }
        }
    }
    for i in 0..veglist.len() {
        if proof(&veglist, i) {
            canv.extend(tree(cx, veglist[i][0], veglist[i][1]));
        }
    }
}

pub struct MountainArgs<'a> {
    /// default `100 + random * 400`
    pub hei: Option<f64>,
    /// default `400 + random * 200`
    pub wid: Option<f64>,
    pub tex: usize,
    pub veg: bool,
    /// texture colour; `None` = texture's default
    pub col: Option<&'a dyn Fn(&mut Cx, f64) -> Rgba>,
}

impl Default for MountainArgs<'_> {
    fn default() -> Self {
        MountainArgs { hei: None, wid: None, tex: 200, veg: true, col: None }
    }
}

/// `Mount.mountain(xoff, yoff, seed, args)`
pub fn mountain(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: MountainArgs) -> Canv {
    let hei = match a.hei {
        Some(v) => v,
        None => 100.0 + cx.random() * 400.0,
    };
    let wid = match a.wid {
        Some(v) => v,
        None => 400.0 + cx.random() * 200.0,
    };
    let (h, w) = (hei, wid);
    let reso = [10usize, 50];
    let mut canv = Vec::new();

    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    let mut hoff = 0.0;
    for j in 0..reso[0] {
        hoff += cx.random() * yoff / 100.0;
        let mut row = Vec::with_capacity(reso[1]);
        for i in 0..reso[1] {
            let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
            let y = x.cos() * cx.noise(x + 10.0, j as f64 * 0.15, seed);
            let p = 1.0 - j as f64 / reso[0] as f64;
            row.push([(x / PI) * w * p, -y * h * p + hoff]);
        }
        ptlist.push(row);
    }
    let nz = cx.nz;
    let pl = &ptlist;
    let last = reso[1] - 1;
    let tree_col = |x: f64, y: f64, base: f64| {
        grey(nz.noise(0.01 * x, 0.01 * y, 0.0) * 0.5 * 0.3 + base)
    };

    // RIM
    vegetate(
        cx,
        pl,
        &mut canv,
        |cx, x, y| {
            tree02(
                cx,
                x + xoff,
                y + yoff - 5.0,
                Tree02Args { col: tree_col(x, y, 0.5), clu: 2.0, ..Default::default() },
            )
        },
        |_, i, j| {
            let ns = nz.noise(j as f64 * 0.1, seed, 0.0);
            i == 0 && ns * ns * ns < 0.1 && pl[i][j][1].abs() / h > 0.2
        },
        |_, _| true,
    );

    // WHITE BG + OUTLINE
    canv.extend(silhouette(cx, &ptlist[0], [0.0, reso[0] as f64 * 4.0], xoff, yoff));
    canv.extend(foot(cx, &ptlist, xoff, yoff));
    let sha = cx.rand_choice(&[0.0, 0.0, 0.0, 0.0, 5.0]);
    canv.extend(texture(
        cx,
        &ptlist,
        TextureArgs { xof: xoff, yof: yoff, tex: a.tex, sha, col: a.col, ..Default::default() },
    ));

    // WATERFALL (not in the original): from a ledge on an inner ridge, well
    // below the skyline, down to the foot.
    if a.veg && h > 200.0 && cx.random() < 0.45 {
        let k = cx.rand_choice(&[2usize, 3, 4]);
        let i = 17 + (cx.random() * 16.0) as usize;
        let top = pl[k][i];
        let sky = skyline(&ptlist[0], top[0]);
        if top[1] - sky > 0.12 * h && -top[1] > 0.3 * h {
            let wid = 16.0 + cx.random() * 12.0;
            canv.extend(waterfall(cx, top[0] + xoff, top[1] + yoff, yoff + 6.0, wid));
        }
    }

    // TOP
    vegetate(
        cx,
        pl,
        &mut canv,
        |cx, x, y| {
            tree02(
                cx,
                x + xoff,
                y + yoff,
                Tree02Args { col: tree_col(x, y, 0.5), ..Default::default() },
            )
        },
        |_, i, j| {
            let ns = nz.noise(i as f64 * 0.1, j as f64 * 0.1, seed + 2.0);
            ns * ns * ns < 0.1 && pl[i][j][1].abs() / h > 0.5
        },
        |_, _| true,
    );

    if a.veg {
        // MIDDLE
        vegetate(
            cx,
            pl,
            &mut canv,
            |cx, x, y| {
                let mut ht = ((h + y) / h) * 70.0;
                ht = ht * 0.3 + cx.random() * ht * 0.7;
                let wid = cx.random() * 3.0 + 1.0;
                tree01(
                    cx,
                    x + xoff,
                    y + yoff,
                    Tree01Args { hei: ht, wid, col: tree_col(x, y, 0.3), ..Default::default() },
                )
            },
            |_, i, j| {
                let ns = nz.noise(i as f64 * 0.2, j as f64 * 0.05, seed);
                j % 2 == 1 && ns.powi(4) < 0.02 && pl[i][j][1].abs() / h < 0.3
            },
            |veglist, i| {
                let mut counter = 0;
                for j in 0..veglist.len() {
                    if i != j
                        && (veglist[i][0] - veglist[j][0]).powi(2)
                            + (veglist[i][1] - veglist[j][1]).powi(2)
                            < 30.0 * 30.0
                    {
                        counter += 1;
                    }
                    if counter > 2 {
                        return true;
                    }
                }
                false
            },
        );

        // BOTTOM
        vegetate(
            cx,
            pl,
            &mut canv,
            |cx, x, y| {
                let mut ht = ((h + y) / h) * 120.0;
                ht = ht * 0.5 + cx.random() * ht * 0.5;
                let bc = cx.random() * 0.1;
                let bp = 1.0;
                let ben = move |x: f64| (x * bc).powf(bp);
                tree03(
                    cx,
                    x + xoff,
                    y + yoff,
                    Tree03Args { hei: ht, ben: Some(&ben), col: tree_col(x, y, 0.3), ..Default::default() },
                )
            },
            |_, i, j| {
                let ns = nz.noise(i as f64 * 0.2, j as f64 * 0.05, seed);
                (j == 0 || j == last) && ns.powi(4) < 0.02
            },
            |_, _| true,
        );

        // CLIFF PINES (not in the original): a couple of small gnarled pines
        // on the upper slopes.
        vegetate(
            cx,
            pl,
            &mut canv,
            |cx, x, y| {
                let hei = 45.0 + cx.random() * 45.0;
                tree04(cx, x + xoff, y + yoff, Tree04Args { hei, wid: 2.5, col: tree_col(x, y, 0.4), ..Default::default() })
            },
            |cx, i, j| {
                let r = pl[i][j][1].abs() / h;
                (1..5).contains(&i) && (0.35..0.8).contains(&r) && cx.random() < 0.012
            },
            |_, i| i < 2,
        );
    }

    // BOTT ARCH
    vegetate(
        cx,
        pl,
        &mut canv,
        |cx, x, y| {
            let tt = cx.rand_choice(&[0, 0, 0, 0, 1, 1, 2]);
            if tt == 1 {
                let wid = cx.norm_rand(40.0, 70.0);
                let sto = cx.rand_choice(&[1.0, 2.0, 2.0, 3.0]);
                let rot = cx.random();
                let sty = cx.rand_choice(&[1.0, 2.0, 3.0]);
                arch02(
                    cx,
                    x + xoff,
                    y + yoff,
                    seed,
                    Arch02Args { wid, sto, rot, sty, ..Default::default() },
                )
            } else if tt == 2 {
                let sto = cx.rand_choice(&[1.0, 1.0, 1.0, 2.0, 2.0]);
                arch04(cx, x + xoff, y + yoff, seed, Arch04Args { sto, ..Default::default() })
            } else {
                Vec::new()
            }
        },
        |_, i, j| {
            let ns = nz.noise(i as f64 * 0.2, j as f64 * 0.05, seed + 10.0);
            i != 0 && (j == 1 || j == last - 1) && ns.powi(4) < 0.004
        },
        |_, _| true,
    );

    // TOP ARCH
    vegetate(
        cx,
        pl,
        &mut canv,
        |cx, x, y| {
            let sto = cx.rand_choice(&[5.0, 7.0]);
            let wid = 40.0 + cx.random() * 20.0;
            arch03(cx, x + xoff, y + yoff, seed, Arch03Args { sto, wid, ..Default::default() })
        },
        |cx, i, j| {
            i == 1 && (j as f64 - pl[i].len() as f64 / 2.0).abs() < 1.0 && cx.random() < 0.008
        },
        |_, _| true,
    );

    // BOTT ROCK
    vegetate(
        cx,
        pl,
        &mut canv,
        |cx, x, y| {
            let wid = 20.0 + cx.random() * 20.0;
            let hei = 20.0 + cx.random() * 20.0;
            rock(cx, x + xoff, y + yoff, seed, RockArgs { wid, hei, sha: 2.0, ..Default::default() })
        },
        |cx, _, j| (j == 0 || j == last) && cx.random() < 0.1,
        |_, _| true,
    );

    // REEDS (not in the original) along the shore at its foot
    if a.veg {
        canv.extend(shore_reeds(cx, xoff, yoff, w * 0.45, (yoff / 700.0).clamp(0.5, 1.1)));
    }

    canv
}

/// Height of a mountain's outline `row` at `x` (rows run left to right).
fn skyline(row: &[Pt], x: f64) -> f64 {
    for w in row.windows(2) {
        if w[0][0] <= x && x <= w[1][0] {
            let t = (x - w[0][0]) / (w[1][0] - w[0][0]).max(1e-9);
            return w[0][1] + t * (w[1][1] - w[0][1]);
        }
    }
    0.0
}

pub struct FlatMountArgs {
    /// default `40 + random * 400`
    pub hei: Option<f64>,
    /// default `400 + random * 200`
    pub wid: Option<f64>,
    pub tex: usize,
    pub cho: f64,
}

impl Default for FlatMountArgs {
    fn default() -> Self {
        FlatMountArgs { hei: None, wid: None, tex: 80, cho: 0.5 }
    }
}

/// Bounding box of a ground polygon, the `grbd` handed to `flatDec`.
#[derive(Clone, Copy, Debug)]
pub struct Bound {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
}

fn bound(plist: &[Pt]) -> Bound {
    let mut b = Bound {
        xmin: f64::INFINITY,
        xmax: f64::NEG_INFINITY,
        ymin: f64::INFINITY,
        ymax: f64::NEG_INFINITY,
    };
    for p in plist {
        b.xmin = b.xmin.min(p[0]);
        b.xmax = b.xmax.max(p[0]);
        b.ymin = b.ymin.min(p[1]);
        b.ymax = b.ymax.max(p[1]);
    }
    b
}

/// `Mount.flatMount(xoff, yoff, seed, args)`
pub fn flat_mount(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: FlatMountArgs) -> Canv {
    let hei = match a.hei {
        Some(v) => v,
        None => 40.0 + cx.random() * 400.0,
    };
    let wid = match a.wid {
        Some(v) => v,
        None => 400.0 + cx.random() * 200.0,
    };
    let cho = a.cho;
    let mut canv = Vec::new();
    let reso = [5usize, 50];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    let mut flat: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    let mut hoff = 0.0;
    for j in 0..reso[0] {
        hoff += cx.random() * yoff / 100.0;
        let mut row: Vec<Pt> = Vec::with_capacity(reso[1]);
        let mut fl: Vec<Pt> = Vec::new();
        for i in 0..reso[1] {
            let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
            let y = ((x * 2.0).cos() + 1.0) * cx.noise(x + 10.0, j as f64 * 0.1, seed);
            let p = 1.0 - (j as f64 / reso[0] as f64) * 0.6;
            let nx = (x / PI) * wid * p;
            let mut ny = -y * hei * p + hoff;
            let h = 100.0;
            if ny < -h * cho + hoff {
                ny = -h * cho + hoff;
                if fl.len() % 2 == 0 {
                    fl.push([nx, ny]);
                }
            } else if fl.len() % 2 == 1 {
                fl.push(*row.last().expect("odd flat run implies a previous point"));
            }
            row.push([nx, ny]);
        }
        ptlist.push(row);
        flat.push(fl);
    }

    canv.extend(silhouette(cx, &ptlist[0], [0.0, reso[0] as f64 * 4.0], xoff, yoff));
    let dis = |cx: &mut Cx| {
        if cx.random() > 0.5 { 0.1 + 0.4 * cx.random() } else { 0.9 - 0.4 * cx.random() }
    };
    canv.extend(texture(
        cx,
        &ptlist,
        TextureArgs { xof: xoff, yof: yoff, tex: a.tex, wid: 2.0, dis: Some(&dis), ..Default::default() },
    ));

    let mut grlist1 = Vec::new();
    let mut grlist2 = Vec::new();
    for f in flat.iter().step_by(2) {
        if f.len() >= 2 {
            grlist1.push(f[0]);
            grlist2.push(f[f.len() - 1]);
        }
    }
    let reed_sca = (yoff / 700.0).clamp(0.5, 1.1);
    if grlist1.is_empty() {
        canv.extend(shore_reeds(cx, xoff, yoff, wid * 0.48, reed_sca));
        return canv;
    }
    let wb = [grlist1[0][0], grlist2[0][0]];
    for i in 0..3 {
        let p = 0.8 - i as f64 * 0.2;
        grlist1.insert(0, [wb[0] * p, grlist1[0][1] - 5.0]);
        grlist2.insert(0, [wb[1] * p, grlist2[0][1] - 5.0]);
    }
    let wb = [grlist1[grlist1.len() - 1][0], grlist2[grlist2.len() - 1][0]];
    for i in 0..3 {
        let p = 0.6 - (i * i) as f64 * 0.1;
        let l1 = grlist1[grlist1.len() - 1][1];
        let l2 = grlist2[grlist2.len() - 1][1];
        grlist1.push([wb[0] * p, l1 + 1.0]);
        grlist2.push([wb[1] * p, l2 + 1.0]);
    }
    let d = 5.0;
    let mut g1 = div(&grlist1, d);
    let g2 = div(&grlist2, d);
    g1.reverse();
    let mut grlist = g1.clone();
    grlist.extend(g2);
    grlist.push(g1[0]);
    // The JS closes the polygon with the *same* point object as its first
    // vertex, so the last iteration's wobble lands on both ends.
    let n = grlist.len();
    for i in 0..n {
        let v = (1.0 - ((i as f64 % d) - d / 2.0).abs() / (d / 2.0)) * 0.12;
        let f = 1.0 - v + cx.noise(grlist[i][1] * 0.5, 0.0, 0.0) * v;
        if i == n - 1 {
            grlist[0][0] *= f;
            grlist[i] = grlist[0];
        } else {
            grlist[i][0] *= f;
        }
    }
    canv.extend(poly(
        &grlist,
        PolyArgs { xof: xoff, yof: yoff, str: Some(Rgba::NONE), fil: Rgba::WHITE, wid: 2.0 },
    ));
    canv.extend(stroke(
        cx,
        &shifted(&grlist, xoff, yoff),
        StrokeArgs { wid: 3.0, col: grey(0.2), ..Default::default() },
    ));
    canv.extend(flat_dec(cx, xoff, yoff, bound(&grlist)));
    // REEDS (not in the original) where the ground meets the water
    canv.extend(shore_reeds(cx, xoff, yoff, wid * 0.48, reed_sca));
    canv
}

/// Default-args rock as `flatDec` places them.
fn dec_rock(cx: &mut Cx, x: f64, y: f64, seed: f64, wid: (f64, f64), hei: (f64, f64), sha: f64) -> Canv {
    let w = wid.0 + cx.random() * wid.1;
    let h = hei.0 + cx.random() * hei.1;
    rock(cx, x, y, seed, RockArgs { wid: w, hei: h, sha, ..Default::default() })
}

/// `Mount.flatDec(xoff, yoff, grbd)`: rocks, trees and a building on a plateau.
pub fn flat_dec(cx: &mut Cx, xoff: f64, yoff: f64, g: Bound) -> Canv {
    let mut canv = Vec::new();
    let mid = (g.ymin + g.ymax) / 2.0;
    let tt = cx.rand_choice(&[0, 0, 1, 2, 3, 4]);

    let mut j = 0.0;
    while j < cx.random() * 5.0 {
        let x = xoff + cx.norm_rand(g.xmin, g.xmax);
        let y = yoff + mid + cx.norm_rand(-10.0, 10.0) + 10.0;
        let s = cx.random() * 100.0;
        canv.extend(dec_rock(cx, x, y, s, (10.0, 20.0), (10.0, 20.0), 2.0));
        j += 1.0;
    }
    let mut j = 0;
    while j < cx.rand_choice(&[0, 0, 1, 2]) {
        let xr = xoff + cx.norm_rand(g.xmin, g.xmax);
        let yr = yoff + mid + cx.norm_rand(-5.0, 5.0) + 20.0;
        let mut k = 0.0;
        while k < 2.0 + cx.random() * 3.0 {
            let x = xr + cx.norm_rand(-30.0, 30.0).max(g.xmin).min(g.xmax);
            let hei = 60.0 + cx.random() * 40.0;
            canv.extend(tree08(cx, x, yr, Tree08Args { hei, ..Default::default() }));
            k += 1.0;
        }
        j += 1;
    }

    if tt == 0 {
        let mut j = 0.0;
        while j < cx.random() * 3.0 {
            let x = xoff + cx.norm_rand(g.xmin, g.xmax);
            let y = yoff + mid + cx.norm_rand(-5.0, 5.0) + 20.0;
            let s = cx.random() * 100.0;
            canv.extend(dec_rock(cx, x, y, s, (50.0, 20.0), (40.0, 20.0), 5.0));
            j += 1.0;
        }
    }
    if tt == 1 {
        let pmin = cx.random() * 0.5;
        let pmax = cx.random() * 0.5 + 0.5;
        let xmin = g.xmin * (1.0 - pmin) + g.xmax * pmin;
        let xmax = g.xmin * (1.0 - pmax) + g.xmax * pmax;
        let mut i = xmin;
        while i < xmax {
            let x = xoff + i + 20.0 * cx.norm_rand(-1.0, 1.0);
            let hei = 100.0 + cx.random() * 200.0;
            canv.extend(tree05(cx, x, yoff + mid + 20.0, Tree05Args { hei, ..Default::default() }));
            i += 30.0;
        }
        let mut j = 0.0;
        while j < cx.random() * 4.0 {
            let x = xoff + cx.norm_rand(g.xmin, g.xmax);
            let y = yoff + mid + cx.norm_rand(-5.0, 5.0) + 20.0;
            let s = cx.random() * 100.0;
            canv.extend(dec_rock(cx, x, y, s, (50.0, 20.0), (40.0, 20.0), 5.0));
            j += 1.0;
        }
    } else if tt == 2 {
        let mut i = 0;
        while i < cx.rand_choice(&[1, 1, 1, 1, 2, 2, 3]) {
            let xr = cx.norm_rand(g.xmin, g.xmax);
            let yr = mid;
            canv.extend(tree04(cx, xoff + xr, yoff + yr + 20.0, Tree04Args::default()));
            let mut j = 0.0;
            while j < cx.random() * 2.0 {
                let x = xoff + g.xmin.max(g.xmax.min(xr + cx.norm_rand(-50.0, 50.0)));
                let y = yoff + yr + cx.norm_rand(-5.0, 5.0) + 20.0;
                let s = j * i as f64 * cx.random() * 100.0;
                canv.extend(dec_rock(cx, x, y, s, (50.0, 20.0), (40.0, 20.0), 5.0));
                j += 1.0;
            }
            i += 1;
        }
    } else if tt == 3 {
        let mut i = 0;
        while i < cx.rand_choice(&[1, 1, 1, 1, 2, 2, 3]) {
            let x = xoff + cx.norm_rand(g.xmin, g.xmax);
            let hei = 60.0 + cx.random() * 60.0;
            canv.extend(tree06(cx, x, yoff + mid, Tree06Args { hei, ..Default::default() }));
            i += 1;
        }
    } else if tt == 4 {
        let pmin = cx.random() * 0.5;
        let pmax = cx.random() * 0.5 + 0.5;
        let xmin = g.xmin * (1.0 - pmin) + g.xmax * pmin;
        let xmax = g.xmin * (1.0 - pmax) + g.xmax * pmax;
        let mut i = xmin;
        while i < xmax {
            let x = xoff + i + 20.0 * cx.norm_rand(-1.0, 1.0);
            let y = yoff + mid + cx.norm_rand(-1.0, 1.0);
            let hei = cx.norm_rand(40.0, 80.0);
            canv.extend(tree07(cx, x, y, Tree07Args { hei, ..Default::default() }));
            i += 20.0;
        }
    }

    let mut i = 0.0;
    while i < 50.0 * cx.random() {
        let x = xoff + cx.norm_rand(g.xmin, g.xmax);
        let y = yoff + cx.norm_rand(g.ymin, g.ymax);
        canv.extend(tree02(cx, x, y, Tree02Args::default()));
        i += 1.0;
    }

    let ts = cx.rand_choice(&[0, 0, 0, 0, 0, 0, 0, 0, 1]);
    if ts == 1 && tt != 4 {
        let x = xoff + cx.norm_rand(g.xmin, g.xmax);
        let s = cx.random();
        let wid = cx.norm_rand(160.0, 200.0);
        let hei = cx.norm_rand(80.0, 100.0);
        let per = cx.random();
        canv.extend(arch01(cx, x, yoff + mid + 20.0, s, Arch01Args { wid, hei, per, ..Default::default() }));
    }
    canv
}

pub struct DistMountArgs {
    pub hei: f64,
    pub len: f64,
    pub seg: usize,
}

impl Default for DistMountArgs {
    fn default() -> Self {
        DistMountArgs { hei: 300.0, len: 2000.0, seg: 5 }
    }
}

/// `Mount.distMount(xoff, yoff, seed, args)`: faint far-away range, filled
/// with small triangles of noisy grey. Spans `[xoff, xoff + len]`.
pub fn dist_mount(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: DistMountArgs) -> Canv {
    let (hei, len, seg) = (a.hei, a.len, a.seg);
    let span = 10.0;
    let nz = cx.nz;
    let mut canv = Vec::new();
    let top = |k: f64| {
        [
            xoff + k * span,
            yoff - hei * nz.noise(k * 0.05, seed, 0.0) * (PI * k / (len / span)).sin().max(0.0).powf(0.5),
        ]
    };
    let bot = |k: f64| {
        [xoff + k * span, yoff + 24.0 * nz.noise(k * 0.05, 2.0, seed) * (PI * k / (len / span)).sin()]
    };
    let rows = len / span / seg as f64;
    let mut i = 0usize;
    while (i as f64) < rows {
        let mut row: Vec<Pt> = (0..=seg).map(|j| top((i * seg + j) as f64)).collect();
        let mut j = 0usize;
        while (j as f64) < seg as f64 / 2.0 + 1.0 {
            row.insert(0, bot((i * seg + j * 2) as f64));
            j += 1;
        }
        let get_col = |p: Pt| {
            let c = (nz.noise(p[0] * 0.02, p[1] * 0.02, yoff) * 55.0 + 200.0).trunc();
            rgb(c, c, c)
        };
        canv.extend(poly(
            &row,
            PolyArgs { fil: get_col(row[row.len() - 1]), str: Some(Rgba::NONE), wid: 1.0, ..Default::default() },
        ));
        for t in triangulate(&row, 100.0, true, false) {
            let co = get_col(mid_pt(&t));
            canv.extend(poly(&t, PolyArgs { fil: co, str: Some(co), wid: 1.0, ..Default::default() }));
        }
        i += 1;
    }
    canv
}

pub struct RockArgs {
    pub hei: f64,
    pub wid: f64,
    pub tex: usize,
    pub sha: f64,
}

impl Default for RockArgs {
    fn default() -> Self {
        RockArgs { hei: 80.0, wid: 100.0, tex: 40, sha: 10.0 }
    }
}

/// `Mount.rock(xoff, yoff, seed, args)`
pub fn rock(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: RockArgs) -> Canv {
    let (hei, wid) = (a.hei, a.wid);
    let reso = [10usize, 50];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    for i in 0..reso[0] {
        let mut nslist: Vec<f64> =
            (0..reso[1]).map(|j| cx.noise(i as f64, j as f64 * 0.2, seed)).collect();
        crate::geom::loop_noise(&mut nslist);
        let mut row = Vec::with_capacity(reso[1]);
        for j in 0..reso[1] {
            let a = (j as f64 / reso[1] as f64) * PI * 2.0 - PI / 2.0;
            let mut l = (wid * hei)
                / ((hei * a.cos()).powi(2) + (wid * a.sin()).powi(2)).sqrt();
            l *= 0.7 + 0.3 * nslist[j];
            let p = 1.0 - i as f64 / reso[0] as f64;
            let nx = a.cos() * l * p;
            let mut ny = -a.sin() * l * p;
            if PI < a || a < 0.0 {
                ny *= 0.2;
            }
            ny += hei * (i as f64 / reso[0] as f64) * 0.2;
            row.push([nx, ny]);
        }
        ptlist.push(row);
    }
    let mut canv = silhouette(cx, &ptlist[0], [0.0, 0.0], xoff, yoff);
    let col = |cx: &mut Cx, _x: f64| rgba(180.0, 180.0, 180.0, 0.3 + cx.random() * 0.3);
    let dis = |cx: &mut Cx| {
        if cx.random() > 0.5 { 0.15 + 0.15 * cx.random() } else { 0.85 - 0.15 * cx.random() }
    };
    canv.extend(texture(
        cx,
        &ptlist,
        TextureArgs {
            xof: xoff,
            yof: yoff,
            tex: a.tex,
            wid: 3.0,
            sha: a.sha,
            col: Some(&col),
            dis: Some(&dis),
            ..Default::default()
        },
    ));
    canv
}
