//! People, hats and sticks (`Man.*` in the original).
//!
//! `man(cx, xoff, yoff, ManArgs)` draws a small robed figure standing on
//! `(xoff, yoff)`. The original's function-valued `hat` / `ite` args are the
//! [`Hat`] and [`Item`] enums.

use crate::brush::{StrokeArgs, stroke};
use crate::geom::{Canv, PolyArgs, Pt, Rgba, bezmh, distance, poly, rgba};
use crate::rng::Cx;
use std::f64::consts::PI;

/// Headwear (`Man.hat01`, `Man.hat02`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hat {
    None,
    Hat01,
    Hat02,
}

/// Item held between the hands (`ite`): nothing, or `Man.stick01`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    None,
    Stick01,
}

pub struct ManArgs {
    pub sca: f64,
    pub hat: Hat,
    pub ite: Item,
    pub fli: bool,
    /// joint angles; `None` = the original's randomised pose
    pub ang: Option<[f64; 9]>,
    pub len: [f64; 9],
}

impl Default for ManArgs {
    fn default() -> Self {
        ManArgs {
            sca: 0.5,
            hat: Hat::Hat01,
            ite: Item::None,
            fli: true,
            ang: None,
            len: [0.0, 30.0, 20.0, 30.0, 30.0, 30.0, 30.0, 30.0, 30.0],
        }
    }
}

/// Offset a polyline on both sides by `wfun(t)`; returns the two edges.
fn expand(cx: &mut Cx, ptlist: &[Pt], wfun: &dyn Fn(f64) -> f64) -> (Vec<Pt>, Vec<Pt>) {
    let mut v0 = Vec::new();
    let mut v1 = Vec::new();
    let _n0 = cx.random() * 10.0;
    let n = ptlist.len();
    if n < 2 {
        return (v0, v1);
    }
    for i in 1..n - 1 {
        let w = wfun(i as f64 / n as f64);
        let a1 = (ptlist[i][1] - ptlist[i - 1][1]).atan2(ptlist[i][0] - ptlist[i - 1][0]);
        let a2 = (ptlist[i][1] - ptlist[i + 1][1]).atan2(ptlist[i][0] - ptlist[i + 1][0]);
        let mut a = (a1 + a2) / 2.0;
        if a < a2 {
            a += PI;
        }
        v0.push([ptlist[i][0] + w * a.cos(), ptlist[i][1] + w * a.sin()]);
        v1.push([ptlist[i][0] - w * a.cos(), ptlist[i][1] - w * a.sin()]);
    }
    let l = n - 1;
    let a0 = (ptlist[1][1] - ptlist[0][1]).atan2(ptlist[1][0] - ptlist[0][0]) - PI / 2.0;
    let a1 = (ptlist[l][1] - ptlist[l - 1][1]).atan2(ptlist[l][0] - ptlist[l - 1][0]) - PI / 2.0;
    let w0 = wfun(0.0);
    let w1 = wfun(1.0);
    v0.insert(0, [ptlist[0][0] + w0 * a0.cos(), ptlist[0][1] + w0 * a0.sin()]);
    v1.insert(0, [ptlist[0][0] - w0 * a0.cos(), ptlist[0][1] - w0 * a0.sin()]);
    v0.push([ptlist[l][0] + w1 * a1.cos(), ptlist[l][1] + w1 * a1.sin()]);
    v1.push([ptlist[l][0] - w1 * a1.cos(), ptlist[l][1] - w1 * a1.sin()]);
    (v0, v1)
}

/// Map a unit-space shape onto the segment p0 -> p1 (x mirrored, as in the original).
fn tranpoly(p0: Pt, p1: Pt, ptlist: &[Pt]) -> Vec<Pt> {
    let ang = (p1[1] - p0[1]).atan2(p1[0] - p0[0]) - PI / 2.0;
    let scl = distance(p0, p1);
    ptlist
        .iter()
        .map(|v| {
            let v = [-v[0], v[1]];
            let d = distance(v, [0.0, 0.0]);
            let a = v[1].atan2(v[0]);
            [p0[0] + d * scl * (ang + a).cos(), p0[1] + d * scl * (ang + a).sin()]
        })
        .collect()
}

fn flipper(plist: &[Pt], fli: bool) -> Vec<Pt> {
    if fli { plist.iter().map(|v| [-v[0], v[1]]).collect() } else { plist.to_vec() }
}

/// `Man.hat01`: a hood with a trailing ribbon.
pub fn hat01(cx: &mut Cx, p0: Pt, p1: Pt, fli: bool) -> Canv {
    let mut canv = Vec::new();
    let seed = cx.random();
    let shape = [
        [-0.3, 0.5],
        [0.3, 0.8],
        [0.2, 1.0],
        [0.0, 1.1],
        [-0.3, 1.15],
        [-0.55, 1.0],
        [-0.65, 0.5],
    ];
    canv.extend(poly(
        &tranpoly(p0, p1, &flipper(&shape, fli)),
        PolyArgs { fil: rgba(100.0, 100.0, 100.0, 0.8), ..Default::default() },
    ));
    let qlist1: Vec<Pt> = (0..10)
        .map(|i| {
            let i = i as f64;
            [-0.3 - cx.noise(i * 0.2, seed, 0.0) * i * 0.1, 0.5 - i * 0.3]
        })
        .collect();
    canv.extend(poly(
        &tranpoly(p0, p1, &flipper(&qlist1, fli)),
        PolyArgs { str: Some(rgba(100.0, 100.0, 100.0, 0.8)), wid: 1.0, ..Default::default() },
    ));
    canv
}

/// `Man.hat02`: a wide conical straw hat.
pub fn hat02(cx: &mut Cx, p0: Pt, p1: Pt, fli: bool) -> Canv {
    let _seed = cx.random();
    let shape = [
        [-0.3, 0.5],
        [-1.1, 0.5],
        [-1.2, 0.6],
        [-1.1, 0.7],
        [-0.3, 0.8],
        [0.3, 0.8],
        [1.0, 0.7],
        [1.3, 0.6],
        [1.2, 0.5],
        [0.3, 0.5],
    ];
    poly(
        &tranpoly(p0, p1, &flipper(&shape, fli)),
        PolyArgs { fil: rgba(100.0, 100.0, 100.0, 0.8), ..Default::default() },
    )
}

/// `Man.stick01`: a slightly bent pole (boat punting stick).
pub fn stick01(cx: &mut Cx, p0: Pt, p1: Pt, fli: bool) -> Canv {
    let seed = cx.random();
    let l = 12;
    let qlist1: Vec<Pt> = (0..l)
        .map(|i| {
            let fi = i as f64;
            [
                -cx.noise(fi * 0.1, seed, 0.0) * 0.1 * (fi / l as f64 * PI).sin() * 5.0,
                fi * 0.3,
            ]
        })
        .collect();
    poly(
        &tranpoly(p0, p1, &flipper(&qlist1, fli)),
        PolyArgs { str: Some(rgba(100.0, 100.0, 100.0, 0.5)), wid: 1.0, ..Default::default() },
    )
}

//      2
//    1/
// 7/  | \_ 6
// 8| 0 \ 5
//      /3
//     4
/// Parent of each joint in the original's skeleton tree
/// `{0: {1: {2, 5: {6}, 7: {8}}, 3: {4}}}`.
const PARENT: [usize; 9] = [usize::MAX, 0, 1, 0, 3, 1, 5, 1, 7];

/// Root-to-joint path (`gpar`).
fn gpar(ind: usize) -> Vec<usize> {
    let mut path = vec![ind];
    let mut i = ind;
    while PARENT[i] != usize::MAX {
        i = PARENT[i];
        path.push(i);
    }
    path.reverse();
    path
}

/// `Man.man(xoff, yoff, args)`
pub fn man(cx: &mut Cx, xoff: f64, yoff: f64, a: ManArgs) -> Canv {
    let sca = a.sca;
    let fli = a.fli;
    let ang = match a.ang {
        Some(ang) => ang,
        None => {
            let a2 = cx.norm_rand(0.0, 0.0);
            let a3 = PI / 4.0 * cx.random();
            let a4 = PI * 3.0 / 4.0 * cx.random();
            let a7 = -PI * 3.0 / 4.0 - PI / 4.0 * cx.random();
            [0.0, -PI / 2.0, a2, a3, a4, PI * 3.0 / 4.0, -PI / 4.0, a7, -PI / 4.0]
        }
    };
    let len: Vec<f64> = a.len.iter().map(|v| v * sca).collect();

    let grot = |ind: usize| gpar(ind).iter().map(|&p| ang[p]).sum::<f64>();
    let gpos = |ind: usize| {
        let mut pos = [0.0, 0.0];
        for p in gpar(ind) {
            let r = grot(p);
            pos[0] += len[p] * r.cos();
            pos[1] += len[p] * r.sin();
        }
        pos
    };
    let pts: Vec<Pt> = (0..ang.len()).map(gpos).collect();
    let yoff = yoff - pts[4][1];
    let sx = if fli { -1.0 } else { 1.0 };
    let to_global = move |v: &Pt| -> Pt { [sx * v[0] + xoff, v[1] + yoff] };

    let cloth = |cx: &mut Cx, plist: &[Pt], fun: &dyn Fn(f64) -> f64| -> Canv {
        let mut canv = Vec::new();
        let tlist = bezmh(plist, 2.0);
        let (t1, t2) = expand(cx, &tlist, fun);
        let outline: Vec<Pt> = t1.iter().chain(t2.iter().rev()).map(to_global).collect();
        canv.extend(poly(&outline, PolyArgs { fil: Rgba::WHITE, ..Default::default() }));
        let g1: Vec<Pt> = t1.iter().map(to_global).collect();
        let g2: Vec<Pt> = t2.iter().map(to_global).collect();
        canv.extend(stroke(
            cx,
            &g1,
            StrokeArgs { wid: 1.0, col: rgba(100.0, 100.0, 100.0, 0.5), ..Default::default() },
        ));
        canv.extend(stroke(
            cx,
            &g2,
            StrokeArgs { wid: 1.0, col: rgba(100.0, 100.0, 100.0, 0.6), ..Default::default() },
        ));
        canv
    };

    let fsleeve = |x: f64| {
        sca * 8.0 * ((0.5 * x * PI).sin() * (x * PI).sin().powf(0.1) + (1.0 - x) * 0.4)
    };
    let fbody = |x: f64| {
        sca * 11.0 * ((0.5 * x * PI).sin() * (x * PI).sin().powf(0.1) + (1.0 - x) * 0.5)
    };
    let fhead = |x: f64| sca * 7.0 * (0.25 - (x - 0.5).powi(2)).powf(0.3);

    let mut canv = Vec::new();
    match a.ite {
        Item::None => {}
        Item::Stick01 => canv.extend(stick01(cx, to_global(&pts[8]), to_global(&pts[6]), fli)),
    }

    canv.extend(cloth(cx, &[pts[1], pts[7], pts[8]], &fsleeve));
    canv.extend(cloth(cx, &[pts[1], pts[0], pts[3], pts[4]], &fbody));
    canv.extend(cloth(cx, &[pts[1], pts[5], pts[6]], &fsleeve));
    canv.extend(cloth(cx, &[pts[1], pts[2]], &fhead));

    let hlist = bezmh(&[pts[1], pts[2]], 2.0);
    let (mut h1, mut h2) = expand(cx, &hlist, &fhead);
    let n1 = (h1.len() as f64 * 0.1).floor() as usize;
    h1.drain(..n1);
    let n2 = (h2.len() as f64 * 0.95).floor() as usize;
    h2.drain(..n2);
    let hair: Vec<Pt> = h1.iter().chain(h2.iter().rev()).map(to_global).collect();
    canv.extend(poly(&hair, PolyArgs { fil: rgba(100.0, 100.0, 100.0, 0.6), ..Default::default() }));

    let (h0, h1) = (to_global(&pts[1]), to_global(&pts[2]));
    match a.hat {
        Hat::None => {}
        Hat::Hat01 => canv.extend(hat01(cx, h0, h1, fli)),
        Hat::Hat02 => canv.extend(hat02(cx, h0, h1, fli)),
    }
    canv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::Noise;

    #[test]
    fn man_is_finite_and_nonempty() {
        let nz = Noise::new(1);
        let mut cx = Cx::new(&nz, 7);
        for hat in [Hat::Hat01, Hat::Hat02, Hat::None] {
            let c = man(&mut cx, 100.0, 500.0, ManArgs { hat, ite: Item::Stick01, ..Default::default() });
            assert!(c.len() >= 13);
            assert!(c.iter().all(|s| s.pts.iter().all(|p| p[0].is_finite() && p[1].is_finite())));
        }
    }
}
