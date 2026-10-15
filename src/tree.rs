//! Trees (`Tree.tree01` .. `Tree.tree08` in the original).
//!
//! STUB: signatures and argument defaults only. Every JS arg key is a field
//! with the same name; JS numbers are f64; colours are `Rgba`.

use crate::geom::{Canv, Rgba, rgba};
use crate::rng::Cx;

fn grey(a: f64) -> Rgba {
    rgba(100.0, 100.0, 100.0, a)
}

pub struct Tree01Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree01Args {
    fn default() -> Self { Tree01Args { hei: 50.0, wid: 3.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree01(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree01Args) -> Canv { Vec::new() }

pub struct Tree02Args { pub hei: f64, pub wid: f64, pub clu: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree02Args {
    fn default() -> Self { Tree02Args { hei: 16.0, wid: 8.0, clu: 5.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree02(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree02Args) -> Canv { Vec::new() }

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
pub fn tree03(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree03Args) -> Canv { Vec::new() }

pub struct Tree04Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree04Args {
    fn default() -> Self { Tree04Args { hei: 300.0, wid: 6.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree04(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree04Args) -> Canv { Vec::new() }

pub struct Tree05Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree05Args {
    fn default() -> Self { Tree05Args { hei: 300.0, wid: 5.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree05(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree05Args) -> Canv { Vec::new() }

pub struct Tree06Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree06Args {
    fn default() -> Self { Tree06Args { hei: 100.0, wid: 6.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree06(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree06Args) -> Canv { Vec::new() }

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
pub fn tree07(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree07Args) -> Canv { Vec::new() }

pub struct Tree08Args { pub hei: f64, pub wid: f64, pub col: Rgba, pub noi: f64 }
impl Default for Tree08Args {
    fn default() -> Self { Tree08Args { hei: 80.0, wid: 1.0, col: grey(0.5), noi: 0.5 } }
}
pub fn tree08(_cx: &mut Cx, _x: f64, _y: f64, _a: Tree08Args) -> Canv { Vec::new() }
