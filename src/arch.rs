//! Architecture and boats (`Arch.*` in the original).
//!
//! STUB: signatures and argument defaults only. Every JS arg key is a field
//! with the same name; JS numbers are f64. `seed` is the original's numeric
//! seed argument (used for noise offsets), not an RNG seed.

use crate::geom::Canv;
use crate::rng::Cx;

pub struct Arch01Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64 }
impl Default for Arch01Args {
    fn default() -> Self { Arch01Args { hei: 70.0, wid: 180.0, rot: 0.7, per: 5.0 } }
}
pub fn arch01(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: Arch01Args) -> Canv { Vec::new() }

pub struct Arch02Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64, pub sty: f64, pub rai: bool }
impl Default for Arch02Args {
    fn default() -> Self { Arch02Args { hei: 10.0, wid: 50.0, rot: 0.3, per: 5.0, sto: 3.0, sty: 1.0, rai: false } }
}
pub fn arch02(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: Arch02Args) -> Canv { Vec::new() }

pub struct Arch03Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64 }
impl Default for Arch03Args {
    fn default() -> Self { Arch03Args { hei: 10.0, wid: 50.0, rot: 0.7, per: 5.0, sto: 7.0 } }
}
pub fn arch03(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: Arch03Args) -> Canv { Vec::new() }

pub struct Arch04Args { pub hei: f64, pub wid: f64, pub rot: f64, pub per: f64, pub sto: f64 }
impl Default for Arch04Args {
    fn default() -> Self { Arch04Args { hei: 15.0, wid: 30.0, rot: 0.7, per: 5.0, sto: 2.0 } }
}
pub fn arch04(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: Arch04Args) -> Canv { Vec::new() }

pub struct Boat01Args { pub len: f64, pub sca: f64, pub fli: bool }
impl Default for Boat01Args {
    fn default() -> Self { Boat01Args { len: 120.0, sca: 1.0, fli: false } }
}
pub fn boat01(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: Boat01Args) -> Canv { Vec::new() }

pub struct TransmissionTower01Args { pub hei: f64, pub wid: f64 }
impl Default for TransmissionTower01Args {
    fn default() -> Self { TransmissionTower01Args { hei: 100.0, wid: 20.0 } }
}
pub fn transmission_tower01(_cx: &mut Cx, _xoff: f64, _yoff: f64, _seed: f64, _a: TransmissionTower01Args) -> Canv { Vec::new() }
