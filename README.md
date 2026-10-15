# shanshui

An endless shan-shui (山水) ink landscape that drifts slowly behind your sway
windows. It is a Rust port of [shan-shui-inf](https://github.com/LingDong-/shan-shui-inf)
by Lingdong Huang, painted on the original's cream paper by default
(`--palette night-soft` and the others give Catppuccin Macchiato colours).

## How it stays cheap

- **CPU paints rarely.** tiny-skia paints a strip twice the output width,
  about 0.6 s at nice 19. Each output gets a new strip after about half an
  hour of visible drift at the default speed.
- **Sway's GPU scrolls.** The strip goes to sway once as a `wl_shm` buffer and
  is unmapped as soon as sway releases it. Drift only moves a fractional
  `wp_viewporter` crop (`--fps` updates per second, default 30, a fraction of a pixel
  each), and sway's bilinear sampling makes the motion continuous: no repaint,
  no upload, and about 0.3% of one core.
- The paper's per-pixel speckle is softened, because sharp speckle would
  visibly "breathe" at half-pixel crop positions.
- **It stops when hidden.** It steps only on frame callbacks, which sway
  doesn't send to a fully covered background, and the drift clock pauses after 5 s.
- About 5 MB resident at idle (about 1 MB of it private), peaking near 22 MB
  while a strip is painted.

The world is deterministic: per-chunk planner seeds and per-element seeds,
so any stretch can be repainted identically, and strips join without seams.
Several outputs form one panorama, ordered by sway layout. Each output fits
the painting's 700-unit band to its own height.

## Use

```sh
cargo build --release
ln -sfn ~/git/shanshui/target/release/shanshui ~/.local/bin/shanshui
```

sway config:

```
swaybg_command -
exec_always ~/.local/bin/shanshui
```

`exec_always` is safe across reloads, because a second instance sees the lock and exits.

Freeze the drift (it then uses no CPU at all; this is remembered across
restarts), for instance on a key:

```
bindsym $mod+Shift+p exec ~/.local/bin/shanshui toggle
```

`shanshui pause`, `shanshui resume` and `shanshui toggle` signal the running
daemon (SIGUSR1/SIGUSR2; its pid is in `$XDG_RUNTIME_DIR/shanshui.lock`).

```
shanshui [--seed N] [--speed U] [--fps F] [--palette paper|night-soft|night|night-tint]
shanshui render [--seed N] [--x X] [--width W] [--height H] [--palette P] -o out.png
```

State (seed and drift position) lives in `~/.local/state/shanshui/state` and
resumes after a restart; `--seed` starts a new world. `SHANSHUI_DEBUG=1`
logs frame callbacks, steps and strip swaps.

## Credits

The landscape is Lingdong Huang's
[shan-shui-inf](https://github.com/LingDong-/shan-shui-inf) (2018, MIT). This
crate ports its algorithms to Rust: mountains, trees, buildings, figures,
boats, brush strokes, the paper texture and the chunk planner. Its source is
kept unchanged as `ref-shan-shui-inf.html` for reference.

The rendering and the Wayland daemon are new to this port: the
seamless strip painter, the deterministic per-chunk seeds, and the
viewporter drift.

`src/noise.rs` ports the Perlin `noise()` that shan-shui-inf embeds from
[p5.js](https://github.com/processing/p5.js) (`src/math/noise.js`, LGPL-2.1),
which adapted it from Processing, toxi and farbrausch.

## License

MIT, like the original. See [LICENSE](LICENSE), which keeps Lingdong Huang's
copyright notice alongside this port's. The p5.js lineage of `src/noise.rs`
is noted above.
