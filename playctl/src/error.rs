use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExitError {
    #[error("user error: {0}")]
    User(String),
    #[error("env error: {0}")]
    Env(String),
    #[error("port exhaustion: tried {tried} ports starting at {start}")]
    PortExhausted { start: u16, tried: u16 },
    #[error("server failed to start within {timeout_ms}ms")]
    ServerNotUp { timeout_ms: u64 },
    #[error("internal error: {0}")]
    Internal(String),
}

impl ExitError {
    pub fn exit_code(&self) -> u8 {
        match self {
            ExitError::User(_) => 1,
            ExitError::Env(_) => 2,
            ExitError::PortExhausted { .. } => 3,
            ExitError::ServerNotUp { .. } => 4,
            ExitError::Internal(_) => 5,
        }
    }
}

/// 把 anyhow::Error 链上的 Display 串接起来，按 ExitError 各变体的消息前缀分流到 exit code。
/// 这种字符串方法比 downcast 更鲁棒：`with_context(|| ExitError::X(...))` 把类型擦除成
/// `ContextError`，downcast 就拿不到原 ExitError；而 ContextError 的 Display 仍然是 X 的 Display，
/// 所以前缀匹配照样命中。
pub fn classify(err: &anyhow::Error) -> u8 {
    let mut buf = err.to_string();
    for cause in err.chain().skip(1) {
        buf.push_str(" | ");
        buf.push_str(&cause.to_string());
    }
    if buf.contains("user error:") {
        1
    } else if buf.contains("env error:") {
        2
    } else if buf.contains("port exhaustion:") {
        3
    } else if buf.contains("server failed to start within") {
        4
    } else {
        // Internal 与未知错误都映射到 5；放在最后兜底。
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn user_error_is_1() {
        let err: anyhow::Error = ExitError::User("bad slug".into()).into();
        assert_eq!(classify(&err), 1);
    }

    #[test]
    fn env_error_is_2() {
        let err: anyhow::Error = ExitError::Env("no project root".into()).into();
        assert_eq!(classify(&err), 2);
    }

    #[test]
    fn port_exhausted_is_3() {
        let err: anyhow::Error = ExitError::PortExhausted { start: 4747, tried: 16 }.into();
        assert_eq!(classify(&err), 3);
    }

    #[test]
    fn server_not_up_is_4() {
        let err: anyhow::Error = ExitError::ServerNotUp { timeout_ms: 5000 }.into();
        assert_eq!(classify(&err), 4);
    }

    #[test]
    fn internal_is_5() {
        let err: anyhow::Error = ExitError::Internal("panic".into()).into();
        assert_eq!(classify(&err), 5);
    }

    #[test]
    fn wrapped_chain_still_matches() {
        let inner: anyhow::Error = ExitError::User("x".into()).into();
        let wrapped = inner.context("during something");
        assert_eq!(classify(&wrapped), 1);
    }

    #[test]
    fn unknown_falls_to_5() {
        let err = anyhow!("nothing typed");
        assert_eq!(classify(&err), 5);
    }

    #[test]
    fn with_context_wrapped_typed_error_classifies_correctly() {
        // 这是 project.rs / index.rs / server.rs 等下游模块用的实际模式。
        // 之前的 downcast 实现会把这个错误归到 5（Internal），现在应当归到 2 (Env)。
        use anyhow::Context;

        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let result: Result<(), std::io::Error> = Err(io_err);
        let wrapped: anyhow::Result<()> = result
            .with_context(|| ExitError::Env(format!("write {}", "/tmp/x")));
        let err = wrapped.unwrap_err();
        assert_eq!(classify(&err), 2);
    }

    #[test]
    fn with_context_user_error_classifies_correctly() {
        use anyhow::Context;
        let io_err = std::io::Error::other("x");
        let r: anyhow::Result<()> = Err::<(), _>(io_err)
            .with_context(|| ExitError::User("bad slug".into()));
        assert_eq!(classify(&r.unwrap_err()), 1);
    }

    #[test]
    fn with_context_internal_error_classifies_correctly() {
        use anyhow::Context;
        let io_err = std::io::Error::other("x");
        let r: anyhow::Result<()> = Err::<(), _>(io_err)
            .with_context(|| ExitError::Internal("boom".into()));
        assert_eq!(classify(&r.unwrap_err()), 5);
    }
}
