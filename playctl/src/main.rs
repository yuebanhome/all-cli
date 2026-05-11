use anyhow::{Context, Result};
use clap::Parser;
use playctl::{
    cli::{Cli, Cmd},
    error::{classify, ExitError},
    index::read_index,
    proc::{
        check_healthz, clear_handle, ensure_runtime_dir, log_path, pick_port, probe, write_handle,
        AliveStatus, ServerHandle,
    },
    project::{detect_project_root, ensure_gitignore},
    server::run as server_run,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(&cli) {
        if !cli.quiet {
            eprintln!("error: {err:?}");
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(classify(&err) as i32);
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.cmd {
        Cmd::Start => cmd_start(cli),
        Cmd::Stop => cmd_stop(cli),
        Cmd::Status => cmd_status(cli),
        Cmd::List => cmd_list(cli),
        Cmd::New { slug, template, title, description } => {
            cmd_new(cli, slug, template, title.as_deref(), description.as_deref())
        }
        Cmd::Reindex => cmd_reindex(cli),
        Cmd::Open { slug } => cmd_open(cli, slug.as_deref()),
        Cmd::PrintTemplate { name } => cmd_print_template(name),
        Cmd::ServeForeground { root, port } => {
            let listener = std::net::TcpListener::bind(("127.0.0.1", *port))
                .with_context(|| ExitError::ServerNotUp { timeout_ms: 0 })?;
            server_run(listener, root.clone())
        }
    }
}

fn resolve_root(cli: &Cli) -> Result<PathBuf> {
    if let Some(r) = &cli.root {
        return Ok(r.clone());
    }
    let cwd = std::env::current_dir().with_context(|| ExitError::Env("cwd".into()))?;
    let (root, found) = detect_project_root(&cwd);
    if !found && !cli.quiet {
        eprintln!("warn: no project marker found; using {}", root.display());
    }
    Ok(root)
}

fn cmd_start(cli: &Cli) -> Result<()> {
    let root = resolve_root(cli)?;
    ensure_runtime_dir(&root)?;
    if let Some(note) = ensure_gitignore(&root)? {
        if !cli.quiet {
            eprintln!("note: {note}");
        }
    }

    if let AliveStatus::Running(h) = probe(&root) {
        print_url(cli, h.port);
        return Ok(());
    }
    clear_handle(&root);

    let (listener, port) = pick_port(cli.port, 16)?;
    drop(listener);

    let exe = std::env::current_exe().with_context(|| ExitError::Internal("current_exe".into()))?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};
        let log = log_path(&root);
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .with_context(|| ExitError::Env(format!("open {}", log.display())))?;
        let log_clone = log_file
            .try_clone()
            .with_context(|| ExitError::Internal("clone log fd".into()))?;

        let mut cmd = Command::new(&exe);
        cmd.arg("serve-foreground")
            .arg("--root")
            .arg(&root)
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_clone));
        unsafe {
            cmd.pre_exec(|| {
                nix::unistd::setsid().ok();
                Ok(())
            });
        }
        let mut child = cmd
            .spawn()
            .with_context(|| ExitError::Internal("spawn child".into()))?;
        let pid = child.id();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if check_healthz(port) {
                write_handle(&root, &ServerHandle { pid, port })?;
                print_url(cli, port);
                std::mem::forget(child);
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        return Err(anyhow::anyhow!(ExitError::ServerNotUp { timeout_ms: 5000 }));
    }

    #[cfg(not(unix))]
    {
        let _ = exe;
        let _ = port;
        Err(anyhow::anyhow!(ExitError::Env(
            "Windows native is not supported in v1; use WSL2 instead".into()
        )))
    }
}

fn cmd_stop(cli: &Cli) -> Result<()> {
    let root = resolve_root(cli)?;
    let Some(h) = playctl::proc::read_handle(&root) else {
        return Ok(());
    };

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let _ = kill(Pid::from_raw(h.pid as i32), Signal::SIGTERM);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if !playctl::proc::is_alive(h.pid) {
                clear_handle(&root);
                if !cli.quiet {
                    println!("stopped");
                }
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = kill(Pid::from_raw(h.pid as i32), Signal::SIGKILL);
    }
    clear_handle(&root);
    if !cli.quiet {
        println!("stopped");
    }
    Ok(())
}

fn cmd_status(cli: &Cli) -> Result<()> {
    let root = resolve_root(cli)?;
    let count = read_index(&root).map(|i| i.playgrounds.len()).unwrap_or(0);
    match probe(&root) {
        AliveStatus::Running(h) => {
            let url = format!("http://127.0.0.1:{}", h.port);
            if cli.json {
                println!(
                    "{{\"state\":\"running\",\"url\":\"{}\",\"count\":{}}}",
                    url, count
                );
            } else {
                println!("running   {url}   {count} playgrounds");
            }
        }
        AliveStatus::Stale | AliveStatus::Absent => {
            if cli.json {
                println!("{{\"state\":\"stopped\"}}");
            } else {
                println!("stopped");
            }
        }
    }
    Ok(())
}

fn print_url(cli: &Cli, port: u16) {
    let url = format!("http://127.0.0.1:{port}/");
    if cli.json {
        println!("{{\"url\":\"{url}\"}}");
    } else {
        println!("{url}");
    }
}

fn cmd_list(_cli: &Cli) -> Result<()> {
    todo!("Task 15")
}
fn cmd_new(
    _cli: &Cli,
    _slug: &str,
    _template: &str,
    _title: Option<&str>,
    _desc: Option<&str>,
) -> Result<()> {
    todo!("Task 16")
}
fn cmd_reindex(_cli: &Cli) -> Result<()> {
    todo!("Task 15")
}
fn cmd_open(_cli: &Cli, _slug: Option<&str>) -> Result<()> {
    todo!("Task 17")
}
fn cmd_print_template(_name: &str) -> Result<()> {
    todo!("Task 18")
}
