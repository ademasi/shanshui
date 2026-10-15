//! Display list and geometry helpers (`poly`, `PolyTools`, `Util` in the original).
//!
//! The original builds SVG strings and concatenates them; here every drawing
//! function returns a `Canv` (a `Vec<Shape>`) and concatenation becomes
//! `extend`. Order is paint order, later shapes on top.

pub type Pt = [f64; 2];

/// Colour as in the original's `rgba(r,g,b,a)` strings (0..255, alpha 0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const NONE: Rgba = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const WHITE: Rgba = Rgba { r: 255.0, g: 255.0, b: 255.0, a: 1.0 };
    pub const BLACK: Rgba = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    pub fn visible(&self) -> bool {
        self.a > 0.0
    }
}

/// `"rgba(r,g,b,a)"`
pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Rgba {
    Rgba { r: r as f32, g: g as f32, b: b as f32, a: a.clamp(0.0, 1.0) as f32 }
}

/// `"rgb(r,g,b)"`
pub fn rgb(r: f64, g: f64, b: f64) -> Rgba {
    rgba(r, g, b, 1.0)
}

/// One SVG `<polyline>`: filled (implicitly closed) and stroked (open).
#[derive(Clone, Debug)]
pub struct Shape {
    pub pts: Vec<Pt>,
    pub fill: Rgba,
    pub stroke: Rgba,
    pub width: f64,
}

pub type Canv = Vec<Shape>;

pub struct PolyArgs {
    pub xof: f64,
    pub yof: f64,
    /// fill; default transparent (`"none"` in the original is also `Rgba::NONE`)
    pub fil: Rgba,
    /// stroke colour; `None` means "same as fil"
    pub str: Option<Rgba>,
    pub wid: f64,
}

impl Default for PolyArgs {
    fn default() -> Self {
        PolyArgs { xof: 0.0, yof: 0.0, fil: Rgba::NONE, str: None, wid: 0.0 }
    }
}

/// `poly(plist, args)`
pub fn poly(plist: &[Pt], a: PolyArgs) -> Canv {
    if plist.is_empty() {
        return Vec::new();
    }
    let pts = plist.iter().map(|p| [p[0] + a.xof, p[1] + a.yof]).collect();
    vec![Shape { pts, fill: a.fil, stroke: a.str.unwrap_or(a.fil), width: a.wid }]
}

/// `PolyTools.midPt(...)`
pub fn mid_pt(plist: &[Pt]) -> Pt {
    let n = plist.len() as f64;
    plist.iter().fold([0.0, 0.0], |acc, v| [v[0] / n + acc[0], v[1] / n + acc[1]])
}

/// `distance(p0, p1)`
pub fn distance(p0: Pt, p1: Pt) -> f64 {
    ((p0[0] - p1[0]).powi(2) + (p0[1] - p1[1]).powi(2)).sqrt()
}

/// `unNan`: NaN/inf coordinates become 0.
pub fn un_nan(plist: &mut [Pt]) {
    for p in plist {
        for v in p.iter_mut() {
            if !v.is_finite() {
                *v = 0.0;
            }
        }
    }
}

/// `loopNoise(nslist)`: in place.
pub fn loop_noise(nslist: &mut [f64]) {
    let n = nslist.len();
    if n < 2 {
        return;
    }
    let dif = nslist[n - 1] - nslist[0];
    let mut bds = [100.0f64, -100.0];
    for i in 0..n {
        nslist[i] += dif * (n - 1 - i) as f64 / (n - 1) as f64;
        bds[0] = bds[0].min(nslist[i]);
        bds[1] = bds[1].max(nslist[i]);
    }
    for v in nslist.iter_mut() {
        *v = crate::rng::mapval(*v, bds[0], bds[1], 0.0, 1.0);
    }
}

/// `bezmh(P, w)`: smooth curve through control points.
pub fn bezmh(p: &[Pt], w: f64) -> Vec<Pt> {
    let owned;
    let p = if p.len() == 2 {
        owned = vec![p[0], mid_pt(&[p[0], p[1]]), p[1]];
        &owned[..]
    } else {
        p
    };
    let mut plist = Vec::new();
    if p.len() < 3 {
        return plist;
    }
    for j in 0..p.len() - 2 {
        let p0 = if j == 0 { p[j] } else { mid_pt(&[p[j], p[j + 1]]) };
        let p1 = p[j + 1];
        let p2 = if j == p.len() - 3 { p[j + 2] } else { mid_pt(&[p[j + 1], p[j + 2]]) };
        let pl = 20;
        let last = (j == p.len() - 3) as usize;
        for i in 0..pl + last {
            let t = i as f64 / pl as f64;
            let u = (1.0 - t).powi(2) + 2.0 * t * (1.0 - t) * w + t * t;
            plist.push([
                ((1.0 - t).powi(2) * p0[0] + 2.0 * t * (1.0 - t) * p1[0] * w + t * t * p2[0]) / u,
                ((1.0 - t).powi(2) * p0[1] + 2.0 * t * (1.0 - t) * p1[1] * w + t * t * p2[1]) / u,
            ]);
        }
    }
    plist
}

/// `div(plist, reso)`: subdivide a polyline.
pub fn div(plist: &[Pt], reso: f64) -> Vec<Pt> {
    let mut rlist = Vec::new();
    if plist.is_empty() {
        return rlist;
    }
    let tl = (plist.len() - 1) as f64 * reso;
    let mut i = 0.0;
    while i < tl {
        let lastp = plist[(i / reso).floor() as usize];
        let nextp = plist[((i / reso).ceil() as usize).min(plist.len() - 1)];
        let p = (i % reso) / reso;
        rlist.push([lastp[0] * (1.0 - p) + nextp[0] * p, lastp[1] * (1.0 - p) + nextp[1] * p]);
        i += 1.0;
    }
    rlist.push(plist[plist.len() - 1]);
    rlist
}

/// `PolyTools.triangulate(plist, {area, convex, optimize})`
pub fn triangulate(plist: &[Pt], area: f64, convex: bool, optimize: bool) -> Vec<Vec<Pt>> {
    let mut out = Vec::new();
    let mut rest = plist.to_vec();
    // The original recurses on the remainder; iterate instead.
    loop {
        if rest.len() <= 3 {
            out.extend(shatter(&rest, area));
            return out;
        }
        let (ear, remainder) = best_ear(&rest, convex, optimize);
        if remainder.is_empty() {
            out.extend(shatter(&ear, area));
            return out;
        }
        out.extend(shatter(&ear, area));
        rest = remainder;
    }
}

fn line_expr(p0: Pt, p1: Pt) -> (f64, f64) {
    let den = p1[0] - p0[0];
    let m = if den == 0.0 { f64::INFINITY } else { (p1[1] - p0[1]) / den };
    (m, p0[1] - m * p0[0])
}

fn intersect(ln0: [Pt; 2], ln1: [Pt; 2]) -> Option<Pt> {
    let le0 = line_expr(ln0[0], ln0[1]);
    let le1 = line_expr(ln1[0], ln1[1]);
    let den = le0.0 - le1.0;
    if den == 0.0 {
        return None;
    }
    let x = (le1.1 - le0.1) / den;
    let y = le0.0 * x + le0.1;
    let on_seg = |p: Pt, ln: [Pt; 2]| {
        ln[0][0].min(ln[1][0]) <= p[0]
            && p[0] <= ln[0][0].max(ln[1][0])
            && ln[0][1].min(ln[1][1]) <= p[1]
            && p[1] <= ln[0][1].max(ln[1][1])
    };
    (on_seg([x, y], ln0) && on_seg([x, y], ln1)).then_some([x, y])
}

fn pt_in_poly(pt: Pt, plist: &[Pt]) -> bool {
    let mut scount = 0;
    for i in 0..plist.len() {
        let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
        if intersect([plist[i], np], [pt, [pt[0] + 999.0, pt[1] + 999.0]]).is_some() {
            scount += 1;
        }
    }
    scount % 2 == 1
}

fn ln_in_poly(ln: [Pt; 2], plist: &[Pt]) -> bool {
    let ep = 0.01;
    let lnc = [
        [ln[0][0] * (1.0 - ep) + ln[1][0] * ep, ln[0][1] * (1.0 - ep) + ln[1][1] * ep],
        [ln[0][0] * ep + ln[1][0] * (1.0 - ep), ln[0][1] * ep + ln[1][1] * (1.0 - ep)],
    ];
    for i in 0..plist.len() {
        let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
        if intersect(lnc, [plist[i], np]).is_some() {
            return false;
        }
    }
    pt_in_poly(mid_pt(&ln), plist)
}

fn sides_of(plist: &[Pt]) -> Vec<f64> {
    (0..plist.len())
        .map(|i| distance(plist[i], plist[if i != plist.len() - 1 { i + 1 } else { 0 }]))
        .collect()
}

fn area_of(plist: &[Pt]) -> f64 {
    let s = sides_of(plist);
    let (a, b, c) = (s[0], s.get(1).copied().unwrap_or(0.0), s.get(2).copied().unwrap_or(0.0));
    let p = (a + b + c) / 2.0;
    (p * (p - a) * (p - b) * (p - c)).sqrt()
}

fn sliver_ratio(plist: &[Pt]) -> f64 {
    area_of(plist) / sides_of(plist).iter().sum::<f64>()
}

fn best_ear(plist: &[Pt], convex: bool, optimize: bool) -> (Vec<Pt>, Vec<Pt>) {
    let n = plist.len();
    let mut best = (plist.to_vec(), Vec::new());
    let mut best_ratio = 0.0;
    for i in 0..n {
        let pt = plist[i];
        let lp = plist[if i != 0 { i - 1 } else { n - 1 }];
        let np = plist[if i != n - 1 { i + 1 } else { 0 }];
        if convex || ln_in_poly([lp, np], plist) {
            let mut q = plist.to_vec();
            q.remove(i);
            let c = (vec![lp, pt, np], q);
            if !optimize {
                return c;
            }
            let r = sliver_ratio(&c.0);
            if r >= best_ratio {
                best_ratio = r;
                best = c;
            }
        }
    }
    best
}

fn shatter(plist: &[Pt], a: f64) -> Vec<Vec<Pt>> {
    if plist.is_empty() {
        return Vec::new();
    }
    let ar = area_of(plist);
    if !(ar >= a) || plist.len() < 3 {
        return vec![plist.to_vec()];
    }
    let s = sides_of(plist);
    let ind = (0..s.len()).fold(0, |im, i| if s[i] > s[im] { i } else { im });
    let nind = (ind + 1) % plist.len();
    let lind = (ind + 2) % plist.len();
    let mid = mid_pt(&[plist[ind], plist[nind]]);
    let mut out = shatter(&[plist[ind], mid, plist[lind]], a);
    out.extend(shatter(&[plist[lind], plist[nind], mid], a));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangulate_square() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let tris = triangulate(&sq, 100.0, false, true);
        assert!(!tris.is_empty());
        assert!(tris.iter().all(|t| t.len() == 3));
    }

    #[test]
    fn div_keeps_ends() {
        let d = div(&[[0.0, 0.0], [10.0, 0.0]], 5.0);
        assert_eq!(d.len(), 6);
        assert_eq!(d[0], [0.0, 0.0]);
        assert_eq!(d[5], [10.0, 0.0]);
    }
}
