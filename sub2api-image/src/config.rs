use crate::errors::AppError;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

/// 解析配置文件路径。优先级：
/// 1. `SUB2API_IMAGE_CONFIG`（绝对路径）
/// 2. `SUB2API_IMAGE_HOME` + `/config.toml`
/// 3. user home + `/.sub2api-image/config.toml`（Linux/macOS 用 HOME，Windows 用 USERPROFILE）
///
/// 三者皆缺失时返回 AppError::Config，避免落到相对路径。
pub fn resolve_config_path() -> Result<PathBuf> {
    if let Ok(p) = env::var("SUB2API_IMAGE_CONFIG") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    if let Ok(home) = env::var("SUB2API_IMAGE_HOME") {
        if !home.is_empty() {
            return Ok(PathBuf::from(home).join("config.toml"));
        }
    }
    let home = home_dir().ok_or_else(|| {
        AppError::Config(
            "cannot locate user home directory; set SUB2API_IMAGE_CONFIG or SUB2API_IMAGE_HOME"
                .into(),
        )
    })?;
    Ok(home.join(".sub2api-image").join("config.toml"))
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(h) = env::var("HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    if let Ok(h) = env::var("USERPROFILE") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    None
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Deserialize, Default)]
pub struct Defaults {
    pub model: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
}

pub fn load() -> Result<Config> {
    let path = resolve_config_path()?;
    let s = std::fs::read_to_string(&path).map_err(|e| {
        AppError::Config(format!(
            "cannot read {}: {}, run `sub2api-image --init` to create",
            path.display(),
            e
        ))
    })?;
    let cfg: Config = toml::from_str(&s)
        .map_err(|e| AppError::Config(format!("cannot parse TOML at {}: {}", path.display(), e)))?;
    if cfg.base_url.trim().is_empty() {
        return Err(AppError::Config("base_url is required".into()).into());
    }
    if cfg.api_key.trim().is_empty() {
        return Err(AppError::Config("api_key is required".into()).into());
    }
    Ok(cfg)
}

const TEMPLATE: &str = r#"# sub2api-image config
base_url = "https://REPLACE_ME.example.com"
api_key  = "REPLACE_ME"

[defaults]
model   = "gpt-image-2"
size    = "auto"
quality = "auto"
"#;

pub fn write_template() -> Result<PathBuf> {
    let path = resolve_config_path()?;
    if path.exists() {
        return Err(AppError::Config(format!(
            "config already exists at {}, edit manually",
            path.display()
        ))
        .into());
    }
    if let Some(parent) = path.parent() {
        let parent_existed_before = parent.exists();
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::Io(format!(
                "cannot create config dir {}: {}",
                parent.display(),
                e
            ))
        })?;
        // 只在"本次由我们新建"的目录上收紧到 0700，避免误改用户已有的目录（如 /tmp）
        #[cfg(unix)]
        if !parent_existed_before {
            restrict_perms(parent, 0o700)
                .with_context(|| format!("cannot tighten permissions on {}", parent.display()))?;
        }
    }
    std::fs::write(&path, TEMPLATE)
        .map_err(|e| AppError::Io(format!("cannot write {}: {}", path.display(), e)))?;
    // Unix 上把文件设为 0600，因为里面会落明文 api_key
    #[cfg(unix)]
    restrict_perms(&path, 0o600)
        .with_context(|| format!("cannot tighten permissions on {}", path.display()))?;
    Ok(path)
}

#[cfg(unix)]
fn restrict_perms(path: &std::path::Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    struct EnvGuard {
        cfg: Option<String>,
        home: Option<String>,
        sys_home: Option<String>,
        sys_userprofile: Option<String>,
    }
    impl EnvGuard {
        fn new() -> Self {
            Self {
                cfg: env::var("SUB2API_IMAGE_CONFIG").ok(),
                home: env::var("SUB2API_IMAGE_HOME").ok(),
                sys_home: env::var("HOME").ok(),
                sys_userprofile: env::var("USERPROFILE").ok(),
            }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for k in [
                "SUB2API_IMAGE_CONFIG",
                "SUB2API_IMAGE_HOME",
                "HOME",
                "USERPROFILE",
            ] {
                env::remove_var(k);
            }
            if let Some(v) = &self.cfg {
                env::set_var("SUB2API_IMAGE_CONFIG", v);
            }
            if let Some(v) = &self.home {
                env::set_var("SUB2API_IMAGE_HOME", v);
            }
            if let Some(v) = &self.sys_home {
                env::set_var("HOME", v);
            }
            if let Some(v) = &self.sys_userprofile {
                env::set_var("USERPROFILE", v);
            }
        }
    }

    fn isolate_env_with_home(home: &std::path::Path) -> EnvGuard {
        let g = EnvGuard::new();
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::set_var("SUB2API_IMAGE_HOME", home);
        g
    }

    #[test]
    #[serial]
    fn explicit_config_env_wins() {
        let _g = EnvGuard::new();
        env::set_var("SUB2API_IMAGE_CONFIG", "/tmp/test-cfg.toml");
        env::set_var("SUB2API_IMAGE_HOME", "/tmp/ignored");
        assert_eq!(
            resolve_config_path().unwrap(),
            PathBuf::from("/tmp/test-cfg.toml")
        );
    }

    #[test]
    #[serial]
    fn home_env_used_when_no_explicit_config() {
        let _g = EnvGuard::new();
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::set_var("SUB2API_IMAGE_HOME", "/tmp/custom");
        assert_eq!(
            resolve_config_path().unwrap(),
            PathBuf::from("/tmp/custom/config.toml")
        );
    }

    #[test]
    #[serial]
    fn falls_back_to_home_var() {
        let _g = EnvGuard::new();
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::remove_var("SUB2API_IMAGE_HOME");
        env::remove_var("USERPROFILE");
        env::set_var("HOME", "/home/me");
        assert_eq!(
            resolve_config_path().unwrap(),
            PathBuf::from("/home/me/.sub2api-image/config.toml")
        );
    }

    #[test]
    #[serial]
    fn falls_back_to_userprofile_when_home_missing() {
        let _g = EnvGuard::new();
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::remove_var("SUB2API_IMAGE_HOME");
        env::remove_var("HOME");
        env::set_var("USERPROFILE", "C:\\Users\\me");
        let p = resolve_config_path().unwrap();
        assert!(
            p.ends_with(".sub2api-image/config.toml") || p.ends_with(".sub2api-image\\config.toml")
        );
    }

    #[test]
    #[serial]
    fn errors_when_no_home_at_all() {
        let _g = EnvGuard::new();
        for k in [
            "SUB2API_IMAGE_CONFIG",
            "SUB2API_IMAGE_HOME",
            "HOME",
            "USERPROFILE",
        ] {
            env::remove_var(k);
        }
        let err = resolve_config_path().unwrap_err();
        assert_eq!(crate::errors::exit_code_from(&err), 2);
        assert!(err.to_string().contains("home directory"));
    }

    #[test]
    #[serial]
    fn empty_env_treated_as_unset() {
        let _g = EnvGuard::new();
        env::set_var("SUB2API_IMAGE_CONFIG", "");
        env::set_var("SUB2API_IMAGE_HOME", "");
        env::set_var("HOME", "/home/me");
        assert_eq!(
            resolve_config_path().unwrap(),
            PathBuf::from("/home/me/.sub2api-image/config.toml")
        );
    }

    #[test]
    #[serial]
    fn load_full_config_ok() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"base_url = "https://x.example.com"
api_key = "sk-abc"

[defaults]
model = "gpt-image-2"
size = "1024x1024"
quality = "high"
"#,
        )
        .unwrap();
        let _g = isolate_env_with_home(dir.path());
        let cfg = load().unwrap();
        assert_eq!(cfg.base_url, "https://x.example.com");
        assert_eq!(cfg.api_key, "sk-abc");
        assert_eq!(cfg.defaults.model.as_deref(), Some("gpt-image-2"));
        assert_eq!(cfg.defaults.size.as_deref(), Some("1024x1024"));
    }

    #[test]
    #[serial]
    fn load_without_defaults_ok() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"base_url = "https://x.example.com"
api_key = "sk-abc"
"#,
        )
        .unwrap();
        let _g = isolate_env_with_home(dir.path());
        let cfg = load().unwrap();
        assert!(cfg.defaults.model.is_none());
    }

    #[test]
    #[serial]
    fn load_missing_api_key_errors() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"base_url = "https://x.example.com"
api_key = ""
"#,
        )
        .unwrap();
        let _g = isolate_env_with_home(dir.path());
        let err = load().unwrap_err();
        assert_eq!(crate::errors::exit_code_from(&err), 2);
        assert!(err.to_string().contains("api_key"));
    }

    #[test]
    #[serial]
    fn load_missing_file_errors() {
        let dir = TempDir::new().unwrap();
        let _g = isolate_env_with_home(dir.path());
        let err = load().unwrap_err();
        assert_eq!(crate::errors::exit_code_from(&err), 2);
    }

    #[test]
    #[serial]
    fn load_bad_toml_errors() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.toml"), r#"not valid toml ][["#).unwrap();
        let _g = isolate_env_with_home(dir.path());
        let err = load().unwrap_err();
        assert_eq!(crate::errors::exit_code_from(&err), 2);
    }

    #[test]
    #[serial]
    fn write_template_creates_file() {
        let dir = TempDir::new().unwrap();
        let _g = isolate_env_with_home(dir.path());
        let path = write_template().unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("REPLACE_ME"));
    }

    #[test]
    #[serial]
    #[cfg(unix)]
    fn write_template_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let _g = isolate_env_with_home(dir.path());
        let path = write_template().unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config file must be 0600, got {:o}", mode);
    }

    #[test]
    #[serial]
    fn write_template_refuses_overwrite() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("config.toml"), "existing").unwrap();
        let _g = isolate_env_with_home(dir.path());
        let err = write_template().unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    #[serial]
    fn template_roundtrip_parses() {
        let _: toml::Value = toml::from_str(TEMPLATE).expect("template must parse");
    }
}
