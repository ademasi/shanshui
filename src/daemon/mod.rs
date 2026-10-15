//! The Wayland wallpaper daemon: one wlr-layer-shell background surface per
//! output, each showing a crop of a painted strip twice the output's width.
//! Drift moves the crop with wp_viewporter (no repaint, no new buffer); a
//! low-priority thread paints the next strip before the crop runs out.

mod painter;
mod persist;
mod wl;

use crate::paint::scale_for_height;
use painter::{Done, Job, Painter};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::os::fd::{AsFd, OwnedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use wayland_client::protocol::{
    wl_buffer::WlBuffer,
    wl_compositor::WlCompositor,
    wl_output::{Transform, WlOutput},
    wl_shm::{self, WlShm},
    wl_surface::WlSurface,
};
use wayland_client::{Connection, Proxy, QueueHandle};
use wayland_protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

pub struct Opts {
    pub seed: Option<u64>,
    /// world units per second
    pub speed: f64,
    pub palette: crate::paint::PaletteKind,
}

/// An output whose frame callback has been outstanding this long counts as
/// hidden; when every output is hidden the drift clock stops.
const HIDDEN_AFTER: Duration = Duration::from_secs(5);
const SAVE_EVERY: Duration = Duration::from_secs(60);

static DEBUG: AtomicBool = AtomicBool::new(false);

pub(crate) fn debug(args: std::fmt::Arguments) {
    if DEBUG.load(Ordering::Relaxed) {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() % 1000.0)
            .unwrap_or(0.0);
        eprintln!("shanshui[{t:7.3}]: {args}");
    }
}

#[derive(Default)]
pub(crate) struct Globals {
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    layer_shell: Option<ZwlrLayerShellV1>,
    viewporter: Option<WpViewporter>,
    xdg_mgr: Option<ZxdgOutputManagerV1>,
}

/// Columns at each end of a strip whose anti-aliasing differs from the same
/// world pixels painted in another strip; the crop never shows them.
/// (Becomes `paint::EDGE` on merge.)
const EDGE: u32 = 8;

/// Buffer width of a strip for an output `pw` pixels wide.
fn strip_w(pw: u32) -> u32 {
    2 * pw + 2 * EDGE
}

/// A painted strip: world pixels `[x0, x0 + w)` at `scale` px per unit.
/// The crop (`w / 2 - EDGE` wide, the output width when it was painted)
/// may start at buffer columns `EDGE..=EDGE + crop_w`.
#[derive(Clone, Copy)]
struct Strip {
    x0: i64,
    scale: f64,
    w: u32,
    h: u32,
}

impl Strip {
    fn crop_w(&self) -> i64 {
        (self.w / 2 - EDGE) as i64
    }
}

struct Ready {
    buf: WlBuffer,
    strip: Strip,
}

pub(crate) struct Surf {
    wl: WlSurface,
    layer: ZwlrLayerSurfaceV1,
    vp: WpViewport,
    configured: bool,
    logical: (i32, i32),
    needs_commit: bool,
    /// when we asked for the frame callback that has not arrived yet
    frame_pending: Option<Instant>,
    cur: Option<Strip>,
    src_x: i64,
    ready: Option<Ready>,
    job: Option<u64>,
}

pub(crate) struct Out {
    name: u32,
    wl: WlOutput,
    xdg: Option<ZxdgOutputV1>,
    oname: String,
    mode: (i32, i32),
    transform: Transform,
    pos: (i32, i32),
    info_done: bool,
    closed: bool,
    surf: Option<Surf>,
}

impl Out {
    fn new(name: u32, wl: WlOutput, xdg: Option<ZxdgOutputV1>) -> Self {
        Out {
            name,
            wl,
            xdg,
            oname: String::new(),
            mode: (0, 0),
            transform: Transform::Normal,
            pos: (0, 0),
            info_done: false,
            closed: false,
            surf: None,
        }
    }

    fn label(&self) -> String {
        if self.oname.is_empty() { format!("output {}", self.name) } else { self.oname.clone() }
    }

    /// Buffer pixels in the surface's (logical) orientation.
    fn phys(&self) -> (u32, u32) {
        let (w, h) = (self.mode.0.max(0) as u32, self.mode.1.max(0) as u32);
        match self.transform {
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
                (h, w)
            }
            _ => (w, h),
        }
    }

    fn drop_surface(&mut self) {
        if let Some(s) = self.surf.take() {
            if let Some(r) = s.ready {
                r.buf.destroy();
            }
            s.vp.destroy();
            s.layer.destroy();
            s.wl.destroy();
        }
    }

    fn teardown(&mut self) {
        self.drop_surface();
        if let Some(x) = self.xdg.take() {
            x.destroy();
        }
        if self.wl.version() >= 3 {
            self.wl.release();
        }
    }
}

pub(crate) struct State {
    g: Globals,
    qh: QueueHandle<State>,
    outputs: Vec<Out>,
    painter: Painter,
    seed: u64,
    speed: f64,
    /// world offset of the panorama's left edge
    w: f64,
    last_advance: Instant,
    next_job: u64,
    released: u64,
    debug_frames: u32,
    debug_steps: u32,
    paused: bool,
}

pub fn run(opts: Opts) -> Result<(), String> {
    DEBUG.store(std::env::var_os("SHANSHUI_DEBUG").is_some_and(|v| v == "1"), Ordering::Relaxed);
    if !opts.speed.is_finite() {
        return Err("--speed must be a finite number".into());
    }
    let _lock = match persist::lock()? {
        persist::Lock::Held(f) => f,
        persist::Lock::Busy => {
            eprintln!("shanshui: already running");
            return Ok(());
        }
    };

    let saved = persist::load();
    let (seed, w0) = match (opts.seed, saved) {
        (Some(s), Some((ss, w))) if s == ss => (s, w),
        (Some(s), _) => (s, 0.0),
        (None, Some((ss, w))) => (ss, w),
        (None, None) => (persist::random_seed(), 0.0),
    };
    debug(format_args!("seed {seed}, W {w0:.2}, speed {}", opts.speed));

    let conn = Connection::connect_to_env().map_err(|e| format!("cannot connect to Wayland: {e}"))?;
    let mut queue = conn.new_event_queue::<State>();
    let qh = queue.handle();
    conn.display().get_registry(&qh, ());

    let (sig_r, sig_w) = rustix::pipe::pipe_with(
        rustix::pipe::PipeFlags::CLOEXEC | rustix::pipe::PipeFlags::NONBLOCK,
    )
    .map_err(|e| format!("pipe: {e}"))?;
    for sig in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT, signal_hook::consts::SIGHUP] {
        let w = sig_w.try_clone().map_err(|e| format!("pipe: {e}"))?;
        signal_hook::low_level::pipe::register(sig, w).map_err(|e| format!("signal: {e}"))?;
    }
    drop(sig_w);

    let mut st = State {
        g: Globals::default(),
        qh: qh.clone(),
        outputs: Vec::new(),
        painter: Painter::spawn(seed, opts.palette)?,
        seed,
        speed: opts.speed,
        w: w0,
        last_advance: Instant::now(),
        next_job: 1,
        released: 0,
        debug_frames: 6,
        debug_steps: 6,
        paused: false,
    };
    queue.roundtrip(&mut st).map_err(|e| format!("Wayland: {e}"))?;
    queue.roundtrip(&mut st).map_err(|e| format!("Wayland: {e}"))?;
    for (ok, what) in [
        (st.g.compositor.is_some(), "wl_compositor v4"),
        (st.g.shm.is_some(), "wl_shm"),
        (st.g.layer_shell.is_some(), "zwlr_layer_shell_v1"),
        (st.g.viewporter.is_some(), "wp_viewporter"),
    ] {
        if !ok {
            return Err(format!("the compositor lacks {what}"));
        }
    }

    let res = event_loop(&conn, &mut queue, &mut st, &sig_r);
    if let Err(e) = persist::save(st.seed, st.w) {
        eprintln!("shanshui: cannot save state: {e}");
    }
    debug(format_args!("exit, saved seed {} W {:.2}", st.seed, st.w));
    res
}

fn event_loop(
    conn: &Connection,
    queue: &mut wayland_client::EventQueue<State>,
    st: &mut State,
    sig: &OwnedFd,
) -> Result<(), String> {
    let lost = |e: &dyn std::fmt::Display| format!("lost the Wayland connection: {e}");
    let mut last_save = Instant::now();
    let mut saved_w = st.w;
    loop {
        st.advance(Instant::now());
        let timeout = st.tick();
        conn.flush().map_err(|e| lost(&e))?;
        let guard = loop {
            match queue.prepare_read() {
                Some(g) => break g,
                None => {
                    queue.dispatch_pending(st).map_err(|e| lost(&e))?;
                }
            }
        };
        let save_in = SAVE_EVERY.saturating_sub(last_save.elapsed());
        let timeout = timeout.map_or(save_in, |t| t.min(save_in));
        let ts = Timespec {
            tv_sec: timeout.as_secs() as _,
            tv_nsec: timeout.subsec_nanos() as _,
        };
        let (wl_ready, paint_ready, sig_ready) = {
            let wl_fd = guard.connection_fd();
            let mut fds = [
                PollFd::new(&wl_fd, PollFlags::IN),
                PollFd::new(&st.painter.wake, PollFlags::IN),
                PollFd::new(sig, PollFlags::IN),
            ];
            match poll(&mut fds, Some(&ts)) {
                Ok(_) | Err(rustix::io::Errno::INTR) => {}
                Err(e) => return Err(format!("poll: {e}")),
            }
            let r = |f: &PollFd| !f.revents().is_empty();
            (r(&fds[0]), r(&fds[1]), r(&fds[2]))
        };
        // Account the drift up to now before any event changes visibility.
        st.advance(Instant::now());
        if wl_ready {
            match guard.read() {
                Ok(_) => {}
                Err(wayland_client::backend::WaylandError::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(lost(&e)),
            }
        } else {
            drop(guard);
        }
        queue.dispatch_pending(st).map_err(|e| lost(&e))?;
        if paint_ready {
            for d in st.painter.finished() {
                st.strip_done(d);
            }
        }
        if sig_ready {
            debug(format_args!("signal, exiting"));
            return Ok(());
        }
        if last_save.elapsed() >= SAVE_EVERY {
            if st.w != saved_w {
                if let Err(e) = persist::save(st.seed, st.w) {
                    eprintln!("shanshui: cannot save state: {e}");
                }
                saved_w = st.w;
            }
            last_save = Instant::now();
        }
    }
}

impl State {
    /// Move the drift clock to `now`. It runs while some output is visible:
    /// one whose last frame callback arrived, or was requested less than
    /// `HIDDEN_AFTER` ago.
    fn advance(&mut self, now: Instant) {
        let mut until: Option<Instant> = None;
        for s in self.outputs.iter().filter_map(|o| o.surf.as_ref()).filter(|s| s.configured) {
            let t = match s.frame_pending {
                None => now,
                Some(req) => req + HIDDEN_AFTER,
            };
            until = Some(until.map_or(t, |u| u.max(t)));
        }
        if let Some(end) = until.map(|u| u.min(now)) {
            if end > self.last_advance {
                self.w += self.speed * end.duration_since(self.last_advance).as_secs_f64();
            }
        }
        self.last_advance = now;
        let paused = until.is_none_or(|u| u < now);
        if paused != self.paused {
            self.paused = paused;
            debug(format_args!(
                "drift clock {} at W {:.2}",
                if paused { "paused (no output visible)" } else { "running" },
                self.w
            ));
        }
    }

    fn running(&self, now: Instant) -> bool {
        self.outputs.iter().filter_map(|o| o.surf.as_ref()).filter(|s| s.configured).any(|s| {
            s.frame_pending.is_none_or(|req| now.duration_since(req) < HIDDEN_AFTER)
        })
    }

    /// Create missing surfaces, request strips, step the crops. Returns how
    /// long until the next output needs a step (None: nothing scheduled).
    fn tick(&mut self) -> Option<Duration> {
        let now = Instant::now();
        self.create_surfaces();
        let mut order: Vec<usize> = (0..self.outputs.len())
            .filter(|&i| {
                let o = &self.outputs[i];
                let (pw, ph) = o.phys();
                pw > 0 && ph > 0 && o.surf.as_ref().is_some_and(|s| s.configured)
            })
            .collect();
        order.sort_by_key(|&i| (self.outputs[i].pos.0, self.outputs[i].pos.1));
        let running = self.running(now);
        let mut cum = 0.0;
        let mut timeout: Option<Duration> = None;
        for i in order {
            let (pw, ph) = self.outputs[i].phys();
            let scale = scale_for_height(ph);
            let left = self.w + cum;
            cum += pw as f64 / scale;
            let t = self.drive(i, left, pw, ph, scale, now);
            if running {
                if let Some(t) = t {
                    timeout = Some(timeout.map_or(t, |u| u.min(t)));
                }
            }
            // While an output is pending we may need to notice it going hidden.
            if let Some(req) = self.outputs[i].surf.as_ref().and_then(|s| s.frame_pending) {
                let deadline = req + HIDDEN_AFTER;
                if deadline > now {
                    let t = deadline - now + Duration::from_millis(1);
                    timeout = Some(timeout.map_or(t, |u| u.min(t)));
                }
            }
        }
        timeout
    }

    fn create_surfaces(&mut self) {
        let (Some(comp), Some(ls), Some(vpr)) =
            (&self.g.compositor, &self.g.layer_shell, &self.g.viewporter)
        else {
            return;
        };
        for o in &mut self.outputs {
            if o.surf.is_some() || !o.info_done || o.closed {
                continue;
            }
            let wl = comp.create_surface(&self.qh, ());
            let layer = ls.get_layer_surface(
                &wl,
                Some(&o.wl),
                Layer::Background,
                "wallpaper".into(),
                &self.qh,
                o.name,
            );
            layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
            layer.set_exclusive_zone(-1);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer.set_size(0, 0);
            let region = comp.create_region(&self.qh, ());
            wl.set_input_region(Some(&region));
            region.destroy();
            let vp = vpr.get_viewport(&wl, &self.qh, ());
            wl.commit();
            debug(format_args!("{}: layer surface created", o.label()));
            o.surf = Some(Surf {
                wl,
                layer,
                vp,
                configured: false,
                logical: (0, 0),
                needs_commit: false,
                frame_pending: None,
                cur: None,
                src_x: i64::MIN,
                ready: None,
                job: None,
            });
        }
    }

    /// Request strips and step one output. Returns the time until its crop
    /// moves by a pixel, if it can step.
    fn drive(&mut self, i: usize, left: f64, pw: u32, ph: u32, scale: f64, now: Instant) -> Option<Duration> {
        let speed = self.speed;
        let o = &mut self.outputs[i];
        let name = o.name;
        let s = o.surf.as_mut()?;
        let sw = strip_w(pw);
        let latest = s.ready.as_ref().map(|r| r.strip).or(s.cur);
        let want = match latest {
            None => true,
            Some(st) if st.w != sw || st.h != ph => true,
            Some(st) => {
                // position of the crop within the usable margin [0, m]
                let t = (left * st.scale).floor() as i64 - st.x0 - EDGE as i64;
                let m = st.crop_w();
                t < 0 || t > m || (speed > 0.0 && t * 4 > 3 * m) || (speed < 0.0 && t * 4 < m)
            }
        };
        if want && s.job.is_none() {
            let target = (left * scale).floor() as i64;
            let x0 = if speed < 0.0 { target - pw as i64 } else { target } - EDGE as i64;
            let id = self.next_job;
            self.next_job += 1;
            s.job = Some(id);
            debug(format_args!("{}: requesting strip x0={x0} ({}x{ph})", o.oname, sw));
            self.painter.request(Job { out: name, id, x0, scale, w: sw, h: ph });
        }

        if s.frame_pending.is_some() {
            if s.needs_commit && s.cur.is_some() {
                s.vp.set_destination(s.logical.0, s.logical.1);
                s.wl.commit();
                s.needs_commit = false;
            }
            return None;
        }
        let mut commit = s.needs_commit;
        let mut swapped = None;
        if let Some(r) = s.ready.take() {
            s.wl.attach(Some(&r.buf), 0, 0);
            s.wl.damage_buffer(0, 0, r.strip.w as i32, r.strip.h as i32);
            let prev = s.cur.map(|c| c.x0 + s.src_x);
            s.cur = Some(r.strip);
            s.src_x = i64::MIN;
            commit = true;
            swapped = Some((prev, r.strip.x0));
        }
        let st = s.cur?;
        let m = st.crop_w();
        let v = left * st.scale;
        let raw = v.floor() as i64 - st.x0;
        let src = raw.clamp(EDGE as i64, EDGE as i64 + m);
        if src != s.src_x || commit {
            s.vp.set_source(src as f64, 0.0, m as f64, st.h as f64);
            s.vp.set_destination(s.logical.0, s.logical.1);
            match swapped {
                Some((None, x0)) => {
                    debug(format_args!("{}: attached first strip x0={x0} src_x={src}", o.oname))
                }
                Some((Some(prev), x0)) => debug(format_args!(
                    "{}: swapped to strip x0={x0} src_x={src}; left edge world px {prev} -> {}",
                    o.oname,
                    x0 + src
                )),
                None => {}
            }
            if self.debug_steps > 0 && src != s.src_x && swapped.is_none() {
                self.debug_steps -= 1;
                debug(format_args!("{}: step src_x={src}", o.oname));
            }
            s.src_x = src;
            s.wl.frame(&self.qh, name);
            s.frame_pending = Some(now);
            s.wl.commit();
            s.needs_commit = false;
            return None;
        }
        // Waiting for the next pixel.
        if speed == 0.0
            || (speed > 0.0 && raw >= EDGE as i64 + m)
            || (speed < 0.0 && raw <= EDGE as i64)
        {
            return None;
        }
        let next_v = if speed > 0.0 { v.floor() + 1.0 } else { v.floor() };
        let secs = ((next_v / st.scale - left) / speed).max(0.0) + 1e-4;
        Some(Duration::from_secs_f64(secs.min(3600.0)))
    }

    fn strip_done(&mut self, d: Done) {
        let Some(o) = self.outputs.iter_mut().find(|o| o.name == d.job.out) else { return };
        let (pw, ph) = o.phys();
        let Some(s) = o.surf.as_mut() else { return };
        if s.job != Some(d.job.id) {
            return;
        }
        s.job = None;
        let fd = match d.fd {
            Ok(fd) => fd,
            Err(e) => {
                eprintln!("shanshui: painting a strip failed: {e}");
                return;
            }
        };
        let j = d.job;
        if j.w != strip_w(pw) || j.h != ph {
            return; // output changed size meanwhile; a new job follows
        }
        let Some(shm) = &self.g.shm else { return };
        let stride = j.w as i32 * 4;
        let pool = shm.create_pool(fd.as_fd(), stride * j.h as i32, &self.qh, ());
        let buf = pool.create_buffer(0, j.w as i32, j.h as i32, stride, wl_shm::Format::Xrgb8888, &self.qh, ());
        // The buffer keeps the pool's memory alive; we need neither the pool
        // nor the fd any more.
        pool.destroy();
        drop(fd);
        let strip = Strip { x0: j.x0, scale: j.scale, w: j.w, h: j.h };
        if let Some(old) = s.ready.replace(Ready { buf, strip }) {
            old.buf.destroy();
        }
    }
}
