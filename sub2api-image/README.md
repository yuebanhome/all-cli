# sub2api-image

gpt-image-2 兼容网关的命令行客户端。支持文生图（`/v1/images/generations`）和图像编辑（`/v1/images/edits`）。

## 安装

**一键安装（Linux / macOS）：**

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli sub2api-image
```

**Windows（PowerShell）：**

```powershell
iwr -useb https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.ps1 -OutFile $env:TEMP\install.ps1
& $env:TEMP\install.ps1 -Cli sub2api-image
```

**从源码构建：**

```bash
cd sub2api-image
cargo install --path .
```

> macOS 用户首次运行如果触发 Gatekeeper：执行 `xattr -d com.apple.quarantine ~/.local/bin/sub2api-image` 即可放行。

## 配置

```bash
sub2api-image --init
# 编辑 ~/.sub2api-image/config.toml，填入 base_url 和 api_key
```

配置示例：
```toml
base_url = "https://your-gateway.example.com"
api_key  = "sk-xxx"

[defaults]
model   = "gpt-image-2"
size    = "1024x1024"
quality = "auto"
```

### 环境变量覆盖

- `SUB2API_IMAGE_CONFIG`：直接指定配置文件绝对路径。
- `SUB2API_IMAGE_HOME`：指定目录，CLI 在其下找 `config.toml`。

## 使用

文生图：
```bash
sub2api-image --prompt "a red apple on a white table" -o apple.png
```

图像编辑（整体重绘）：
```bash
sub2api-image --image origin.png --prompt "make it watercolor" -o edited.png
```

图像编辑（局部，带遮罩）：
```bash
sub2api-image --image origin.png --mask mask.png \
  --prompt "replace sky with starry night" -o edited.png
```

### Flag 列表

| Flag | 说明 |
|------|------|
| `--prompt <TEXT>` | 图像描述（必填，除 `--init`） |
| `-o, --output <PATH>` | 输出文件路径（必填，除 `--init`） |
| `--image <PATH>` | 传入则切换为 edit 模式 |
| `--mask <PATH>` | 遮罩图（仅 edit；alpha=0 区域被重绘） |
| `--quality <Q>` | auto \| high \| medium \| low |
| `--size <SIZE>` | 图像尺寸（如 `1024x1024` / `auto`），覆盖配置默认 |
| `--model <NAME>` | 覆盖默认 model |
| `--init` | 写配置模板后退出 |
| `--quiet` | 关闭 stderr 调试日志 |

### Exit code

| code | 含义 |
|------|------|
| 0 | 成功 |
| 1 | 未分类错误 |
| 2 | 配置 / 输入错误 |
| 3 | 网络错误 |
| 4 | API 返回 4xx/5xx |
| 5 | 响应解析失败（含不支持 URL 模式） |
| 6 | IO 错误 |

## 限制（第一版）

- 固定 `n=1`，不支持一次生成多张。
- 不支持多图输入（`image[]`）。
- 响应 `url` 模式被明确拒绝（服务端需返回 `b64_json`）。
- 不自动重试；429 / 5xx 直接失败，由调用方决定是否重试。
