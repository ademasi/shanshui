# shanshui

An endless shan-shui (山水) ink landscape that drifts slowly behind your sway
windows. Rust port of [LingDong-'s shan-shui-inf](https://github.com/LingDong-/shan-shui-inf)
(MIT, original kept as `ref-shan-shui-inf.html`), painted in the Catppuccin
Macchiato palette.

## How it stays cheap

- **CPU paints rarely.** tiny-skia paints a strip twice the output width,
  about 0.6 s at nice 19. Each output gets a new strip after about half an
  hour of visible drift at the default speed.
- **Sway's GPU scrolls.** The strip goes to sway once as a `wl_shm` buffer and
  is unmapped as soon as sway releases it. Each 1 px drift step only moves a
  `wp_viewporter` crop: no repaint, no upload.
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

```
shanshui [--seed N] [--speed U] [--palette night-soft|night|night-tint|paper]
shanshui render [--seed N] [--x X] [--width W] [--height H] [--palette P] -o out.png
```

State (seed and drift position) lives in `~/.local/state/shanshui/state` and
resumes after a restart; `--seed` starts a new world. `SHANSHUI_DEBUG=1`
logs frame callbacks, steps and strip swaps.
