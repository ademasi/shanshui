//! Architecture and boats (`Arch.*` in the original).
//!
//! Every JS arg key is a field with the same name; JS numbers are f64.
//! `seed` is the original's numeric seed argument (used for noise offsets),
//! not an RNG seed.

use crate::brush::{StrokeArgs, TextureArgs, stroke, texture};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, div, poly, rgba, un_nan};
use crate::man::{self, Hat, Item, ManArgs};
use crate::rng::Cx;
use std::f64::consts::PI;

fn grey(a: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, a)
}

fn shift(pts: &[Pt], xoff: f64, yoff: f64) -> Vec<Pt> {
    pts.iter().map(|p| [p[0] + xoff, p[1] + yoff]).collect()
}

/// `stroke(pts + offset, {col, noi, wid, fun: x => 1})`, the arch outline brush.
fn flat_stroke(cx: &mut Cx, pts: &[Pt], xoff: f64, yoff: f64, col: Rgba, noi: f64, wid: f64) -> Canv {
    let one = |_: f64| 1.0;
    stroke(
        cx,
        &shift(pts, xoff, yoff),
        StrokeArgs { col, noi, wid, fun: Some(&one), ..Default::default() },
    )
}

/// `flip(ptlist)` around x = 0 for a flat point list.
fn flip(pts: Vec<Pt>) -> Vec<Pt> {
    pts.into_iter().map(|p| [-p[0], p[1]]).collect()
}

/// `hut`: thatched roof of `Arch.arch01`.
fn hut(cx: &mut Cx, xoff: f64, yoff: f64, hei: f64, wid: f64, tex: usize) -> Canv {
    let reso = [10usize, 10usize];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    for i in 0..reso[0] {
        let heir = hei + hei * 0.2 * cx.random();
        let row = (0..reso[1])
            .map(|j| {
                let tj = j as f64 / (reso[1] - 1) as f64;
                let nx = wid * (i as f64 / (reso[0] - 1) as f64 - 0.5) * tj.powf(0.7);
                [nx, heir * tj]
            })
            .collect();
        ptlist.push(row);
    }
    let first = &ptlist[0];
    let last = &ptlist[ptlist.len() - 1];
    let mut outline: Vec<Pt> = first[..first.len() - 1].to_vec();
    outline.extend(last[..last.len() - 1].iter().rev());

    let mut canv = Vec::new();
    canv.extend(poly(
        &outline,
        PolyArgs { xof: xoff, yof: yoff, fil: Rgba::WHITE, str: Some(Rgba::NONE), ..Default::default() },
    ));
    for edge in [first, last] {
        canv.extend(poly(
            edge,
            PolyArgs { xof: xoff, yof: yoff, fil: Rgba::NONE, str: Some(grey(0.3)), wid: 2.0 },
        ));
    }
    let col = |cx: &mut Cx, _x: f64| rgba(120.0, 120.0, 120.0, 0.3 + cx.random() * 0.3);
    let dis = |cx: &mut Cx| cx.wtrand(|a| a * a);
    let noi = |_: f64| 5.0;
    canv.extend(texture(
        cx,
        &ptlist,
        TextureArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 1.0,
            len: 0.25,
            col: Some(&col),
            dis: Some(&dis),
            noi: Some(&noi),
            ..Default::default()
        },
    ));
    canv
}

/// Facade decoration for `box` (`deco(style, {hsp, vsp})`).
#[derive(Clone, Copy)]
struct Deco {
    style: u32,
    hsp: [usize; 2],
    vsp: [usize; 2],
}

/// `deco(style, {pul, pur, pdl, pdr, hsp, vsp})`: window/lattice lines.
fn deco(d: Deco, pul: Pt, pur: Pt, pdl: Pt, pdr: Pt) -> Vec<Vec<Pt>> {
    let (hsp, vsp) = (d.hsp, d.vsp);
    let mut plist = Vec::new();
    if hsp[0] == 0 || vsp[0] == 0 {
        return plist;
    }
    let dl = div(&[pul, pdl], vsp[1] as f64);
    let dr = div(&[pur, pdr], vsp[1] as f64);
    let du = div(&[pul, pur], hsp[1] as f64);
    let dd = div(&[pdl, pdr], hsp[1] as f64);
    let vs = vsp[1] as f64;
    match d.style {
        1 | 3 => {
            let mlu = du[hsp[0]];
            let mru = du[du.len() - 1 - hsp[0]];
            let mld = dd[hsp[0]];
            let mrd = dd[du.len() - 1 - hsp[0]];
            let mut i = vsp[0];
            while i + vsp[0] < dl.len() {
                let mml = div(&[mlu, mld], vs)[i];
                let mmr = div(&[mru, mrd], vs)[i];
                if d.style == 1 {
                    plist.push(div(&[mml, dl[i]], 5.0));
                    plist.push(div(&[mmr, dr[i]], 5.0));
                } else {
                    let mmu = div(&[mlu, mru], vs)[i];
                    let mmd = div(&[mld, mrd], vs)[i];
                    plist.push(div(&[mml, mmr], 5.0));
                    plist.push(div(&[mmu, mmd], 5.0));
                }
                i += vsp[0];
            }
            plist.push(div(&[mlu, mld], 5.0));
            plist.push(div(&[mru, mrd], 5.0));
        }
        2 => {
            let mut i = hsp[0];
            while i + hsp[0] < du.len() {
                plist.push(div(&[du[i], dd[i]], 5.0));
                i += hsp[0];
            }
        }
        _ => {}
    }
    plist
}

struct BoxArgs {
    hei: f64,
    wid: f64,
    rot: f64,
    per: f64,
    tra: bool,
    bot: bool,
    wei: f64,
    dec: Option<Deco>,
}

impl Default for BoxArgs {
    fn default() -> Self {
        BoxArgs { hei: 20.0, wid: 120.0, rot: 0.7, per: 4.0, tra: true, bot: true, wei: 3.0, dec: None }
    }
}

/// `box`: one storey's walls.
fn box_(cx: &mut Cx, xoff: f64, yoff: f64, a: BoxArgs) -> Canv {
    let BoxArgs { hei, wid, rot, per, tra, bot, wei, dec } = a;
    let mid = -wid * 0.5 + wid * rot;
    let bmid = -wid * 0.5 + wid * (1.0 - rot);
    let mut ptlist = Vec::new();
    ptlist.push(div(&[[-wid * 0.5, -hei], [-wid * 0.5, 0.0]], 5.0));
    ptlist.push(div(&[[wid * 0.5, -hei], [wid * 0.5, 0.0]], 5.0));
    if bot {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [mid, per]], 5.0));
        ptlist.push(div(&[[wid * 0.5, 0.0], [mid, per]], 5.0));
    }
    ptlist.push(div(&[[mid, -hei], [mid, per]], 5.0));
    if tra {
        if bot {
            ptlist.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], 5.0));
            ptlist.push(div(&[[wid * 0.5, 0.0], [bmid, -per]], 5.0));
        }
        ptlist.push(div(&[[bmid, -hei], [bmid, -per]], 5.0));
    }
    let surf = if rot < 0.5 { 1.0 } else { -1.0 };
    if let Some(d) = dec {
        ptlist.extend(deco(
            d,
            [surf * wid * 0.5, -hei],
            [mid, -hei + per],
            [surf * wid * 0.5, 0.0],
            [mid, per],
        ));
    }
    let polist = [[-wid * 0.5, -hei], [wid * 0.5, -hei], [wid * 0.5, 0.0], [mid, per], [-wid * 0.5, 0.0]];

    let mut canv = Vec::new();
    if !tra {
        canv.extend(poly(
            &polist,
            PolyArgs { xof: xoff, yof: yoff, str: Some(Rgba::NONE), fil: Rgba::WHITE, ..Default::default() },
        ));
    }
    for pl in &ptlist {
        canv.extend(flat_stroke(cx, pl, xoff, yoff, grey(0.4), 1.0, wei));
    }
    canv
}

struct RailArgs {
    hei: f64,
    wid: f64,
    rot: f64,
    per: f64,
    seg: f64,
    wei: f64,
    tra: bool,
    fro: bool,
}

impl Default for RailArgs {
    fn default() -> Self {
        RailArgs { hei: 20.0, wid: 180.0, rot: 0.7, per: 4.0, seg: 4.0, wei: 1.0, tra: true, fro: true }
    }
}

/// `rail`: balustrade around a storey.
fn rail(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: RailArgs) -> Canv {
    let RailArgs { hei, wid, rot, per, seg, wei, tra, fro } = a;
    let mid = -wid * 0.5 + wid * rot;
    let bmid = -wid * 0.5 + wid * (1.0 - rot);
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    if fro {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [mid, per]], seg));
        ptlist.push(div(&[[mid, per], [wid * 0.5, 0.0]], seg));
    }
    if tra {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], seg));
        ptlist.push(div(&[[bmid, -per], [wid * 0.5, 0.0]], seg));
    }
    if fro {
        ptlist.push(div(&[[-wid * 0.5, -hei], [mid, -hei + per]], seg));
        ptlist.push(div(&[[mid, -hei + per], [wid * 0.5, -hei]], seg));
    }
    if tra {
        ptlist.push(div(&[[-wid * 0.5, -hei], [bmid, -hei - per]], seg));
        ptlist.push(div(&[[bmid, -hei - per], [wid * 0.5, -hei]], seg));
    }
    let n = ptlist.len();
    if n == 0 {
        return Vec::new();
    }
    if tra {
        // The original slices the same polyline twice ((open + n) % n == open).
        let open = (cx.random() * n as f64).floor() as usize;
        let l = &mut ptlist[open];
        let k = l.len().saturating_sub(2).max(1);
        l.truncate(k);
    }
    let mut canv = Vec::new();
    for i in 0..n / 2 {
        let k = (n / 2 + i) % n;
        for j in 0..ptlist[i].len() {
            ptlist[i][j][1] += (cx.noise(i as f64, j as f64 * 0.5, seed) - 0.5) * hei;
            let jj = j % ptlist[k].len();
            ptlist[k][jj][1] += (cx.noise(i as f64 + 0.5, j as f64 * 0.5, seed) - 0.5) * hei;
            let mut ln = div(&[ptlist[i][j], ptlist[k][jj]], 2.0);
            ln[0][0] += (cx.random() - 0.5) * hei * 0.5;
            canv.extend(poly(
                &ln,
                PolyArgs { xof: xoff, yof: yoff, fil: Rgba::NONE, str: Some(grey(0.5)), wid: 2.0 },
            ));
        }
    }
    for pl in &ptlist {
        canv.extend(flat_stroke(cx, pl, xoff, yoff, grey(0.5), 0.5, wei));
    }
    canv
}

struct RoofArgs {
    hei: f64,
    wid: f64,
    rot: f64,
    per: f64,
    cor: f64,
    wei: f64,
}

impl Default for RoofArgs {
    fn default() -> Self {
        RoofArgs { hei: 20.0, wid: 120.0, rot: 0.7, per: 4.0, cor: 5.0, wei: 3.0 }
    }
}

/// `roof`: hip roof of `Arch.arch02`. The original's `pla` signboard text
/// ("Pizza Hut" easter egg) is not drawn: the display list has no text.
fn roof(cx: &mut Cx, xoff: f64, yoff: f64, a: RoofArgs) -> Canv {
    let RoofArgs { hei, wid, rot, per, cor, wei } = a;
    let opf = |p: Vec<Pt>| if rot < 0.5 { flip(p) } else { p };
    let rrot = if rot < 0.5 { 1.0 - rot } else { rot };
    let mid = -wid * 0.5 + wid * rrot;
    let quat = (mid + wid * 0.5) * 0.5 - mid;

    let ptlist = [
        div(
            &opf(vec![
                [-wid * 0.5 + quat, -hei - per / 2.0],
                [-wid * 0.5 + quat * 0.5, -hei / 2.0 - per / 4.0],
                [-wid * 0.5 - cor, 0.0],
            ]),
            5.0,
        ),
        div(
            &opf(vec![[mid + quat, -hei], [(mid + quat + wid * 0.5) / 2.0, -hei / 2.0], [wid * 0.5 + cor, 0.0]]),
            5.0,
        ),
        div(
            &opf(vec![[mid + quat, -hei], [mid + quat / 2.0, -hei / 2.0 + per / 2.0], [mid + cor, per]]),
            5.0,
        ),
        div(&opf(vec![[-wid * 0.5 - cor, 0.0], [mid + cor, per]]), 5.0),
        div(&opf(vec![[wid * 0.5 + cor, 0.0], [mid + cor, per]]), 5.0),
        div(&opf(vec![[-wid * 0.5 + quat, -hei - per / 2.0], [mid + quat, -hei]]), 5.0),
    ];

    let polist = opf(vec![
        [-wid * 0.5, 0.0],
        [-wid * 0.5 + quat, -hei - per / 2.0],
        [mid + quat, -hei],
        [wid * 0.5, 0.0],
        [mid, per],
    ]);
    let mut canv = Vec::new();
    canv.extend(poly(
        &polist,
        PolyArgs { xof: xoff, yof: yoff, str: Some(Rgba::NONE), fil: Rgba::WHITE, ..Default::default() },
    ));
    for pl in &ptlist {
        canv.extend(flat_stroke(cx, pl, xoff, yoff, grey(0.4), 1.0, wei));
    }
    canv
}

struct PagroofArgs {
    hei: f64,
    wid: f64,
    per: f64,
    cor: f64,
    sid: usize,
    wei: f64,
}

impl Default for PagroofArgs {
    fn default() -> Self {
        PagroofArgs { hei: 20.0, wid: 120.0, per: 4.0, cor: 10.0, sid: 4, wei: 3.0 }
    }
}

/// `pagroof`: pagoda roof with upturned corners.
fn pagroof(cx: &mut Cx, xoff: f64, yoff: f64, a: PagroofArgs) -> Canv {
    let PagroofArgs { hei, wid, per, cor, sid, wei } = a;
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    let mut polist: Vec<Pt> = vec![[0.0, -hei]];
    for i in 0..sid {
        let t = i as f64 / (sid - 1) as f64 - 0.5;
        let fx = wid * t;
        let fy = per * (1.0 - t.abs() * 2.0);
        let fxx = (wid + cor) * t;
        if i > 0 {
            let prev = ptlist[ptlist.len() - 1][2];
            ptlist.push(vec![prev, [fxx, fy]]);
        }
        ptlist.push(vec![[0.0, -hei], [fx * 0.5, (-hei + fy) * 0.5], [fxx, fy]]);
        polist.push([fxx, fy]);
    }
    let mut canv = Vec::new();
    canv.extend(poly(
        &polist,
        PolyArgs { xof: xoff, yof: yoff, str: Some(Rgba::NONE), fil: Rgba::WHITE, ..Default::default() },
    ));
    for pl in &ptlist {
        canv.extend(flat_stroke(cx, &div(pl, 5.0), xoff, yoff, grey(0.4), 1.0, wei));
    }
    canv
}

pub struct Arch01Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64 }
impl Default for Arch01Args {
    fn default() -> Self { Arch01Args { hei: 70.0, wid: 180.0, rot: 0.7, per: 5.0 } }
}
/// `Arch.arch01`: thatched hut / pavilion with people.
pub fn arch01(cx: &mut Cx, xoff: f64, yoff: f64, seed: f64, a: Arch01Args) -> Canv {
    let Arch01Args { hei, wid, per, .. } = a;
    let p = 0.4 + cx.random() * 0.2;
    let h0 = hei * p;
    let h1 = hei * (1.0 - p);

    let mut canv = Vec::new();
    canv.extend(hut(cx, xoff, yoff - hei, h0, wid, 300));
    canv.extend(box_(
        cx,
        xoff,
        yoff,
        BoxArgs { hei: h1, wid: wid * 2.0 / 3.0, per, bot: false, ..Default::default() },
    ));
    let seg = (3.0 + cx.random() * 3.0).floor();
    canv.extend(rail(
        cx,
        xoff,
        yoff,
        seed,
        RailArgs { tra: true, fro: false, hei: 10.0, wid, per: per * 2.0, seg, ..Default::default() },
    ));
    let mcnt = cx.rand_choice(&[0, 1, 1, 2]);
    if mcnt == 1 {
        let x = xoff + cx.norm_rand(-wid / 3.0, wid / 3.0);
        let fli = cx.rand_choice(&[true, false]);
        canv.extend(man::man(cx, x, yoff, ManArgs { fli, sca: 0.42, ..Default::default() }));
    } else if mcnt == 2 {
        let x = xoff + cx.norm_rand(-wid / 4.0, -wid / 5.0);
        canv.extend(man::man(cx, x, yoff, ManArgs { fli: false, sca: 0.42, ..Default::default() }));
        let x = xoff + cx.norm_rand(wid / 5.0, wid / 4.0);
        canv.extend(man::man(cx, x, yoff, ManArgs { fli: true, sca: 0.42, ..Default::default() }));
    }
    let seg = (3.0 + cx.random() * 3.0).floor();
    canv.extend(rail(
        cx,
        xoff,
        yoff,
        seed,
        RailArgs { tra: false, fro: true, hei: 10.0, wid, per: per * 2.0, seg, ..Default::default() },
    ));
    canv
}

pub struct Arch02Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64, pub sty: f64, pub rai: bool }
impl Default for Arch02Args {
    fn default() -> Self { Arch02Args { hei: 10.0, wid: 50.0, rot: 0.3, per: 5.0, sto: 3.0, sty: 1.0, rai: false } }
}
/// `Arch.arch02`: stacked house with hip roofs and facade lattice.
pub fn arch02(cx: &mut Cx, xoff: f64, yoff: f64, _seed: f64, a: Arch02Args) -> Canv {
    let Arch02Args { hei, wid, rot, per, sto, sty, rai } = a;
    let sty = sty as u32;
    let dec = match sty {
        1 | 2 => Some(Deco { style: sty, hsp: [1, 5], vsp: [1, 2] }),
        3 => Some(Deco { style: 3, hsp: [1, 4], vsp: [1, 3] }),
        _ => None,
    };
    let mut canv = Vec::new();
    let mut hoff = 0.0;
    let mut i = 0;
    while (i as f64) < sto {
        let fi = i as f64;
        canv.extend(box_(
            cx,
            xoff,
            yoff - hoff,
            BoxArgs { tra: false, hei, wid: wid * 0.85f64.powf(fi), rot, wei: 1.5, per, dec, ..Default::default() },
        ));
        if rai {
            canv.extend(rail(
                cx,
                xoff,
                yoff - hoff,
                fi * 0.2,
                RailArgs {
                    wid: wid * 0.85f64.powf(fi) * 1.1,
                    hei: hei / 2.0,
                    per,
                    rot,
                    wei: 0.5,
                    tra: false,
                    ..Default::default()
                },
            ));
        }
        if sto == 1.0 {
            // "Pizza Hut" signboard roll; text is not drawn (see `roof`).
            let _ = cx.random();
        }
        canv.extend(roof(
            cx,
            xoff,
            yoff - hoff - hei,
            RoofArgs { hei, wid: wid * 0.9f64.powf(fi), rot, wei: 1.5, per, ..Default::default() },
        ));
        hoff += hei * 1.5;
        i += 1;
    }
    canv
}

pub struct Arch03Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64 }
impl Default for Arch03Args {
    fn default() -> Self { Arch03Args { hei: 10.0, wid: 50.0, rot: 0.7, per: 5.0, sto: 7.0 } }
}
/// `Arch.arch03`: tall pagoda.
pub fn arch03(cx: &mut Cx, xoff: f64, yoff: f64, _seed: f64, a: Arch03Args) -> Canv {
    let Arch03Args { hei, wid, rot, per, sto } = a;
    let dec = Some(Deco { style: 1, hsp: [1, 4], vsp: [1, 2] });
    let mut canv = Vec::new();
    let mut hoff = 0.0;
    let mut i = 0;
    while (i as f64) < sto {
        let fi = i as f64;
        canv.extend(box_(
            cx,
            xoff,
            yoff - hoff,
            BoxArgs {
                tra: false,
                hei,
                wid: wid * 0.85f64.powf(fi),
                rot,
                wei: 1.5,
                per: per / 2.0,
                dec,
                ..Default::default()
            },
        ));
        canv.extend(rail(
            cx,
            xoff,
            yoff - hoff,
            fi * 0.2,
            RailArgs {
                seg: 5.0,
                wid: wid * 0.85f64.powf(fi) * 1.1,
                hei: hei / 2.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: false,
                ..Default::default()
            },
        ));
        canv.extend(pagroof(
            cx,
            xoff,
            yoff - hoff - hei,
            PagroofArgs { hei: hei * 1.5, wid: wid * 0.9f64.powf(fi), wei: 1.5, per, ..Default::default() },
        ));
        hoff += hei * 1.5;
        i += 1;
    }
    canv
}

pub struct Arch04Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64 }
impl Default for Arch04Args {
    fn default() -> Self { Arch04Args { hei: 15.0, wid: 30.0, rot: 0.7, per: 5.0, sto: 2.0 } }
}
/// `Arch.arch04`: small open pavilion (transparent walls, pagoda roof).
pub fn arch04(cx: &mut Cx, xoff: f64, yoff: f64, _seed: f64, a: Arch04Args) -> Canv {
    let Arch04Args { hei, wid, rot, per, sto } = a;
    let mut canv = Vec::new();
    let mut hoff = 0.0;
    let mut i = 0;
    while (i as f64) < sto {
        let fi = i as f64;
        canv.extend(box_(
            cx,
            xoff,
            yoff - hoff,
            BoxArgs {
                tra: true,
                hei,
                wid: wid * 0.85f64.powf(fi),
                rot,
                wei: 1.5,
                per: per / 2.0,
                dec: None,
                ..Default::default()
            },
        ));
        canv.extend(rail(
            cx,
            xoff,
            yoff - hoff,
            fi * 0.2,
            RailArgs {
                seg: 3.0,
                wid: wid * 0.85f64.powf(fi) * 1.2,
                hei: hei / 3.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: true,
                ..Default::default()
            },
        ));
        canv.extend(pagroof(
            cx,
            xoff,
            yoff - hoff - hei,
            PagroofArgs { hei, wid: wid * 0.9f64.powf(fi), wei: 1.5, per, ..Default::default() },
        ));
        hoff += hei * 1.2;
        i += 1;
    }
    canv
}

pub struct Boat01Args { pub len: f64, pub sca: f64, pub fli: bool }
impl Default for Boat01Args {
    fn default() -> Self { Boat01Args { len: 120.0, sca: 1.0, fli: false } }
}
/// `Arch.boat01`: a skiff with a punting boatman in a straw hat.
pub fn boat01(cx: &mut Cx, xoff: f64, yoff: f64, _seed: f64, a: Boat01Args) -> Canv {
    let Boat01Args { len, sca, fli } = a;
    let mut canv = Vec::new();
    let dir = if fli { -1.0 } else { 1.0 };
    canv.extend(man::man(
        cx,
        xoff + 20.0 * sca * dir,
        yoff,
        ManArgs {
            ite: Item::Stick01,
            hat: Hat::Hat02,
            sca: 0.5 * sca,
            fli: !fli,
            len: [0.0, 30.0, 20.0, 30.0, 10.0, 30.0, 30.0, 30.0, 30.0],
            ang: None,
        },
    ));

    let fun1 = |x: f64| (x * PI).sin().powf(0.5) * 7.0 * sca;
    let fun2 = |x: f64| (x * PI).sin().powf(0.5) * 10.0 * sca;
    let mut plist1 = Vec::new();
    let mut plist2 = Vec::new();
    let step = 5.0 * sca;
    if step > 0.0 {
        let mut i = 0.0;
        while i < len * sca {
            plist1.push([i * dir, fun1(i / len)]);
            plist2.push([i * dir, fun2(i / len)]);
            i += step;
        }
    }
    let mut plist = plist1;
    plist.extend(plist2.into_iter().rev());
    un_nan(&mut plist);
    canv.extend(poly(&plist, PolyArgs { xof: xoff, yof: yoff, fil: Rgba::WHITE, ..Default::default() }));
    let fun = |x: f64| (x * PI * 2.0).sin();
    canv.extend(stroke(
        cx,
        &shift(&plist, xoff, yoff),
        StrokeArgs { wid: 1.0, fun: Some(&fun), col: grey(0.4), ..Default::default() },
    ));
    canv
}

pub struct TransmissionTower01Args { pub hei: f64, pub wid: f64 }
impl Default for TransmissionTower01Args {
    fn default() -> Self { TransmissionTower01Args { hei: 100.0, wid: 20.0 } }
}
/// `Arch.transmissionTower01`: lattice pylon.
pub fn transmission_tower01(cx: &mut Cx, xoff: f64, yoff: f64, _seed: f64, a: TransmissionTower01Args) -> Canv {
    let TransmissionTower01Args { hei, wid } = a;
    let mut canv = Vec::new();
    let half = |_: f64| 0.5;
    let mut quickstroke = |cx: &mut Cx, pl: &[Pt]| {
        let pts = shift(&div(pl, 5.0), xoff, yoff);
        canv.extend(stroke(
            cx,
            &pts,
            StrokeArgs { wid: 1.0, fun: Some(&half), col: grey(0.4), ..Default::default() },
        ));
    };

    let p00 = [-wid * 0.05, -hei];
    let p01 = [wid * 0.05, -hei];
    let p10 = [-wid * 0.1, -hei * 0.9];
    let p11 = [wid * 0.1, -hei * 0.9];
    let p20 = [-wid * 0.2, -hei * 0.5];
    let p21 = [wid * 0.2, -hei * 0.5];
    let p30 = [-wid * 0.5, 0.0];
    let p31 = [wid * 0.5, 0.0];

    let bch = [[0.7, -0.85], [1.0, -0.675], [0.7, -0.5]];
    for b in bch {
        quickstroke(cx, &[[-b[0] * wid, b[1] * hei], [b[0] * wid, b[1] * hei]]);
        quickstroke(cx, &[[-b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]]);
        quickstroke(cx, &[[b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]]);
        quickstroke(cx, &[[-b[0] * wid, b[1] * hei], [-b[0] * wid, (b[1] + 0.1) * hei]]);
        quickstroke(cx, &[[b[0] * wid, b[1] * hei], [b[0] * wid, (b[1] + 0.1) * hei]]);
    }

    let l10 = div(&[p00, p10, p20, p30], 5.0);
    let l11 = div(&[p01, p11, p21, p31], 5.0);
    for i in 0..l10.len() - 1 {
        quickstroke(cx, &[l10[i], l11[i + 1]]);
        quickstroke(cx, &[l11[i], l10[i + 1]]);
    }

    quickstroke(cx, &[p00, p01]);
    quickstroke(cx, &[p10, p11]);
    quickstroke(cx, &[p20, p21]);
    quickstroke(cx, &[p00, p10, p20, p30]);
    quickstroke(cx, &[p01, p11, p21, p31]);
    canv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::Noise;

    fn finite(c: &Canv) -> bool {
        c.iter().all(|s| s.pts.iter().all(|p| p[0].is_finite() && p[1].is_finite()))
    }

    fn all_kinds(cx: &mut Cx, x: f64) -> Vec<Canv> {
        vec![
            arch01(cx, x, 500.0, 0.0, Default::default()),
            arch02(cx, x + 200.0, 500.0, 0.0, Default::default()),
            arch02(cx, x + 300.0, 500.0, 0.0, Arch02Args { sty: 2.0, rot: 0.8, rai: true, ..Default::default() }),
            arch02(cx, x + 400.0, 500.0, 0.0, Arch02Args { sty: 3.0, sto: 1.0, ..Default::default() }),
            arch03(cx, x + 500.0, 500.0, 0.0, Default::default()),
            arch04(cx, x + 600.0, 500.0, 0.0, Default::default()),
            boat01(cx, x + 700.0, 500.0, 0.0, Boat01Args { sca: 0.6, ..Default::default() }),
            transmission_tower01(cx, x + 900.0, 500.0, 0.0, Default::default()),
        ]
    }

    #[test]
    fn every_kind_draws_finite_shapes() {
        let nz = Noise::new(3);
        for seed in 0..20 {
            let mut cx = Cx::new(&nz, seed);
            for (k, c) in all_kinds(&mut cx, 0.0).iter().enumerate() {
                assert!(!c.is_empty(), "kind {k} empty");
                assert!(finite(c), "kind {k} has non-finite points (seed {seed})");
            }
        }
    }

    #[test]
    #[ignore]
    fn preview_arch_png() {
        let nz = Noise::new(11);
        let mut canv = Vec::new();
        for row in 0..2 {
            let mut cx = Cx::new(&nz, 100 + row);
            let y = 330.0 + row as f64 * 250.0;
            canv.extend(arch01(&mut cx, 120.0, y, 0.0, Default::default()));
            canv.extend(arch02(&mut cx, 330.0, y, 0.0, Arch02Args { sty: 1.0 + row as f64, rot: 0.3 + 0.5 * row as f64, ..Default::default() }));
            canv.extend(arch02(&mut cx, 440.0, y, 0.0, Arch02Args { sty: 3.0, wid: 60.0, sto: 2.0, rai: true, rot: 0.7, ..Default::default() }));
            canv.extend(arch03(&mut cx, 560.0, y, 0.0, Arch03Args { wid: 50.0, sto: 5.0 + 2.0 * row as f64, ..Default::default() }));
            canv.extend(arch04(&mut cx, 670.0, y, 0.0, Arch04Args { sto: 1.0 + row as f64, ..Default::default() }));
            canv.extend(boat01(&mut cx, 740.0, y, 0.0, Boat01Args { sca: y / 800.0, fli: row == 1, ..Default::default() }));
            canv.extend(transmission_tower01(&mut cx, 950.0, y, 0.0, Default::default()));
        }
        let dir = std::env::temp_dir();
        let out = dir.join("shanshui-arch.png");
        crate::paint::preview_png(&canv, 0.0, 1100, 700, out.to_str().unwrap()).unwrap();
        // 4x zoom on the middle of the row (world x 280..830).
        let zoom = dir.join("shanshui-arch_zoom.png");
        crate::paint::preview_png(&canv, 280.0, 2200, 2800, zoom.to_str().unwrap()).unwrap();
    }
}
