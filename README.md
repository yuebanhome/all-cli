# all-cli

以后所有的 CLI 交互都从这里出发。

本仓库作为多个独立 CLI 的统一发布入口：每个子目录都是一个独一无二的 CLI 工具，拥有自己的源码、构建配置和文档。后续会有更多 CLI 持续加入。

## CLI 列表

| 目录 | 名称 | 简介 |
|------|------|------|
| [`sub2api-image`](./sub2api-image) | sub2api-image | gpt-image-2 兼容网关的命令行客户端，支持文生图与图像编辑 |

## 目录结构约定

- 每个 CLI 独占一个顶层目录，目录名即 CLI 名称。
- 每个 CLI 自带 `README.md`，描述安装、配置和用法。
- 每个 CLI 自管理依赖与构建（如 `Cargo.toml`、`package.json` 等），互不干扰。

## 新增 CLI

新增一个 CLI 时：

1. 在仓库根目录创建独立子目录。
2. 在该目录下放置完整的源码、构建配置和 `README.md`。
3. 在本文件的「CLI 列表」中追加一行，指向新目录。

## 安装（任意 CLI 通用）

预编译二进制发布于本仓库的 GitHub Releases。统一安装入口：

**Linux / macOS：**

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli <cli-name>
```

**Windows（PowerShell）：**

```powershell
iwr -useb https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.ps1 -OutFile $env:TEMP\install.ps1
& $env:TEMP\install.ps1 -Cli <cli-name>
```

脚本会按本机平台自动选择二进制，并强制 SHA256 校验。详细参数见 [`scripts/install.sh`](./scripts/install.sh) / [`scripts/install.ps1`](./scripts/install.ps1)。

## 发布与版本

每个 CLI 独立版本号，独立打 tag、独立发布 GitHub Release。流程见 [`docs/RELEASING.md`](./docs/RELEASING.md)。

tag 命名约定：`<cli-name>-v<semver>`，例：`sub2api-image-v0.1.0`、`sub2api-image-v0.1.0-rc.1`（带后缀的视为 pre-release，不会被标为 latest）。

## Skills

可下载的 Codex skills 放在 [`skills/`](./skills)。当前提供：

| 目录 | 用途 |
|------|------|
| [`skills/sub2api-image`](./skills/sub2api-image) | 让 Agent 安装、配置和使用 `sub2api-image` 生成或编辑图片 |

使用时复制对应目录到你的 Codex skills 目录，或在支持的客户端中按目录安装。
