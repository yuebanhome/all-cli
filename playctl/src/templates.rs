use crate::error::ExitError;
use anyhow::{anyhow, Result};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub const VALID_TEMPLATE_NAMES: &[&str] = &[
    "design-playground",
    "data-explorer",
    "concept-map",
    "document-critique",
    "diff-review",
    "code-map",
];

/// 读取模板原文。name 不带 .md 后缀。
pub fn read_template(name: &str) -> Result<String> {
    let file = format!("{}.md", name);
    match Templates::get(&file) {
        Some(f) => {
            let bytes = f.data.into_owned();
            String::from_utf8(bytes)
                .map_err(|e| anyhow!(ExitError::Internal(format!("template utf8: {e}"))))
        }
        None => Err(anyhow!(ExitError::User(format!(
            "unknown template '{}'; valid: {}",
            name,
            VALID_TEMPLATE_NAMES.join(", ")
        )))),
    }
}

/// 读取静态资源（用于 server 渲染首页等）。
pub fn read_asset(name: &str) -> Result<Vec<u8>> {
    Assets::get(name)
        .map(|f| f.data.into_owned().to_vec())
        .ok_or_else(|| anyhow!(ExitError::Internal(format!("asset missing: {name}"))))
}

/// 给 `playctl new` 用的 HTML 脚手架（与模板正交，刻意保持极简）。
pub fn html_scaffold(title: &str, description: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<style>
:root {{ color-scheme: dark; }}
body {{ font-family: system-ui,-apple-system,sans-serif; background:#0b0b0b; color:#eaeaea; margin:0; padding:24px; }}
main {{ max-width: 960px; margin: 0 auto; }}
</style>
</head>
<body>
<main>
  <h1>{title}</h1>
  <p>{desc}</p>
  <!-- TODO(claude): fill controls + preview + prompt block per template guidance -->
</main>
<script>
// TODO(claude): wire up controls → preview → prompt-with-copy
</script>
</body>
</html>
"#,
        title = html_escape(title),
        desc = html_escape(description),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_template_is_user_error() {
        let err = read_template("does-not-exist").unwrap_err();
        assert_eq!(crate::error::classify(&err), 1);
    }

    #[test]
    fn html_escape_basics() {
        assert_eq!(html_escape("a<b&c\""), "a&lt;b&amp;c&quot;");
    }

    #[test]
    fn scaffold_contains_title() {
        let s = html_scaffold("My Tool", "do stuff");
        assert!(s.contains("<title>My Tool</title>"));
        assert!(s.contains("do stuff"));
    }
}
