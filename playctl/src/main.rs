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
        Cmd::New {
            slug,
            template,
            title,
            description,
        } => cmd_new(
            cli,
            slug,
            template,
            title.as_deref(),
            description.as_deref(),
        ),
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
        Err(anyhow::anyhow!(ExitError::ServerNotUp { timeout_ms: 5000 }))
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

fn cmd_list(cli: &Cli) -> Result<()> {
    let root = resolve_root(cli)?;
    let idx = read_index(&root)?;
    if cli.json {
        let body = serde_json::to_string(&idx)
            .with_context(|| ExitError::Internal("encode index".into()))?;
        println!("{body}");
        return Ok(());
    }
    if idx.playgrounds.is_empty() {
        if !cli.quiet {
            println!("no playgrounds in {}", root.display());
        }
        return Ok(());
    }
    let port = playctl::proc::read_handle(&root)
        .map(|h| h.port)
        .unwrap_or(cli.port);
    println!("{:<24} {:<28} {:<32} CREATED", "SLUG", "TITLE", "URL");
    for p in &idx.playgrounds {
        println!(
            "{:<24} {:<28} {:<32} {}",
            truncate(&p.slug, 24),
            truncate(&p.title, 28),
            format!("http://127.0.0.1:{port}/{}/", p.slug),
            p.created_at
        );
    }
    Ok(())
}
fn cmd_new(
    cli: &Cli,
    slug: &str,
    template: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    if !is_valid_new_slug(slug) {
        return Err(anyhow::anyhow!(ExitError::User(format!(
            "invalid slug '{slug}'; must match [a-z0-9][a-z0-9-]{{0,63}}"
        ))));
    }
    if !playctl::templates::VALID_TEMPLATE_NAMES.contains(&template) {
        return Err(anyhow::anyhow!(ExitError::User(format!(
            "unknown template '{template}'; valid: {}",
            playctl::templates::VALID_TEMPLATE_NAMES.join(", ")
        ))));
    }

    let root = resolve_root(cli)?;
    let dir = root.join(".playgrounds").join(slug);
    if dir.exists() {
        return Err(anyhow::anyhow!(ExitError::User(format!(
            "{} already exists",
            dir.display()
        ))));
    }
    std::fs::create_dir_all(&dir)
        .with_context(|| ExitError::Env(format!("mkdir {}", dir.display())))?;

    let title_owned = title.map(str::to_string).unwrap_or_else(|| humanize(slug));
    let desc_owned = description.unwrap_or("").to_string();
    let html = playctl::templates::html_scaffold(&title_owned, &desc_owned);
    std::fs::write(dir.join("index.html"), html)
        .with_context(|| ExitError::Env(format!("write {}/index.html", dir.display())))?;

    // 更新 index.json
    let mut idx = read_index(&root)?;
    idx.playgrounds.push(playctl::index::Playground {
        slug: slug.to_string(),
        title: title_owned,
        description: desc_owned,
        template: template.to_string(),
        created_at: playctl::index::now_iso(),
    });
    playctl::index::write_index(&root, &idx)?;

    // 自启动（若未运行）
    let port = match probe(&root) {
        AliveStatus::Running(h) => h.port,
        _ => {
            cmd_start(cli)?;
            playctl::proc::read_handle(&root)
                .map(|h| h.port)
                .unwrap_or(cli.port)
        }
    };

    let url = format!("http://127.0.0.1:{port}/{slug}/");
    if cli.json {
        println!("{{\"url\":\"{url}\"}}");
    } else {
        println!("{url}");
    }
    Ok(())
}
fn cmd_reindex(cli: &Cli) -> Result<()> {
    let root = resolve_root(cli)?;
    let mut new = playctl::index::reindex(&root)?;
    // 保留 template 字段（旧 index 里有，HTML 里没有）
    if let Ok(old) = read_index(&root) {
        for p in &mut new.playgrounds {
            if let Some(prev) = old.playgrounds.iter().find(|x| x.slug == p.slug) {
                if !prev.template.is_empty() {
                    p.template = prev.template.clone();
                }
            }
        }
    }
    playctl::index::write_index(&root, &new)?;
    if !cli.quiet {
        println!("reindexed {} playground(s)", new.playgrounds.len());
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn is_valid_new_slug(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn humanize(slug: &str) -> String {
    slug.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cmd_open(cli: &Cli, slug: Option<&str>) -> Result<()> {
    let root = resolve_root(cli)?;
    let port = match probe(&root) {
        AliveStatus::Running(h) => h.port,
        _ => {
            cmd_start(cli)?;
            playctl::proc::read_handle(&root)
                .map(|h| h.port)
                .unwrap_or(cli.port)
        }
    };
    let url = match slug {
        Some(s) => format!("http://127.0.0.1:{port}/{s}/"),
        None => format!("http://127.0.0.1:{port}/"),
    };
    let opened = open_url(&url);
    if !opened && !cli.quiet {
        println!("(could not auto-open) {url}");
    } else if !cli.quiet {
        println!("{url}");
    }
    Ok(())
}

fn open_url(url: &str) -> bool {
    use std::process::Command;

    // WSL2 优先尝试 wslview，再 fallback 到 cmd.exe
    if is_wsl() {
        if Command::new("wslview")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
        if Command::new("cmd.exe")
            .args(["/c", "start", "", url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("cmd.exe")
            .args(["/c", "start", "", url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn is_wsl() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    std::fs::read_to_string("/proc/version")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}
fn cmd_print_template(name: &str) -> Result<()> {
    let body = playctl::templates::read_template(name)?;
    print!("{body}");
    Ok(())
}
