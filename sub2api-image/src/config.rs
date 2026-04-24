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
    PathBuf::from(home).join(".sub2api-image").join("config.toml")
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
        if let Some(v) = saved_cfg { env::set_var("SUB2API_IMAGE_CONFIG", v); }
        if let Some(v) = saved_home { env::set_var("SUB2API_IMAGE_HOME", v); }
    }

    #[test]
    fn home_env_used_when_no_explicit_config() {
        let saved_cfg = env::var("SUB2API_IMAGE_CONFIG").ok();
        let saved_home = env::var("SUB2API_IMAGE_HOME").ok();

        env::remove_var("SUB2API_IMAGE_CONFIG");
        env::set_var("SUB2API_IMAGE_HOME", "/tmp/custom");
        assert_eq!(resolve_config_path(), PathBuf::from("/tmp/custom/config.toml"));

        env::remove_var("SUB2API_IMAGE_HOME");
        if let Some(v) = saved_cfg { env::set_var("SUB2API_IMAGE_CONFIG", v); }
        if let Some(v) = saved_home { env::set_var("SUB2API_IMAGE_HOME", v); }
    }
}
