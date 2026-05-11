/// Playground slug 校验：单点维护，避免 server.rs / main.rs 各写一份。
///
/// 规则与 spec §3 一致：`[a-z0-9][a-z0-9-]{0,63}`。
/// - 长度 1..=64
/// - 首字符必须是 ascii 小写字母或数字
/// - 其余字符 ascii 小写字母、数字、或连字符 `-`
///
/// 不接受大写、下划线、点、斜杠、percent-encoded 序列等，任何此外的输入
/// 都会被 HTTP 路由层拒绝（404），CLI 也会拒绝创建。
pub fn is_valid_slug(s: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_slugs() {
        assert!(is_valid_slug("foo"));
        assert!(is_valid_slug("foo-bar"));
        assert!(is_valid_slug("a1b2"));
        assert!(is_valid_slug("0"));
        assert!(is_valid_slug(&"a".repeat(64)));
    }

    #[test]
    fn rejects_invalid_slugs() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("Foo"));
        assert!(!is_valid_slug("-foo"));
        assert!(!is_valid_slug("foo_bar"));
        assert!(!is_valid_slug("foo.bar"));
        assert!(!is_valid_slug("foo/bar"));
        assert!(!is_valid_slug(".."));
        assert!(!is_valid_slug("../etc"));
        assert!(!is_valid_slug(&"a".repeat(65)));
    }
}
