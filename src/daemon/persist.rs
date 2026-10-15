//! Saved drift state and the single-instance lock.

use rustix::fs::{FlockOperation, flock};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub fn state_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".local/state"),
    };
    Some(base.join("shanshui/state"))
}

pub struct Saved {
    pub seed: u64,
    pub w: f64,
    /// drift frozen by `shanshui pause`
    pub frozen: bool,
}

/// The state file, if there is a readable one.
pub fn load() -> Option<Saved> {
    let text = fs::read_to_string(state_path()?).ok()?;
    let mut seed = None;
    let mut w = 0.0;
    let mut frozen = false;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match (it.next(), it.next()) {
            (Some("seed"), Some(v)) => seed = v.parse().ok(),
            (Some("w"), Some(v)) => w = v.parse().ok().filter(|x: &f64| x.is_finite()).unwrap_or(0.0),
            (Some("paused"), Some(v)) => frozen = v == "1",
            _ => {}
        }
    }
    Some(Saved { seed: seed?, w, frozen })
}

pub fn save(seed: u64, w: f64, frozen: bool) -> Result<(), String> {
    let path = state_path().ok_or("no XDG_STATE_HOME or HOME")?;
    let dir = path.parent().ok_or("bad state path")?;
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = path.with_extension("tmp");
    let mut f = File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    writeln!(f, "seed {seed}\nw {w}\npaused {}", frozen as u8)
        .map_err(|e| format!("{}: {e}", tmp.display()))?;
    drop(f);
    fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))
}

pub enum Lock {
    Held(#[allow(dead_code)] File),
    Busy,
}

fn lock_path() -> Result<PathBuf, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or("XDG_RUNTIME_DIR is not set")?;
    Ok(PathBuf::from(dir).join("shanshui.lock"))
}

/// flock `$XDG_RUNTIME_DIR/shanshui.lock` and write our pid into it; the
/// lock lives as long as the File.
pub fn lock() -> Result<Lock, String> {
    let path = lock_path()?;
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    match flock(&f, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {
            f.set_len(0).map_err(|e| format!("{}: {e}", path.display()))?;
            write!(&f, "{}", std::process::id()).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Lock::Held(f))
        }
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(Lock::Busy),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Pid of the running daemon: the lock is held and its pid is in the file.
pub fn running_pid() -> Result<rustix::process::Pid, String> {
    let path = lock_path()?;
    let not_running = || "not running".to_string();
    let f = File::open(&path).map_err(|_| not_running())?;
    if flock(&f, FlockOperation::NonBlockingLockShared).is_ok() {
        return Err(not_running()); // nobody holds it
    }
    let pid: i32 = fs::read_to_string(&path).ok().and_then(|s| s.trim().parse().ok()).ok_or_else(not_running)?;
    rustix::process::Pid::from_raw(pid).ok_or_else(not_running)
}

pub fn random_seed() -> u64 {
    let mut b = [0u8; 8];
    if rustix::rand::getrandom(&mut b, rustix::rand::GetRandomFlags::empty()).is_ok_and(|n| n == 8)
    {
        // keep it short enough to type back in
        return u64::from_ne_bytes(b) % 1_000_000_000;
    }
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    (t ^ (std::process::id() as u64).rotate_left(32)) % 1_000_000_000
}
