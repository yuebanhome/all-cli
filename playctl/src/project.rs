use crate::error::ExitError;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const ROOT_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "Cargo.toml",
    "pnpm-workspace.yaml",
    "pyproject.toml",
    "go.mod",
];

/// 从 start 向上查找第一个含 ROOT_MARKERS 之一的目录。
/// 找不到 → 返回 cwd 并发出告警（让调用方决定是否 print）。
pub fn detect_project_root(start: &Path) -> (PathBuf, bool) {
    let mut cur: Option<&Path> = Some(start);
    while let Some(p) = cur {
        for m in ROOT_MARKERS {
            if p.join(m).exists() {
                return (p.to_path_buf(), true);
            }
        }
        cur = p.parent();
    }
    (start.to_path_buf(), false)
}

/// 必要时给项目根 .gitignore 追加 .playgrounds/。
/// 返回 Some(action) 表示有动作（用于 CLI 打印通知），None 表示 noop。
pub fn ensure_gitignore(project_root: &Path) -> Result<Option<&'static str>> {
    let gi = project_root.join(".gitignore");
    let has_git = project_root.join(".git").exists();

    if !gi.exists() {
        if !has_git {
            return Ok(None);
        }
        fs::write(&gi, ".playgrounds/\n")
            .with_context(|| ExitError::Env(format!("write {}", gi.display())))?;
        return Ok(Some("created .gitignore with .playgrounds/"));
    }

    let content = fs::read_to_string(&gi)
        .with_context(|| ExitError::Env(format!("read {}", gi.display())))?;
    let mentioned = content
        .lines()
        .any(|l| l.trim_start() == ".playgrounds/" || l.trim_start() == ".playgrounds");
    if mentioned {
        return Ok(None);
    }

    let mut new = content;
    if !new.ends_with('\n') {
        new.push('\n');
    }
    new.push_str("\n# live-playground\n.playgrounds/\n");
    fs::write(&gi, new)
        .with_context(|| ExitError::Env(format!("append {}", gi.display())))?;
    Ok(Some("appended .playgrounds/ to .gitignore"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn touch(p: &Path) {
        std::fs::write(p, "").unwrap();
    }

    #[test]
    fn detect_finds_git_dir() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        let (root, found) = detect_project_root(td.path());
        assert!(found);
        assert_eq!(root, td.path());
    }

    #[test]
    fn detect_walks_upward() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        let nested = td.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        let (root, found) = detect_project_root(&nested);
        assert!(found);
        assert_eq!(root, td.path());
    }

    #[test]
    fn detect_falls_back_to_cwd_with_flag() {
        let td = TempDir::new().unwrap();
        let (root, found) = detect_project_root(td.path());
        assert!(!found);
        assert_eq!(root, td.path());
    }

    #[test]
    fn detect_finds_cargo_toml() {
        let td = TempDir::new().unwrap();
        touch(&td.path().join("Cargo.toml"));
        let (_, found) = detect_project_root(td.path());
        assert!(found);
    }

    #[test]
    fn gitignore_skipped_without_git() {
        let td = TempDir::new().unwrap();
        let r = ensure_gitignore(td.path()).unwrap();
        assert!(r.is_none());
        assert!(!td.path().join(".gitignore").exists());
    }

    #[test]
    fn gitignore_created_with_git_present() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        let r = ensure_gitignore(td.path()).unwrap();
        assert_eq!(r, Some("created .gitignore with .playgrounds/"));
        let body = std::fs::read_to_string(td.path().join(".gitignore")).unwrap();
        assert!(body.contains(".playgrounds/"));
    }

    #[test]
    fn gitignore_noop_when_already_present() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        std::fs::write(td.path().join(".gitignore"), "target/\n.playgrounds/\n").unwrap();
        let r = ensure_gitignore(td.path()).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn gitignore_append_when_missing_line() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        std::fs::write(td.path().join(".gitignore"), "target/\n").unwrap();
        let r = ensure_gitignore(td.path()).unwrap();
        assert_eq!(r, Some("appended .playgrounds/ to .gitignore"));
        let body = std::fs::read_to_string(td.path().join(".gitignore")).unwrap();
        assert!(body.contains(".playgrounds/"));
        assert!(body.contains("target/"));
    }
}
