//! STUB: the Wayland wallpaper daemon (layer-shell + viewporter drift).

pub struct Opts {
    pub seed: Option<u64>,
    /// world units per second
    pub speed: f64,
    pub palette: crate::paint::PaletteKind,
}

pub fn run(_opts: Opts) -> Result<(), String> {
    Err("daemon not implemented yet".into())
}
