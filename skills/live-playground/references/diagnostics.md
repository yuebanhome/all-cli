# diagnostics

live-playground v2 在调用 `playctl` 过程中可能遇到的故障分类、退出码语义、`playctl` 未安装时的处置流程，以及 v2 主路径下 `stderr` warning 出现时的兜底动作。

## 退出码对照表

下表覆盖 `playctl` 当前定义的全部退出码。AI 在每次调用 `playctl --root <project_root> ...` 之后都应当先看 exit code，再决定下一步动作；不要默默忽略非 0 退出。

| code | 含义 | AI 应对 |
|---|---|---|
| 0 | 成功 | 继续 |
| 1 | 用户错误（参数 / 非法 slug / template 未知） | 修正 AI 自己传给 playctl 的参数；常见原因：slug 不符合 `[a-z0-9][a-z0-9-]{0,63}`、未知 `--template` 名（v2 主路径不用 `playctl new`，避免触发此错） |
| 2 | env error：`.runtime/` 不可写、`cwd()` 系统调用失败、写 `.gitignore` 失败等真正的环境问题 | 把 playctl stderr 转给用户，让用户检查文件系统权限 |
| 3 | 端口耗尽（4747–4762 这 16 个连续端口都被占） | 让用户 `playctl --root <other> stop` 释放另一项目占用的端口，或加 `--port <higher>` 试更高端口 |
| 4 | 服务在 5 秒 healthz 超时内未就绪 | 给用户看 `<project_root>/.playgrounds/.runtime/server.log` 找原因 |
| 5 | 内部错误（playctl 自身 bug 或 panic） | 把 playctl stderr 转给用户，建议提 issue |

## ⚠ exit 2 文案纠正

这一节是**给后续维护本文档的人**的明示提醒，避免 v1 的错误文案被回归带回来。

- v1 `skills/live-playground/SKILL.md` 的 Diagnostics 节把 exit 2 描述为「用户不在项目目录里且没传 `--root`」，这是**旧语义**，当前 `playctl` 已经不这样行为了。
- 当前 `playctl`（`>= 0.1.0`）在找不到 root marker（`.git` / `Cargo.toml` / `package.json` / `pnpm-workspace.yaml` / `pyproject.toml` / `go.mod`）时，会 **fallback 到 cwd** 并向 **stderr** 打印 `warn: no project marker found; using <cwd>`；见 `playctl/src/project.rs:17–28`（`detect_project_root` 的 fallback 分支）和 `playctl/src/main.rs:62–72`（`resolve_root` 中的 warn 打印）。这是 warning，**不会** exit 2。
- 因此 exit 2 的**正确**语义就是上文对照表里写的：`.runtime/` 不可写、`cwd()` 系统调用失败、写 `.gitignore` 失败等真正的文件系统 / 环境层面错误。
- 维护本文档时**不要**照抄 v1 的旧文案；本节本身就是写给后续维护者的护栏，避免回归。

## playctl 未装

AI 在进入工作流前先跑一次 `command -v playctl`。命令不存在时：

- **停下来**，把下面的安装命令直接呈现给用户。
- **不要**默默 `cargo install --git ...`——用户未必装了 Rust toolchain；静默地拉源码编译会浪费时间并掩盖真实意图。

Linux / macOS 主路径：

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli playctl
```

Windows 不再单独维护原生安装路径，请用 WSL2 后回到上面 Linux 主路径，与 v1 `SKILL.md` *Install* 节的口径保持一致。

从本地 checkout 安装（备选，开发或离线场景）：

```bash
cd playctl && cargo install --path .
```

兼容声明：本文档以及整个 live-playground v2 skill 假定 `playctl >= 0.1.0`。

## stderr warning 处理

当 AI 调用 `playctl --root <project_root> start --json` 时，`stderr` 上**可能**出现：

```
warn: no project marker found; using <cwd>
```

出现条件：AI 调用 `playctl` 时**没有**显式传 `--root`，并且当前 `cwd` 又不在任何含 root marker 的目录树里。**这条 warning 不是失败**——`stdout` 仍是合法 JSON，命令的退出码仍是 0。

但在 v2 主路径下这条 warning **理论上不会出现**，原因如下：

- `SKILL.md` 的 *Workflow* 节里 AI 已经先做了显式的 `<project_root>` 锚定（用 `git rev-parse --show-toplevel`，或向上找 `.git` / `Cargo.toml` 等 marker）。
- 锚定到 `<project_root>` 之后，AI 总是带 `--root <project_root>` 调用 `playctl`；`--root` 走的是 `resolve_root` 的早返回分支，不进入 `detect_project_root` 的 fallback，因此**根本不会**触发这条 warn 打印。

如果这条 warning 真的出现了：

- 说明 AI 的前置锚定逻辑出 bug 了（要么没跑 `git rev-parse`，要么没把结果带到 `--root` 参数里）。
- **中止当前流程**，回到 *Workflow* 重新确定 `<project_root>`，然后重试整次 `playctl` 调用。
- 这是冗余兜底，不是主路径；本节存在仅是为了在 AI 行为偏离时给一个明确的故障标记。
