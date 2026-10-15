//! Background strip painter: one low-priority thread that owns its own
//! `World`, paints strips straight into memfds and hands back the fds.
//! The mapping is dropped as soon as the paint is done, so the pixels live
//! only in the memfd (and later in the compositor's pool) — never in our RSS.

use crate::paint::{Palette, PaletteKind, PixelFormat, render_strip};
use crate::world::World;
use rustix::event::{EventfdFlags, eventfd};
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};
use std::os::fd::OwnedFd;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

pub struct Job {
    pub out: u32,
    pub id: u64,
    pub x0: i64,
    pub scale: f64,
    pub w: u32,
    pub h: u32,
}

pub struct Done {
    pub job: Job,
    pub fd: Result<OwnedFd, String>,
}

pub struct Painter {
    tx: Sender<Job>,
    rx: Receiver<Done>,
    /// eventfd, readable when a `Done` is waiting
    pub wake: OwnedFd,
}

impl Painter {
    pub fn spawn(seed: u64, palette: PaletteKind) -> Result<Painter, String> {
        let wake = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)
            .map_err(|e| format!("eventfd: {e}"))?;
        let wake2 = wake.try_clone().map_err(|e| format!("eventfd: {e}"))?;
        let (tx, jrx) = channel::<Job>();
        let (dtx, rx) = channel::<Done>();
        std::thread::Builder::new()
            .name("paint".into())
            .spawn(move || {
                // Per-thread nice on Linux.
                let _ = rustix::process::setpriority_process(Some(rustix::thread::gettid()), 19);
                let world = World::new(seed);
                let pal = Palette::new(palette);
                for job in jrx {
                    let t0 = Instant::now();
                    let fd = paint(&world, &pal, &job);
                    super::debug(format_args!(
                        "painted strip out={} x0={} {}x{} in {} ms",
                        job.out,
                        job.x0,
                        job.w,
                        job.h,
                        t0.elapsed().as_millis()
                    ));
                    if dtx.send(Done { job, fd }).is_err() {
                        break;
                    }
                    let _ = rustix::io::write(&wake2, &1u64.to_ne_bytes());
                    trim_heap();
                }
            })
            .map_err(|e| format!("cannot start paint thread: {e}"))?;
        Ok(Painter { tx, rx, wake })
    }

    pub fn request(&self, job: Job) {
        let _ = self.tx.send(job);
    }

    /// Drain finished strips (and reset the eventfd).
    pub fn finished(&self) -> Vec<Done> {
        let mut b = [0u8; 8];
        let _ = rustix::io::read(&self.wake, &mut b);
        self.rx.try_iter().collect()
    }
}

fn paint(world: &World, pal: &Palette, job: &Job) -> Result<OwnedFd, String> {
    let len = job.w as usize * job.h as usize * 4;
    let fd = memfd_create("shanshui-strip", MemfdFlags::CLOEXEC)
        .map_err(|e| format!("memfd_create: {e}"))?;
    ftruncate(&fd, len as u64).map_err(|e| format!("ftruncate: {e}"))?;
    // SAFETY: fresh private memfd of exactly `len` bytes, mapped shared;
    // nobody else can see it until we hand out the fd.
    unsafe {
        let p = mmap(
            std::ptr::null_mut(),
            len,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::SHARED,
            &fd,
            0,
        )
        .map_err(|e| format!("mmap: {e}"))?;
        let buf = std::slice::from_raw_parts_mut(p.cast::<u8>(), len);
        render_strip(world, job.x0, job.scale, job.w, job.h, buf, PixelFormat::Bgrx, pal);
        let _ = munmap(p, len);
    }
    Ok(fd)
}

/// Give the paint thread's freed heap back to the kernel (glibc keeps a
/// per-thread arena otherwise).
fn trim_heap() {
    unsafe extern "C" {
        fn malloc_trim(pad: usize) -> i32;
    }
    // SAFETY: plain glibc call, no pointers involved.
    unsafe {
        malloc_trim(0);
    }
}
