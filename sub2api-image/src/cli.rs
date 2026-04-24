use anyhow::{bail, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "sub2api-image", version, about = "CLI for gpt-image-2 API")]
pub struct Args {
    /// 图像描述（非 --init 时必填）
    #[arg(long)]
    pub prompt: Option<String>,

    /// 输出文件路径（非 --init 时必填）
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// 传入则切换为 edit 模式
    #[arg(long)]
    pub image: Option<PathBuf>,

    /// 遮罩图（仅 edit 模式）
    #[arg(long, requires = "image")]
    pub mask: Option<PathBuf>,

    /// 图像质量：auto | high | medium | low
    #[arg(long)]
    pub quality: Option<String>,

    /// 覆盖默认 model
    #[arg(long)]
    pub model: Option<String>,

    /// 写入配置模板后退出
    #[arg(
        long,
        conflicts_with_all = ["prompt", "output", "image", "mask", "quality", "model"]
    )]
    pub init: bool,

    /// 关闭 stderr 调试日志
    #[arg(long)]
    pub quiet: bool,
}

impl Args {
    pub fn validate(&self) -> Result<()> {
        if self.init {
            return Ok(());
        }
        if self.prompt.is_none() {
            bail!("input error: --prompt is required");
        }
        if self.output.is_none() {
            bail!("input error: --output is required");
        }
        if let Some(img) = &self.image {
            if !img.exists() {
                bail!(
                    "input error: --image file does not exist: {}",
                    img.display()
                );
            }
        }
        if let Some(mask) = &self.mask {
            if !mask.exists() {
                bail!(
                    "input error: --mask file does not exist: {}",
                    mask.display()
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_from(parts: &[&str]) -> Args {
        let mut full = vec!["sub2api-image"];
        full.extend_from_slice(parts);
        Args::try_parse_from(full).unwrap()
    }

    #[test]
    fn init_alone_is_valid() {
        let a = args_from(&["--init"]);
        a.validate().unwrap();
    }

    #[test]
    fn init_with_prompt_is_rejected_by_clap() {
        let r = Args::try_parse_from(["sub2api-image", "--init", "--prompt", "x"]);
        assert!(r.is_err(), "clap should reject --init with --prompt");
    }

    #[test]
    fn missing_prompt_fails_validate() {
        let a = args_from(&["-o", "/tmp/x.png"]);
        let err = a.validate().unwrap_err();
        assert!(err.to_string().contains("--prompt is required"));
    }

    #[test]
    fn missing_output_fails_validate() {
        let a = args_from(&["--prompt", "x"]);
        let err = a.validate().unwrap_err();
        assert!(err.to_string().contains("--output is required"));
    }

    #[test]
    fn mask_without_image_rejected_by_clap() {
        let r = Args::try_parse_from([
            "sub2api-image",
            "--prompt",
            "x",
            "-o",
            "/tmp/x.png",
            "--mask",
            "m.png",
        ]);
        assert!(r.is_err(), "clap should reject --mask without --image");
    }

    #[test]
    fn image_missing_file_fails_validate() {
        let a = args_from(&[
            "--prompt",
            "x",
            "-o",
            "/tmp/x.png",
            "--image",
            "/nonexistent-path-xxx.png",
        ]);
        let err = a.validate().unwrap_err();
        assert!(err.to_string().contains("--image file does not exist"));
    }

    #[test]
    fn happy_path_generate_validates() {
        let a = args_from(&["--prompt", "hello", "-o", "/tmp/out.png"]);
        a.validate().unwrap();
    }
}
