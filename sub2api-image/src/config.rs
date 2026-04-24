use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

pub fn resolve_config_path() -> PathBuf {
    if let Ok(p) = env::var("SUB2API_IMAGE_CONFIG") {
        return PathBuf::from(p);
    }
    if let Ok(home) = env::var("SUB2API_IMAGE_HOME") {
        return PathBuf::from(home).join("config.toml");
    }
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join(".sub2api-image")
        .join("config.toml")
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
    let path = resolve_config_path();
    let s = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "config error: cannot read {}, run `sub2api-image --init` to create",
            path.display()
        )
    })?;
    let cfg: Config = toml::from_str(&s)
        .with_context(|| format!("config error: cannot parse TOML at {}", path.display()))?;
    if cfg.base_url.trim().is_empty() {
        bail!("config error: base_url is required");
    }
    if cfg.api_key.trim().is_empty() {
        bail!("config error: api_key is required");
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

pub fn write_template() -> Result<std::path::PathBuf> {
    let path = resolve_config_path();
    if path.exists() {
        bail!(
            "config error: config already exists at {}, edit manually",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("io error: cannot create config dir {}", parent.display()))?;
    }
    std::fs::write(&path, TEMPLATE)
        .with_context(|| format!("io error: cannot write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试环境变量优先级。注意：env 测试默认串行跑（单进程全局状态）。
    #[test]
    fn explicit_config_env_wins() {
        // 保存原值
        let saved_cfg = env::var("SUB2API_IMAGE_CONFIG").ok();
        let saved_home = env::var("SUB2API_IMAGE_HOME").ok();

        env::set_var("SUB2API_IMAGE_CONFIG", "/tmp/test-cfg.toml");
        env::set_var("SUB2API_IMAGE_HOME", "/tmp/ignored");
        assert_eq!(resolve_config_path(), PathBuf::from("/tmp/test-cfg.toml"));

        // 恢复
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::remove_var("SUB2API_IMAGE_HOME");
        if let Some(v) = saved_cfg {
            env::set_var("SUB2API_IMAGE_CONFIG", v);
        }
        if let Some(v) = saved_home {
            env::set_var("SUB2API_IMAGE_HOME", v);
        }
    }

    #[test]
    fn home_env_used_when_no_explicit_config() {
        let saved_cfg = env::var("SUB2API_IMAGE_CONFIG").ok();
        let saved_home = env::var("SUB2API_IMAGE_HOME").ok();

        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::set_var("SUB2API_IMAGE_HOME", "/tmp/custom");
        assert_eq!(
            resolve_config_path(),
            PathBuf::from("/tmp/custom/config.toml")
        );

        env::remove_var("SUB2API_IMAGE_HOME");
        if let Some(v) = saved_cfg {
            env::set_var("SUB2API_IMAGE_CONFIG", v);
        }
        if let Some(v) = saved_home {
            env::set_var("SUB2API_IMAGE_HOME", v);
        }
    }

    use tempfile::TempDir;

    fn isolate_env(home: &std::path::Path) -> EnvGuard {
        let guard = EnvGuard::new();
        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::set_var("SUB2API_IMAGE_HOME", home);
        guard
    }

    struct EnvGuard {
        cfg: Option<String>,
        home: Option<String>,
    }
    impl EnvGuard {
        fn new() -> Self {
            Self {
                cfg: env::var("SUB2API_IMAGE_CONFIG").ok(),
                home: env::var("SUB2API_IMAGE_HOME").ok(),
            }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            env::remove_var("SUB2API_IMAGE_CONFIG");
            env::remove_var("SUB2API_IMAGE_HOME");
            if let Some(v) = &self.cfg {
                env::set_var("SUB2API_IMAGE_CONFIG", v);
            }
            if let Some(v) = &self.home {
                env::set_var("SUB2API_IMAGE_HOME", v);
            }
        }
    }

    #[test]
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
        let _g = isolate_env(dir.path());
        let cfg = load().unwrap();
        assert_eq!(cfg.base_url, "https://x.example.com");
        assert_eq!(cfg.api_key, "sk-abc");
        assert_eq!(cfg.defaults.model.as_deref(), Some("gpt-image-2"));
        assert_eq!(cfg.defaults.size.as_deref(), Some("1024x1024"));
    }

    #[test]
    fn load_without_defaults_ok() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"base_url = "https://x.example.com"
api_key = "sk-abc"
"#,
        )
        .unwrap();
        let _g = isolate_env(dir.path());
        let cfg = load().unwrap();
        assert!(cfg.defaults.model.is_none());
    }

    #[test]
    fn load_missing_api_key_errors() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"base_url = "https://x.example.com"
api_key = ""
"#,
        )
        .unwrap();
        let _g = isolate_env(dir.path());
        let err = load().unwrap_err();
        assert!(err.to_string().contains("config error"));
        assert!(err.to_string().contains("api_key"));
    }

    #[test]
    fn load_missing_file_errors() {
        let dir = TempDir::new().unwrap();
        let _g = isolate_env(dir.path());
        let err = load().unwrap_err();
        assert!(err.to_string().contains("config error"));
    }

    #[test]
    fn load_bad_toml_errors() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.toml"), r#"not valid toml ][["#).unwrap();
        let _g = isolate_env(dir.path());
        let err = load().unwrap_err();
        assert!(err.to_string().contains("config error"));
    }

    #[test]
    fn write_template_creates_file() {
        let dir = TempDir::new().unwrap();
        let _g = isolate_env(dir.path());
        let path = write_template().unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("REPLACE_ME"));
    }

    #[test]
    fn write_template_refuses_overwrite() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("config.toml"), "existing").unwrap();
        let _g = isolate_env(dir.path());
        let err = write_template().unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn template_roundtrip_parses() {
        // 模板内容应能被 TOML 解析器接受（即使字段是 REPLACE_ME）
        let _: toml::Value = toml::from_str(TEMPLATE).expect("template must parse");
    }
}
