//! Brush primitives: `stroke`, `blob`, `texture` from the original.

use crate::geom::{Canv, PolyArgs, Pt, Rgba, loop_noise, poly, rgba};
use crate::rng::Cx;
use std::f64::consts::PI;

pub struct StrokeArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub wid: f64,
    pub col: Rgba,
    pub noi: f64,
    pub out: f64,
    /// width profile along the stroke; default `sin(x * PI)`
    pub fun: Option<&'a dyn Fn(f64) -> f64>,
}

impl Default for StrokeArgs<'_> {
    fn default() -> Self {
        StrokeArgs {
            xof: 0.0,
            yof: 0.0,
            wid: 2.0,
            col: rgba(200.0, 200.0, 200.0, 0.9),
            noi: 0.5,
            out: 1.0,
            fun: None,
        }
    }
}

/// `stroke(ptlist, args)`: a tapered brush stroke along a polyline.
pub fn stroke(cx: &mut Cx, ptlist: &[Pt], a: StrokeArgs) -> Canv {
    if ptlist.is_empty() {
        return Vec::new();
    }
    let default_fun = |x: f64| (x * PI).sin();
    let fun: &dyn Fn(f64) -> f64 = a.fun.unwrap_or(&default_fun);
    let n = ptlist.len();
    let mut v0 = Vec::with_capacity(n);
    let mut v1 = Vec::with_capacity(n);
    let n0 = cx.random() * 10.0;
    for i in 1..n.saturating_sub(1) {
        let mut w = a.wid * fun(i as f64 / n as f64);
        w = w * (1.0 - a.noi) + w * a.noi * cx.noise(i as f64 * 0.5, n0, 0.0);
        let a1 = (ptlist[i][1] - ptlist[i - 1][1]).atan2(ptlist[i][0] - ptlist[i - 1][0]);
        let a2 = (ptlist[i][1] - ptlist[i + 1][1]).atan2(ptlist[i][0] - ptlist[i + 1][0]);
        let mut ang = (a1 + a2) / 2.0;
        if ang < a2 {
            ang += PI;
        }
        v0.push([ptlist[i][0] + w * ang.cos(), ptlist[i][1] + w * ang.sin()]);
        v1.push([ptlist[i][0] - w * ang.cos(), ptlist[i][1] - w * ang.sin()]);
    }
    // [p0] + v0 + reverse(v1 + [pN]) + [p0]
    let mut vtx = Vec::with_capacity(2 * n + 2);
    vtx.push(ptlist[0]);
    vtx.extend(v0);
    vtx.push(ptlist[n - 1]);
    vtx.extend(v1.into_iter().rev());
    vtx.push(ptlist[0]);
    poly(
        &vtx,
        PolyArgs { xof: a.xof, yof: a.yof, fil: a.col, str: Some(a.col), wid: a.out },
    )
}

pub struct BlobArgs<'a> {
    pub len: f64,
    pub wid: f64,
    pub ang: f64,
    pub col: Rgba,
    pub noi: f64,
    /// half-width profile over p in [0, 2]
    pub fun: Option<&'a dyn Fn(f64) -> f64>,
}

impl Default for BlobArgs<'_> {
    fn default() -> Self {
        BlobArgs {
            len: 20.0,
            wid: 5.0,
            ang: 0.0,
            col: rgba(200.0, 200.0, 200.0, 0.9),
            noi: 0.5,
            fun: None,
        }
    }
}

/// `blob(x, y, {ret: 1, ...})`: the blob outline.
pub fn blob_pts(cx: &mut Cx, x: f64, y: f64, a: &BlobArgs) -> Vec<Pt> {
    let default_fun = |x: f64| {
        if x <= 1.0 {
            (x * PI).sin().powf(0.5)
        } else {
            -((x + 1.0) * PI).sin().powf(0.5)
        }
    };
    let fun: &dyn Fn(f64) -> f64 = a.fun.unwrap_or(&default_fun);
    let reso = 20usize;
    let mut lalist = Vec::with_capacity(reso + 1);
    for i in 0..=reso {
        let p = i as f64 / reso as f64 * 2.0;
        let xo = a.len / 2.0 - (p - 1.0).abs() * a.len;
        let yo = fun(p) * a.wid / 2.0;
        lalist.push(((xo * xo + yo * yo).sqrt(), yo.atan2(xo)));
    }
    let n0 = cx.random() * 10.0;
    let mut nslist: Vec<f64> = (0..=reso).map(|i| cx.noise(i as f64 * 0.05, n0, 0.0)).collect();
    loop_noise(&mut nslist);
    lalist
        .iter()
        .zip(&nslist)
        .map(|(&(l, ang), &ns)| {
            let ns = ns * a.noi + (1.0 - a.noi);
            [x + (ang + a.ang).cos() * l * ns, y + (ang + a.ang).sin() * l * ns]
        })
        .collect()
}

/// `blob(x, y, args)`: a filled ink blob (leaves, dots).
pub fn blob(cx: &mut Cx, x: f64, y: f64, a: BlobArgs) -> Canv {
    let plist = blob_pts(cx, x, y, &a);
    poly(&plist, PolyArgs { fil: a.col, str: Some(a.col), wid: 0.0, ..Default::default() })
}

pub struct TextureArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub tex: usize,
    pub wid: f64,
    pub len: f64,
    pub sha: f64,
    /// noise amplitude per layer; default `30 / x`
    pub noi: Option<&'a dyn Fn(f64) -> f64>,
    /// colour per stroke index fraction; default grey with random alpha < 0.3
    pub col: Option<&'a dyn Fn(&mut Cx, f64) -> Rgba>,
    /// distribution of stroke centres; default biased to the outer thirds
    pub dis: Option<&'a dyn Fn(&mut Cx) -> f64>,
}

impl Default for TextureArgs<'_> {
    fn default() -> Self {
        TextureArgs {
            xof: 0.0,
            yof: 0.0,
            tex: 400,
            wid: 1.5,
            len: 0.2,
            sha: 0.0,
            noi: None,
            col: None,
            dis: None,
        }
    }
}

fn default_tex_col(cx: &mut Cx, _x: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, cx.random() * 0.3)
}

fn default_tex_dis(cx: &mut Cx) -> f64 {
    if cx.random() > 0.5 {
        (1.0 / 3.0) * cx.random()
    } else {
        2.0 / 3.0 + (1.0 / 3.0) * cx.random()
    }
}

/// `texture(ptlist, {ret: 1, ...})`: the texture polylines without drawing them.
/// `ptlist` is a grid of layers, each layer a polyline of equal length.
pub fn texture_list(cx: &mut Cx, ptlist: &[Vec<Pt>], a: &TextureArgs) -> Vec<Vec<Pt>> {
    if ptlist.is_empty() || ptlist[0].is_empty() {
        return Vec::new();
    }
    let default_noi = |x: f64| 30.0 / x;
    let noi: &dyn Fn(f64) -> f64 = a.noi.unwrap_or(&default_noi);
    let dis: &dyn Fn(&mut Cx) -> f64 = a.dis.unwrap_or(&default_tex_dis);
    let reso = [ptlist.len(), ptlist[0].len()];
    let mut texlist = Vec::with_capacity(a.tex);
    for i in 0..a.tex {
        let mid = (dis(cx) * reso[1] as f64) as i64;
        let hlen = (cx.random() * (reso[1] as f64 * a.len)).floor() as i64;
        let start = (mid - hlen).clamp(0, reso[1] as i64) as usize;
        let end = (mid + hlen).clamp(0, reso[1] as i64) as usize;
        let layer = i as f64 / a.tex as f64 * (reso[0] - 1) as f64;
        let (lo, hi) = (layer.floor() as usize, (layer.ceil() as usize).min(reso[0] - 1));
        let p = layer - layer.floor();
        let mut line = Vec::with_capacity(end.saturating_sub(start));
        for j in start..end {
            let (Some(a0), Some(a1)) = (ptlist[lo].get(j), ptlist[hi].get(j)) else { continue };
            let x = a0[0] * p + a1[0] * (1.0 - p);
            let y = a0[1] * p + a1[1] * (1.0 - p);
            let amp = noi(layer + 1.0);
            let ns = [
                amp * (cx.noise(x, j as f64 * 0.5, 0.0) - 0.5),
                amp * (cx.noise(y, j as f64 * 0.5, 0.0) - 0.5),
            ];
            line.push([x + ns[0], y + ns[1]]);
        }
        texlist.push(line);
    }
    texlist
}

/// `texture(ptlist, args)`: hatching strokes across a mountain/rock grid.
pub fn texture(cx: &mut Cx, ptlist: &[Vec<Pt>], a: TextureArgs) -> Canv {
    let texlist = texture_list(cx, ptlist, &a);
    let col: &dyn Fn(&mut Cx, f64) -> Rgba = a.col.unwrap_or(&default_tex_col);
    let mut canv = Vec::new();
    let shift = |l: &Vec<Pt>| l.iter().map(|p| [p[0] + a.xof, p[1] + a.yof]).collect::<Vec<_>>();
    let sha = a.sha;
    if sha != 0.0 {
        for line in texlist.iter().step_by(2) {
            let pts = shift(line);
            canv.extend(stroke(
                cx,
                &pts,
                StrokeArgs { col: rgba(100.0, 100.0, 100.0, 0.1), wid: sha, ..Default::default() },
            ));
        }
    }
    // JS: for (j = 0 + sha; j < len; j += 1 + sha) — sha is a width but also an index step.
    let step = 1 + sha as usize;
    let mut j = sha as usize;
    let n = texlist.len();
    while j < n {
        let pts = shift(&texlist[j]);
        let c = col(cx, j as f64 / n as f64);
        canv.extend(stroke(cx, &pts, StrokeArgs { col: c, wid: a.wid, ..Default::default() }));
        j += step;
    }
    canv
}
