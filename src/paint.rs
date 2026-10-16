//! Rasterise a stretch of the world into a pixel strip and map the ink onto
//! the palette.
//!
//! The original draws grey ink onto a transparent SVG multiplied over paper,
//! which equals "draw on white, multiply by paper". We draw on white with
//! tiny-skia, read the ink amount `t = 1 - luminance`, and look the result up
//! in a palette ramp.

use crate::geom::{Canv, Rgba, Shape};
use crate::rng::mix;
use crate::world::{BAND_HEIGHT, World};
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Point,
    RadialGradient, Rect, SpreadMode, Stroke, Transform,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// `wl_shm` ARGB8888/XRGB8888 on little-endian: bytes B, G, R, A
    Bgrx,
    /// straight RGBA bytes, for PNG output
    Rgba,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKind {
    /// Catppuccin Macchiato base as paper, pale grey-blue ink
    Night,
    /// as Night, but the densest ink stops at overlay1: calmer
    NightSoft,
    /// as Night, with a lavender cast in the mid-tones
    NightTint,
    /// cream paper, dark ink, as the original
    Paper,
}

impl PaletteKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "night" => Some(Self::Night),
            "night-soft" => Some(Self::NightSoft),
            "night-tint" => Some(Self::NightTint),
            "paper" => Some(Self::Paper),
            _ => None,
        }
    }
}

pub struct Palette {
    /// colour for each ink amount 0..=255
    lut: [[u8; 3]; 256],
    /// paper grain amplitude, in LUT steps
    grain: f32,
    /// Paper only: the original's paper texture, one quadrant of its
    /// mirrored 512 px tile (`PAPER_Q` x `PAPER_Q` brightness values).
    paper: Option<Box<[u8]>>,
}

const PAPER_TILE: i64 = 512;
const PAPER_Q: usize = PAPER_TILE as usize / 2 + 1;

/// The original's background canvas: `c = 245 + noise(i/10, j/10) * 10 -
/// random * 20`, mirrored into a 512 px tile, tinted (c, .95c, .85c).
fn paper_tile() -> Box<[u8]> {
    let nz = crate::noise::Noise::new(0x7061_7065_72);
    let mut rng = crate::rng::Rng::new(0x7061_7065_72);
    let q = PAPER_Q;
    let mut c = vec![0f32; q * q];
    for i in 0..q {
        for j in 0..q {
            let v = 245.0 + nz.noise(i as f64 * 0.1, j as f64 * 0.1, 0.0) * 10.0 - rng.random() * 20.0;
            c[i * q + j] = v as f32;
        }
    }
    // Soften the per-pixel speckle with a [1 2 1] blur: the drift samples
    // between pixels, and sharp speckle would visibly "breathe" (half its
    // contrast at half-pixel offsets). The ink stays crisp.
    let at = |c: &[f32], i: isize, j: isize| {
        let f = |v: isize| v.clamp(0, q as isize - 1) as usize;
        c[f(i) * q + f(j)]
    };
    let mut t = vec![0u8; q * q];
    for i in 0..q as isize {
        for j in 0..q as isize {
            let mut sum = 0.0;
            for (di, wi) in [(-1, 1.0), (0, 2.0), (1, 1.0)] {
                for (dj, wj) in [(-1, 1.0), (0, 2.0), (1, 1.0)] {
                    sum += wi * wj * at(&c, i + di, j + dj);
                }
            }
            t[i as usize * q + j as usize] = (sum / 16.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    t.into_boxed_slice()
}

fn hex(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32]
}

fn ramp(stops: &[(f32, [f32; 3])], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            let u = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return [0, 1, 2].map(|i| c0[i] + (c1[i] - c0[i]) * u);
        }
    }
    stops[stops.len() - 1].1
}

impl Palette {
    pub fn new(kind: PaletteKind) -> Self {
        let mut lut = [[0u8; 3]; 256];
        // Catppuccin Macchiato
        let (base, surface1, surface2) = (hex(0x24273a), hex(0x494d64), hex(0x5b6078));
        let (overlay1, overlay2) = (hex(0x8087a2), hex(0x939ab7));
        let (subtext1, text, lavender) = (hex(0xb8c0e0), hex(0xcad3f5), hex(0xb7bdf8));
        let night = [
            (0.0, base),
            (0.12, surface1),
            (0.28, surface2),
            (0.5, overlay1),
            (0.72, subtext1),
            (1.0, text),
        ];
        let surface0 = hex(0x363a4f);
        let overlay0 = hex(0x6e738d);
        let soft = [
            (0.0, base),
            (0.15, surface0),
            (0.35, surface1),
            (0.55, surface2),
            (0.78, overlay0),
            (1.0, overlay1),
        ];
        let violet_grey = [0, 1, 2].map(|i| overlay2[i] * 0.6 + lavender[i] * 0.4);
        let tint = [
            (0.0, base),
            (0.12, surface1),
            (0.3, violet_grey),
            (0.62, lavender),
            (1.0, text),
        ];
        for (i, slot) in lut.iter_mut().enumerate() {
            let t = i as f32 / 255.0;
            let c = match kind {
                PaletteKind::Night => ramp(&night, t),
                PaletteKind::NightSoft => ramp(&soft, t),
                PaletteKind::NightTint => ramp(&tint, t),
                // multiply: paper * luminance
                PaletteKind::Paper => hex(0xf5e9d0).map(|p| p * (1.0 - t)),
            };
            *slot = c.map(|v| v.round().clamp(0.0, 255.0) as u8);
        }
        let grain = match kind {
            PaletteKind::Paper => 12.0,
            _ => 5.0,
        };
        let paper = (kind == PaletteKind::Paper).then(paper_tile);
        Palette { lut, grain, paper }
    }
}

/// Pixels per world unit for a strip of `height_px`.
pub fn scale_for_height(height_px: u32) -> f64 {
    height_px as f64 / BAND_HEIGHT
}

/// Outer columns of a strip whose anti-aliasing differs slightly from the
/// same world pixels painted inside a wider strip (tiny-skia clips paths at
/// the pixmap edge). Only show `[EDGE, w - EDGE)` where strips must join.
pub const EDGE: u32 = 8;

/// Paint world pixels `[x0_px, x0_px + w)` (at `scale` px per world unit)
/// into `out` (`w * h * 4` bytes). Pixel `(x, y)` of the strip shows world
/// point `((x0_px + x) / scale, y / scale)`, so strips with the same scale
/// line up exactly however they are cut, except within `EDGE` of the sides.
pub fn render_strip(
    world: &World,
    x0_px: i64,
    scale: f64,
    w: u32,
    h: u32,
    out: &mut [u8],
    fmt: PixelFormat,
    pal: &Palette,
) {
    assert_eq!(out.len(), w as usize * h as usize * 4, "strip buffer size");
    {
        let mut pm = PixmapMut::from_bytes(out, w, h).expect("non-empty strip");
        pm.fill(Color::WHITE);
        let wx0 = x0_px as f64 / scale;
        let wx1 = (x0_px + w as i64) as f64 / scale;
        for e in world.plan(wx0, wx1) {
            let canv = world.draw(&e);
            draw_canv(&mut pm, &canv, x0_px, scale, w, h);
        }
    }
    map_palette(out, x0_px, w, h, fmt, pal);
}

fn color(c: Rgba) -> Color {
    Color::from_rgba(
        (c.r / 255.0).clamp(0.0, 1.0),
        (c.g / 255.0).clamp(0.0, 1.0),
        (c.b / 255.0).clamp(0.0, 1.0),
        c.a.clamp(0.0, 1.0),
    )
    .unwrap_or(Color::TRANSPARENT)
}

/// Paint a `geom::haze`: a radial gradient on the unit circle, stretched to
/// the ellipse, with a smooth `(1 - t^2)^2` falloff.
fn draw_haze(pm: &mut PixmapMut, paint: &mut Paint, s: &Shape, x0: f64, scale: f64, w: u32, h: u32) {
    let [l, t] = s.pts[0];
    let [r, b] = s.pts[1];
    let (l, r, t, b) = (l * scale - x0, r * scale - x0, t * scale, b * scale);
    if r < 0.0 || l > w as f64 || b < 0.0 || t > h as f64 || r - l < 0.5 || b - t < 0.5 {
        return;
    }
    let stops = [0.0f32, 0.2, 0.4, 0.6, 0.8, 1.0]
        .map(|p| {
            let a = (1.0 - p * p).powi(2);
            GradientStop::new(p, color(Rgba { a: s.fill.a * a, ..s.fill }))
        })
        .to_vec();
    let (rx, ry) = ((r - l) / 2.0, (b - t) / 2.0);
    let tr = Transform::from_row(rx as f32, 0.0, 0.0, ry as f32, (l + rx) as f32, (t + ry) as f32);
    let origin = Point::from_xy(0.0, 0.0);
    let Some(shader) = RadialGradient::new(origin, 0.0, origin, 1.0, stops, SpreadMode::Pad, tr) else {
        return;
    };
    // Clip the box to the strip (plus a margin), so f32 stays precise.
    let rect = Rect::from_ltrb(
        l.max(-4.0) as f32,
        t.max(-4.0) as f32,
        r.min(w as f64 + 4.0) as f32,
        b.min(h as f64 + 4.0) as f32,
    );
    if let Some(rect) = rect {
        paint.shader = shader;
        pm.fill_rect(rect, paint, Transform::identity(), None);
    }
}

fn draw_canv(pm: &mut PixmapMut, canv: &Canv, x0_px: i64, scale: f64, w: u32, h: u32) {
    let x0 = x0_px as f64;
    let mut paint = Paint { anti_alias: true, ..Default::default() };
    for s in canv {
        if s.pts.len() < 2 {
            continue;
        }
        if s.haze {
            if s.fill.visible() {
                draw_haze(pm, &mut paint, s, x0, scale, w, h);
            }
            continue;
        }
        let has_fill = s.fill.visible();
        let has_stroke = s.stroke.visible() && s.width > 0.0;
        if !has_fill && !has_stroke {
            continue;
        }
        // Transform in f64 first: world x can be large, f32 is not precise enough.
        let mut minx = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        let mut pb = PathBuilder::with_capacity(s.pts.len() + 1, s.pts.len() + 1);
        let mut started = false;
        for p in &s.pts {
            let (px, py) = (p[0] * scale - x0, p[1] * scale);
            if !(px.is_finite() && py.is_finite()) {
                continue;
            }
            minx = minx.min(px);
            maxx = maxx.max(px);
            miny = miny.min(py);
            maxy = maxy.max(py);
            if started {
                pb.line_to(px as f32, py as f32);
            } else {
                pb.move_to(px as f32, py as f32);
                started = true;
            }
        }
        let pad = s.width * scale * 2.5 + 2.0; // miter joins reach 2x width
        if maxx < -pad || minx > w as f64 + pad || maxy < -pad || miny > h as f64 + pad {
            continue;
        }
        let Some(path) = pb.finish() else { continue };
        if has_fill {
            paint.set_color(color(s.fill));
            pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
        if has_stroke {
            paint.set_color(color(s.stroke));
            let st = Stroke {
                width: (s.width * scale) as f32,
                miter_limit: 4.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                dash: None,
            };
            pm.stroke_path(&path, &paint, &st, Transform::identity(), None);
        }
    }
}

fn map_palette(buf: &mut [u8], x0_px: i64, w: u32, _h: u32, fmt: PixelFormat, pal: &Palette) {
    let w = w as usize;
    for (row, line) in buf.chunks_exact_mut(w * 4).enumerate() {
        for (col, px) in line.chunks_exact_mut(4).enumerate() {
            // Opaque white background, so premultiplied == straight.
            let lum = (px[0] as u32 * 54 + px[1] as u32 * 183 + px[2] as u32 * 19) >> 8;
            let ink = 255 - lum.min(255) as i32;
            if let Some(tile) = &pal.paper {
                // Multiply onto the paper tile, keyed to the world pixel.
                let fold = |v: i64| {
                    let v = v.rem_euclid(PAPER_TILE);
                    (if v <= PAPER_TILE / 2 { v } else { PAPER_TILE - v }) as usize
                };
                let c = tile[fold(x0_px + col as i64) * PAPER_Q + fold(row as i64)] as u32;
                let l = lum.min(255);
                let (r, g, b) = (c * l / 255, c * l * 95 / 25500, c * l * 85 / 25500);
                let (r, g, b) = (r as u8, g as u8, b as u8);
                match fmt {
                    PixelFormat::Bgrx => px.copy_from_slice(&[b, g, r, 255]),
                    PixelFormat::Rgba => px.copy_from_slice(&[r, g, b, 255]),
                }
                continue;
            }
            // Grain keyed to the world pixel, so it is seamless across strips.
            let gx = (x0_px + col as i64) as u64;
            let g = mix(gx.wrapping_mul(0x1_0000_0001) ^ row as u64);
            let jitter = ((g >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * pal.grain;
            let i = (ink as f32 + jitter).round().clamp(0.0, 255.0) as usize;
            let [r, g, b] = pal.lut[i];
            match fmt {
                PixelFormat::Bgrx => {
                    px[0] = b;
                    px[1] = g;
                    px[2] = r;
                    px[3] = 255;
                }
                PixelFormat::Rgba => {
                    px[0] = r;
                    px[1] = g;
                    px[2] = b;
                    px[3] = 255;
                }
            }
        }
    }
}

/// Debug helper for porting work: paint a bare `Canv` (world coordinates) to
/// a PNG in the night palette. `wx0` is the world x of the left edge.
pub fn preview_png(canv: &Canv, wx0: f64, w: u32, h: u32, path: &str) -> Result<(), String> {
    let scale = scale_for_height(h);
    let x0_px = (wx0 * scale).round() as i64;
    let mut buf = vec![0u8; w as usize * h as usize * 4];
    {
        let mut pm = PixmapMut::from_bytes(&mut buf, w, h).ok_or("empty preview")?;
        pm.fill(Color::WHITE);
        draw_canv(&mut pm, canv, x0_px, scale, w, h);
    }
    map_palette(&mut buf, x0_px, w, h, PixelFormat::Rgba, &Palette::new(PaletteKind::Night));
    let pm = tiny_skia::Pixmap::from_vec(buf, tiny_skia::IntSize::from_wh(w, h).ok_or("size")?)
        .ok_or("bad pixmap size")?;
    pm.save_png(path).map_err(|e| format!("{path}: {e}"))
}

#[cfg(test)]
mod edge_tests {
    use super::*;

    fn strip(world: &World, x0: i64, w: u32, h: u32) -> Vec<u8> {
        let mut b = vec![0u8; w as usize * h as usize * 4];
        let pal = Palette::new(PaletteKind::Night);
        render_strip(world, x0, scale_for_height(h), w, h, &mut b, PixelFormat::Rgba, &pal);
        b
    }

    #[test]
    fn strips_join_inside_edge() {
        let world = World::new(3);
        let (w, h) = (200usize, 300u32);
        for p in [1000i64, 1373, 2222, 4097] {
            let narrow = strip(&world, p, w as u32, h);
            let wide = strip(&world, p - 300, 800, h);
            // Within 3, except rare isolated pixels up to 8: a supersample
            // flips when f32 rounding differs between strips (as in
            // `world::tests::strips_are_seamless`).
            let mut over = 0;
            for y in 0..h as usize {
                for x in EDGE as usize..w - EDGE as usize {
                    for c in 0..3 {
                        let a = narrow[(y * w + x) * 4 + c] as i32;
                        let b = wide[(y * 800 + x + 300) * 4 + c] as i32;
                        assert!((a - b).abs() <= 8, "p={p} x={x} y={y}: {a} vs {b}");
                        over += ((a - b).abs() > 3) as usize;
                    }
                }
            }
            let total = h as usize * (w - 2 * EDGE as usize) * 3;
            assert!(over * 10_000 < total, "p={p}: {over} values differ by more than 3");
        }
    }
}
