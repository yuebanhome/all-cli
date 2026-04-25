use std::fmt;

/// 应用错误分类。所有 bail/Err 路径都应通过本枚举构造，避免依赖错误消息字符串前缀。
#[derive(Debug)]
pub enum AppError {
    Config(String),
    Input(String),
    Network(String),
    Api(String),
    Parse(String),
    Io(String),
}

impl AppError {
    pub fn exit_code(&self) -> u8 {
        match self {
            AppError::Config(_) | AppError::Input(_) => 2,
            AppError::Network(_) => 3,
            AppError::Api(_) => 4,
            AppError::Parse(_) => 5,
            AppError::Io(_) => 6,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(s) => write!(f, "config error: {s}"),
            AppError::Input(s) => write!(f, "input error: {s}"),
            AppError::Network(s) => write!(f, "network error: {s}"),
            AppError::Api(s) => write!(f, "api error: {s}"),
            AppError::Parse(s) => write!(f, "response parse error: {s}"),
            AppError::Io(s) => write!(f, "io error: {s}"),
        }
    }
}

impl std::error::Error for AppError {}

/// 从 anyhow::Error 链里 downcast 出 AppError 取其 exit_code。anyhow::downcast_ref 会遍历整条 context 链。
pub fn exit_code_from(err: &anyhow::Error) -> u8 {
    err.downcast_ref::<AppError>()
        .map(AppError::exit_code)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_maps_to_2() {
        let err: anyhow::Error = AppError::Config("api_key missing".into()).into();
        assert_eq!(exit_code_from(&err), 2);
    }

    #[test]
    fn input_error_maps_to_2() {
        let err: anyhow::Error = AppError::Input("--prompt required".into()).into();
        assert_eq!(exit_code_from(&err), 2);
    }

    #[test]
    fn network_error_maps_to_3() {
        let err: anyhow::Error = AppError::Network("timed out".into()).into();
        assert_eq!(exit_code_from(&err), 3);
    }

    #[test]
    fn api_error_maps_to_4() {
        let err: anyhow::Error = AppError::Api("401 Invalid API key".into()).into();
        assert_eq!(exit_code_from(&err), 4);
    }

    #[test]
    fn parse_error_maps_to_5() {
        let err: anyhow::Error = AppError::Parse("data empty".into()).into();
        assert_eq!(exit_code_from(&err), 5);
    }

    #[test]
    fn io_error_maps_to_6() {
        let err: anyhow::Error = AppError::Io("permission denied".into()).into();
        assert_eq!(exit_code_from(&err), 6);
    }

    #[test]
    fn unclassified_maps_to_1() {
        let err = anyhow::anyhow!("some random thing");
        assert_eq!(exit_code_from(&err), 1);
    }

    #[test]
    fn chain_cause_also_matched() {
        // 底层 reqwest 错误被 wrap 成 AppError::Network 后仍可识别。
        let root: anyhow::Error = AppError::Api("unauthorized".into()).into();
        let wrapped = root.context("while generating image");
        assert_eq!(exit_code_from(&wrapped), 4);
    }

    #[test]
    fn wrapping_preserves_classification() {
        // 模拟：底层 std::io::Error 作为 source，外层包 AppError::Network。
        let io = std::io::Error::new(std::io::ErrorKind::TimedOut, "boom");
        let err: anyhow::Error =
            anyhow::Error::new(io).context(AppError::Network("send failed".into()));
        assert_eq!(exit_code_from(&err), 3);
        let s = format!("{err:#}");
        assert!(s.contains("network error: send failed"));
        assert!(s.contains("boom"));
    }

    #[test]
    fn display_includes_prefix() {
        assert_eq!(AppError::Config("x".into()).to_string(), "config error: x");
        assert_eq!(AppError::Api("y".into()).to_string(), "api error: y");
    }
}
