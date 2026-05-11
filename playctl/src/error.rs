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

/// 按 ExitError 各变体的 Display 前缀，从 anyhow::Error 链上分流到 exit code。
///
/// 用前缀匹配而非 downcast：`with_context(|| ExitError::X(...))` 会把类型擦成
/// `ContextError`，downcast 拿不到原 ExitError；但 ContextError 的 Display 仍然
/// 是 X 的 Display，因此 chain 里某一段一定会以 X 的固定前缀开头。
///
/// 对每段单独 `starts_with` 检查，不把整条链 join 后做 `contains`，避免用户
/// 字符串（路径名、用户输入）恰好包含 "user error:" / "env error:" 字面量时被
/// 错误分类。
pub fn classify(err: &anyhow::Error) -> u8 {
    for link in err.chain() {
        let s = link.to_string();
        if s.starts_with("user error:") {
            return 1;
        }
        if s.starts_with("env error:") {
            return 2;
        }
        if s.starts_with("port exhaustion:") {
            return 3;
        }
        if s.starts_with("server failed to start within") {
            return 4;
        }
        if s.starts_with("internal error:") {
            return 5;
        }
    }
    // 未在链上找到任一变体前缀（裸 anyhow!("...") 等）→ 视为 Internal。
    5
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
        let err: anyhow::Error = ExitError::PortExhausted {
            start: 4747,
            tried: 16,
        }
        .into();
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
        let wrapped: anyhow::Result<()> =
            result.with_context(|| ExitError::Env(format!("write {}", "/tmp/x")));
        let err = wrapped.unwrap_err();
        assert_eq!(classify(&err), 2);
    }

    #[test]
    fn with_context_user_error_classifies_correctly() {
        use anyhow::Context;
        let io_err = std::io::Error::other("x");
        let r: anyhow::Result<()> =
            Err::<(), _>(io_err).with_context(|| ExitError::User("bad slug".into()));
        assert_eq!(classify(&r.unwrap_err()), 1);
    }

    #[test]
    fn with_context_internal_error_classifies_correctly() {
        use anyhow::Context;
        let io_err = std::io::Error::other("x");
        let r: anyhow::Result<()> =
            Err::<(), _>(io_err).with_context(|| ExitError::Internal("boom".into()));
        assert_eq!(classify(&r.unwrap_err()), 5);
    }

    #[test]
    fn user_payload_with_marker_does_not_collide() {
        // 如果 payload 字面量里含 "user error:" / "env error:" 等，前缀匹配
        // (per-chain starts_with) 必须只看每段开头而不是 contains 整段；
        // 否则用户输入就能伪造分类。
        let inner: anyhow::Error = ExitError::Internal("rejected user error: forged".into()).into();
        assert_eq!(classify(&inner), 5);

        let inner2: anyhow::Error =
            ExitError::Env("read /tmp/contains env error: in name".into()).into();
        assert_eq!(classify(&inner2), 2);
    }

    #[test]
    fn context_payload_containing_marker_does_not_collide() {
        use anyhow::Context;
        let io_err = std::io::Error::other("x");
        // Context 字符串里恰好含 "user error:" 字面量，但 ExitError 自己是
        // Internal —— 应当分类为 5 而不是被字符串干扰到 1。
        let wrapped: anyhow::Result<()> = Err::<(), _>(io_err)
            .with_context(|| ExitError::Internal("payload mentions user error: foo".into()));
        assert_eq!(classify(&wrapped.unwrap_err()), 5);
    }
}
