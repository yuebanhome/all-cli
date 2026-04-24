#[cfg(test)]
use anyhow::anyhow;
#[cfg(not(test))]
use anyhow;

/// 根据 anyhow::Error 链里的消息前缀映射到 exit code。
/// 约定：所有错误在构造时用特定前缀（如 "config error:" / "api error:"）。
#[allow(dead_code)]
pub fn exit_code_from(err: &anyhow::Error) -> u8 {
    let mut buf = err.to_string();
    for cause in err.chain().skip(1) {
        buf.push_str(" | ");
        buf.push_str(&cause.to_string());
    }
    if buf.contains("config error") { 2 }
    else if buf.contains("input error") { 2 }
    else if buf.contains("network error") { 3 }
    else if buf.contains("api error") { 4 }
    else if buf.contains("response parse error") { 5 }
    else if buf.contains("io error") { 6 }
    else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_maps_to_2() {
        let err = anyhow!("config error: api_key missing");
        assert_eq!(exit_code_from(&err), 2);
    }

    #[test]
    fn network_error_maps_to_3() {
        let err = anyhow!("network error: timed out");
        assert_eq!(exit_code_from(&err), 3);
    }

    #[test]
    fn api_error_maps_to_4() {
        let err = anyhow!("api error: 401 Invalid API key");
        assert_eq!(exit_code_from(&err), 4);
    }

    #[test]
    fn parse_error_maps_to_5() {
        let err = anyhow!("response parse error: data empty");
        assert_eq!(exit_code_from(&err), 5);
    }

    #[test]
    fn io_error_maps_to_6() {
        let err = anyhow!("io error: permission denied");
        assert_eq!(exit_code_from(&err), 6);
    }

    #[test]
    fn unknown_maps_to_1() {
        let err = anyhow!("some random thing");
        assert_eq!(exit_code_from(&err), 1);
    }

    #[test]
    fn chain_cause_also_matched() {
        let root = anyhow!("api error: unauthorized");
        let wrapped = root.context("while generating image");
        assert_eq!(exit_code_from(&wrapped), 4);
    }
}
