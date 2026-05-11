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

/// 从 HTML 中提取 `<meta name="description" content="...">` 的 content 值。
///
/// 顺序无关：`<meta content="..." name="description">` 也能识别。
/// 引号支持双引号和单引号，属性匹配大小写不敏感。
/// 只读第一个匹配的 meta 标签，best-effort，遇到畸形结构返回 None。
fn extract_description(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<meta") {
        let tag_start = cursor + rel;
        // 标签后必须紧跟空白 / `>`，避免误匹配 `<metafoo>`。
        let next_byte = lower.as_bytes().get(tag_start + 5).copied();
        let is_meta = matches!(next_byte, Some(b) if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == b'>' || b == b'/');
        if !is_meta {
            cursor = tag_start + 5;
            continue;
        }
        let after = &lower[tag_start..];
        let Some(end_rel) = after.find('>') else {
            break;
        };
        let tag_lower = &after[..end_rel];
        let tag_orig = &html[tag_start..tag_start + end_rel];
        let name = attr_value(tag_lower, tag_orig, "name");
        let is_description = name
            .map(|v| v.eq_ignore_ascii_case("description"))
            .unwrap_or(false);
        if is_description {
            if let Some(content) = attr_value(tag_lower, tag_orig, "content") {
                return Some(content.trim().to_string());
            }
        }
        cursor = tag_start + end_rel + 1;
    }
    None
}

/// 在已经截到 `<` 与 `>` 之间的 tag 字符串里找属性值。
///
/// `tag_lower` 和 `tag_orig` 字节长度必须一致（来自同一段切片）：用 lower
/// 找属性边界与名字，用 orig 切取大小写敏感的值。
fn attr_value<'a>(tag_lower: &str, tag_orig: &'a str, attr: &str) -> Option<&'a str> {
    debug_assert_eq!(tag_lower.len(), tag_orig.len());
    let needle = format!("{attr}=");
    let mut pos = 0;
    while let Some(rel) = tag_lower[pos..].find(&needle) {
        let idx = pos + rel;
        // 属性名前必须是空白（或紧跟 `<meta` 即 idx==5）；否则像 `xname=` 这种
        // 子串匹配要跳过。
        let prev_ok = idx == 0
            || idx == 5
            || tag_lower.as_bytes()[idx - 1].is_ascii_whitespace()
            || tag_lower.as_bytes()[idx - 1] == b'/';
        if !prev_ok {
            pos = idx + needle.len();
            continue;
        }
        let value_start = idx + needle.len();
        let bytes = tag_orig.as_bytes();
        let first = *bytes.get(value_start)?;
        let (body_start, terminator): (usize, &[u8]) = match first {
            b'"' => (value_start + 1, b"\""),
            b'\'' => (value_start + 1, b"'"),
            _ => {
                // 无引号属性值：以空白或标签结束截止
                let end_off = tag_orig[value_start..]
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(tag_orig.len() - value_start);
                return Some(&tag_orig[value_start..value_start + end_off]);
            }
        };
        let rest = &tag_orig[body_start..];
        let end_rel = rest.find(|c: char| c == terminator[0] as char)?;
        return Some(&rest[..end_rel]);
    }
    None
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
    fn extract_description_handles_reversed_attribute_order() {
        // content 在 name 之前 —— 旧的 contains 顺序解析会丢失这种 HTML。
        let s = r#"<meta content="reverse order" name="description">"#;
        assert_eq!(extract_description(s), Some("reverse order".into()));
    }

    #[test]
    fn extract_description_handles_single_quotes() {
        let s = r#"<meta name='description' content='sq desc'>"#;
        assert_eq!(extract_description(s), Some("sq desc".into()));
    }

    #[test]
    fn extract_description_skips_unrelated_meta() {
        let s = r#"<meta charset="utf-8">
<meta name="viewport" content="width=device-width">
<meta content="found" name="description">"#;
        assert_eq!(extract_description(s), Some("found".into()));
    }

    #[test]
    fn extract_description_no_match_returns_none() {
        let s = r#"<meta name="author" content="someone">"#;
        assert_eq!(extract_description(s), None);
    }

    #[test]
    fn extract_description_case_insensitive_attribute() {
        let s = r#"<META NAME="DESCRIPTION" CONTENT="upper">"#;
        assert_eq!(extract_description(s), Some("upper".into()));
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
