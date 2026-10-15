use shanshui::daemon;
use shanshui::paint::{Palette, PaletteKind, PixelFormat, render_strip, scale_for_height};
use shanshui::world::World;
use std::process::ExitCode;
use std::time::Instant;

const HELP: &str = "\
shanshui: an endless shan-shui ink landscape drifting behind your windows

USAGE:
  shanshui [--seed N] [--speed U] [--fps F] [--palette P]
      Run as the wallpaper (wlr-layer-shell background, one surface per output).
  shanshui pause | resume | toggle
      Freeze or restart the drift of the running wallpaper (remembered across
      restarts). Frozen, it uses no CPU; it also stops by itself when covered.
  shanshui render [--seed N] [--x X] [--width W] [--height H] [--palette P] -o FILE.png
      Paint one stretch of landscape to a PNG.

OPTIONS:
  --seed N      world seed (default: saved state, else random)
  --speed U     drift speed in world units per second (default 3, ~4.6 px/s at 1080p)
  --fps F       sub-pixel crop updates per second while visible (default 30)
  --palette P   paper | night-soft | night | night-tint (default paper)
  --x X         world x of the left edge, in world units (render, default 0)
  --width W     pixels (render, default 1920)
  --height H    pixels (render, default 1080)
  -o FILE       output PNG (render)
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("shanshui: {e}");
            ExitCode::FAILURE
        }
    }
}

fn palette_arg(args: &mut pico_args::Arguments) -> Result<PaletteKind, String> {
    let p: Option<String> = args.opt_value_from_str("--palette").map_err(|e| e.to_string())?;
    match p {
        None => Ok(PaletteKind::Paper),
        Some(s) => PaletteKind::parse(&s).ok_or_else(|| format!("unknown palette '{s}'")),
    }
}

fn run() -> Result<(), String> {
    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        print!("{HELP}");
        return Ok(());
    }
    let sub = args.subcommand().map_err(|e| e.to_string())?;
    let seed: Option<u64> = args.opt_value_from_str("--seed").map_err(|e| e.to_string())?;
    let palette = palette_arg(&mut args)?;
    match sub.as_deref() {
        Some("render") => {
            let x: f64 = args.opt_value_from_str("--x").map_err(|e| e.to_string())?.unwrap_or(0.0);
            let w: u32 = args.opt_value_from_str("--width").map_err(|e| e.to_string())?.unwrap_or(1920);
            let h: u32 = args.opt_value_from_str("--height").map_err(|e| e.to_string())?.unwrap_or(1080);
            let out: String = args.value_from_str("-o").map_err(|_| "render needs -o FILE.png".to_string())?;
            reject_rest(args)?;
            let seed = seed.unwrap_or(1);
            let t0 = Instant::now();
            let world = World::new(seed);
            let scale = scale_for_height(h);
            let mut buf = vec![0u8; w as usize * h as usize * 4];
            render_strip(
                &world,
                (x * scale).round() as i64,
                scale,
                w,
                h,
                &mut buf,
                PixelFormat::Rgba,
                &Palette::new(palette),
            );
            let painted = t0.elapsed();
            let pm = tiny_skia::Pixmap::from_vec(buf, tiny_skia::IntSize::from_wh(w, h).unwrap())
                .ok_or("bad pixmap size")?;
            pm.save_png(&out).map_err(|e| format!("{out}: {e}"))?;
            eprintln!("{out}: {w}x{h}, seed {seed}, painted in {} ms", painted.as_millis());
            Ok(())
        }
        None => {
            let speed: f64 = args.opt_value_from_str("--speed").map_err(|e| e.to_string())?.unwrap_or(3.0);
            let fps: f64 = args.opt_value_from_str("--fps").map_err(|e| e.to_string())?.unwrap_or(30.0);
            reject_rest(args)?;
            daemon::run(daemon::Opts { seed, speed, fps, palette })
        }
        Some(cmd @ ("pause" | "resume" | "toggle")) => {
            reject_rest(args)?;
            daemon::control(cmd)
        }
        Some(other) => Err(format!("unknown command '{other}' (try --help)")),
    }
}

fn reject_rest(args: pico_args::Arguments) -> Result<(), String> {
    let rest = args.finish();
    if rest.is_empty() {
        Ok(())
    } else {
        Err(format!("unexpected arguments: {rest:?}"))
    }
}
