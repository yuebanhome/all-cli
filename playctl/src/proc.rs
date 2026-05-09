use crate::error::ExitError;
use anyhow::{Context, Result};
use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHandle {
    pub pid: u32,
    pub port: u16,
}

pub fn runtime_dir(project_root: &Path) -> PathBuf {
    project_root.join(".playgrounds").join(".runtime")
}
pub fn pid_path(project_root: &Path) -> PathBuf {
    runtime_dir(project_root).join("server.pid")
}
pub fn port_path(project_root: &Path) -> PathBuf {
    runtime_dir(project_root).join("server.port")
}
pub fn log_path(project_root: &Path) -> PathBuf {
    runtime_dir(project_root).join("server.log")
}

pub fn ensure_runtime_dir(project_root: &Path) -> Result<()> {
    let d = runtime_dir(project_root);
    fs::create_dir_all(&d)
        .with_context(|| ExitError::Env(format!("mkdir {}", d.display())))?;
    Ok(())
}

pub fn write_handle(project_root: &Path, h: &ServerHandle) -> Result<()> {
    ensure_runtime_dir(project_root)?;
    fs::write(pid_path(project_root), h.pid.to_string())
        .with_context(|| ExitError::Env("write pid".into()))?;
    fs::write(port_path(project_root), h.port.to_string())
        .with_context(|| ExitError::Env("write port".into()))?;
    Ok(())
}

pub fn read_handle(project_root: &Path) -> Option<ServerHandle> {
    let pid = fs::read_to_string(pid_path(project_root)).ok()?;
    let port = fs::read_to_string(port_path(project_root)).ok()?;
    let pid: u32 = pid.trim().parse().ok()?;
    let port: u16 = port.trim().parse().ok()?;
    Some(ServerHandle { pid, port })
}

pub fn clear_handle(project_root: &Path) {
    let _ = fs::remove_file(pid_path(project_root));
    let _ = fs::remove_file(port_path(project_root));
}

#[cfg(unix)]
pub fn is_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
pub fn is_alive(_pid: u32) -> bool {
    false
}

/// 半秒超时连一次 /healthz；不阻塞超过 ~500ms。
pub fn check_healthz(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let stream = match TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_millis(250),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(Duration::from_millis(250))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(250))).ok();
    use std::io::{Read, Write};
    let req = b"GET /healthz HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n";
    let mut s = stream;
    if s.write_all(req).is_err() {
        return false;
    }
    let mut buf = [0u8; 64];
    let n = s.read(&mut buf).unwrap_or(0);
    let head = String::from_utf8_lossy(&buf[..n]);
    head.starts_with("HTTP/1.0 200") || head.starts_with("HTTP/1.1 200")
}

#[derive(Debug, PartialEq, Eq)]
pub enum AliveStatus {
    Running(ServerHandle),
    Stale,
    Absent,
}

pub fn probe(project_root: &Path) -> AliveStatus {
    let Some(h) = read_handle(project_root) else {
        return AliveStatus::Absent;
    };
    if !is_alive(h.pid) {
        return AliveStatus::Stale;
    }
    if !check_healthz(h.port) {
        return AliveStatus::Stale;
    }
    AliveStatus::Running(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_handle() {
        let td = TempDir::new().unwrap();
        let h = ServerHandle { pid: 12345, port: 4747 };
        write_handle(td.path(), &h).unwrap();
        assert_eq!(read_handle(td.path()), Some(h));
        clear_handle(td.path());
        assert_eq!(read_handle(td.path()), None);
    }

    #[test]
    #[cfg(unix)]
    fn current_process_is_alive() {
        let pid = std::process::id();
        assert!(is_alive(pid));
    }

    #[test]
    fn dead_pid_is_not_alive() {
        // 2^30 超出常见 pid 范围，必然不存活。
        assert!(!is_alive(1 << 30));
    }

    #[test]
    fn probe_absent_when_no_files() {
        let td = TempDir::new().unwrap();
        assert_eq!(probe(td.path()), AliveStatus::Absent);
    }

    #[test]
    fn probe_stale_when_pid_dead() {
        let td = TempDir::new().unwrap();
        write_handle(td.path(), &ServerHandle { pid: u32::MAX, port: 1 }).unwrap();
        assert_eq!(probe(td.path()), AliveStatus::Stale);
    }
}
