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

/// 把 anyhow::Error 链解到 ExitError；找不到则按 Internal 处理。
pub fn classify(err: &anyhow::Error) -> u8 {
    for cause in err.chain() {
        if let Some(e) = cause.downcast_ref::<ExitError>() {
            return e.exit_code();
        }
    }
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
}
