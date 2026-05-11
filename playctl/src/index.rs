use crate::error::ExitError;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Playground {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub template: String,
    /// RFC3339 / ISO8601 with Z suffix.
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Index {
    pub version: u32,
    pub project_name: String,
    pub playgrounds: Vec<Playground>,
}

impl Index {
    pub fn empty(project_name: &str) -> Self {
        Self {
            version: 1,
            project_name: project_name.to_string(),
            playgrounds: vec![],
        }
    }
}

/// .playgrounds/index.json 路径。
pub fn index_path(project_root: &Path) -> PathBuf {
    project_root.join(".playgrounds").join("index.json")
}

pub fn read_index(project_root: &Path) -> Result<Index> {
    let p = index_path(project_root);
    if !p.exists() {
        let name = project_root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".into());
        return Ok(Index::empty(&name));
    }
    let body =
        fs::read_to_string(&p).with_context(|| ExitError::Env(format!("read {}", p.display())))?;
    let idx: Index = serde_json::from_str(&body)
        .with_context(|| ExitError::Env(format!("parse {}", p.display())))?;
    if idx.version != 1 {
        return Err(anyhow!(ExitError::Env(format!(
            "{} has unsupported version {}",
            p.display(),
            idx.version
        ))));
    }
    Ok(idx)
}

pub fn write_index(project_root: &Path, idx: &Index) -> Result<()> {
    let p = index_path(project_root);
    fs::create_dir_all(p.parent().unwrap())
        .with_context(|| ExitError::Env(format!("mkdir {}", p.parent().unwrap().display())))?;
    let body = serde_json::to_string_pretty(idx)
        .map_err(|e| anyhow!(ExitError::Internal(format!("encode index.json: {e}"))))?;
    fs::write(&p, body).with_context(|| ExitError::Env(format!("write {}", p.display())))?;
    Ok(())
}

/// 扫描 .playgrounds/<slug>/index.html 重建 index.json。
/// title / description 取自 HTML <title> 和 <meta name="description">；template 留空（reindex 出来的条目可能没有该信息）。
pub fn reindex(project_root: &Path) -> Result<Index> {
    let dir = project_root.join(".playgrounds");
    let project_name = project_root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let mut idx = Index::empty(&project_name);

    if !dir.exists() {
        return Ok(idx);
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .with_context(|| ExitError::Env(format!("read_dir {}", dir.display())))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let html = entry.path().join("index.html");
        if !html.exists() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().into_owned();
        let body = fs::read_to_string(&html)
            .with_context(|| ExitError::Env(format!("read {}", html.display())))?;
        let title = extract_title(&body).unwrap_or_else(|| slug.clone());
        let description = extract_description(&body).unwrap_or_default();
        let created_at = file_mtime_iso(&html).unwrap_or_else(now_iso);
        idx.playgrounds.push(Playground {
            slug,
            title,
            description,
            template: String::new(),
            created_at,
        });
    }
    Ok(idx)
}

pub fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn file_mtime_iso(p: &Path) -> Option<String> {
    let m = fs::metadata(p).ok()?.modified().ok()?;
    let dt: OffsetDateTime = m.into();
    dt.format(&time::format_description::well_known::Rfc3339)
        .ok()
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end_rel = lower[start..].find("</title>")?;
    Some(html[start..start + end_rel].trim().to_string())
}

fn extract_description(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let needle = "name=\"description\"";
    let pos = lower.find(needle)?;
    let after = &html[pos..];
    let content_idx = after.to_ascii_lowercase().find("content=\"")? + "content=\"".len();
    let after2 = &after[content_idx..];
    let end = after2.find('"')?;
    Some(after2[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_index_has_version_1() {
        let i = Index::empty("foo");
        assert_eq!(i.version, 1);
        assert_eq!(i.project_name, "foo");
        assert!(i.playgrounds.is_empty());
    }

    #[test]
    fn read_returns_empty_when_missing() {
        let td = TempDir::new().unwrap();
        let i = read_index(td.path()).unwrap();
        assert_eq!(i.version, 1);
        assert!(i.playgrounds.is_empty());
    }

    #[test]
    fn round_trip() {
        let td = TempDir::new().unwrap();
        let mut i = Index::empty("p");
        i.playgrounds.push(Playground {
            slug: "x".into(),
            title: "X".into(),
            description: "d".into(),
            template: "design-playground".into(),
            created_at: "2026-05-09T12:00:00Z".into(),
        });
        write_index(td.path(), &i).unwrap();
        let i2 = read_index(td.path()).unwrap();
        assert_eq!(i, i2);
    }

    #[test]
    fn extract_title_works() {
        assert_eq!(
            extract_title("<html><head><TITLE>Hello</TITLE></head>"),
            Some("Hello".into())
        );
        assert_eq!(extract_title("no title"), None);
    }

    #[test]
    fn extract_description_works() {
        let s = r#"<meta name="description" content="my desc">"#;
        assert_eq!(extract_description(s), Some("my desc".into()));
    }

    #[test]
    fn reindex_picks_up_html_files() {
        let td = TempDir::new().unwrap();
        let pg = td.path().join(".playgrounds/foo");
        fs::create_dir_all(&pg).unwrap();
        fs::write(
            pg.join("index.html"),
            r#"<html><head><title>Foo</title>
<meta name="description" content="desc"></head></html>"#,
        )
        .unwrap();
        let i = reindex(td.path()).unwrap();
        assert_eq!(i.playgrounds.len(), 1);
        assert_eq!(i.playgrounds[0].slug, "foo");
        assert_eq!(i.playgrounds[0].title, "Foo");
        assert_eq!(i.playgrounds[0].description, "desc");
    }

    #[test]
    fn reindex_skips_dotted_dirs() {
        let td = TempDir::new().unwrap();
        let runtime = td.path().join(".playgrounds/.runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("index.html"), "<title>X</title>").unwrap();
        let i = reindex(td.path()).unwrap();
        assert!(i.playgrounds.is_empty());
    }

    #[test]
    fn unsupported_version_rejected() {
        let td = TempDir::new().unwrap();
        let path = index_path(td.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"version":2,"project_name":"p","playgrounds":[]}"#,
        )
        .unwrap();
        let err = read_index(td.path()).unwrap_err();
        assert_eq!(crate::error::classify(&err), 2);
    }
}
