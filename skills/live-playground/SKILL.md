---
name: live-playground
description: Use when the user faces a decision among 3+ options, choices that need to be seen / compared / clicked, multi-dimensional tuning, or per-item review. Builds a local interactive HTML page; user answers in browser; AI receives a structured fenced ask-result block.
---

# live-playground

## Overview

`live-playground` v2 把"这一轮对话里 AI 想让人决策的问题"渲染成本地交互式 HTML：人在浏览器里点选 / 打分 / 批注 / 排序 / 比较，复制一段结构化 prompt 粘回聊天，AI 据此继续工作。它取代纯文本罗列，专门服务"需要看见才能决定"的场景，不替代普通文本作答。

## Install

Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/yuebanhome/all-cli/main/scripts/install.sh \
  | bash -s -- --cli playctl
```

从本地 checkout 安装：`cd playctl && cargo install --path .`。

兼容性：本 skill 假设 `playctl >= 0.1.0`。

## When to use

满足以下任一条即起 HTML：

- 需要人在 **3 个或以上选项**间选择；
- 选项**需要看见才能决定**（视觉差异 / 空间 / 颜色 / 距离 / 并排比较）；
- 决策依赖**多个相互影响的维度同时调节**（譬如同时定调间距和字号）；
- 需要**逐项审阅**一批条目（diff 行 / 文件列表 / 提案 sections）并对每项给反馈。

## When NOT to use

- 纯 yes/no、2 选一、纯文本问题；
- 用户已经给出明确答案，AI 只是需要确认；
- 用户明确说"直接说就行"或"不要给我开页面"。

## Workflow

AI 角度的 7 步回路：

1. **判断触发**：按上面的 *When to use* / *When NOT to use* 决定走 HTML 还是文本作答。
2. **选模板**：从 `references/question-templates.md` 的 *模板对照表* 节里挑 1 个
   （`single-choice` / `multi-choice` / `rating` / `ranking` / `form` / `annotate` / `compare`）。
3. **选 slug**：首轮起一个与任务匹配的 slug，追问复用同一 slug。
   **必须匹配** `^[a-z0-9][a-z0-9-]{0,63}$`。
   非法字符（大写、`_`、`.`、`/`、首字符是 `-`、长度 > 64）会被 HTTP 层 404 拒绝。
   写盘前 AI 必须自检；不通过则转为 kebab-lowercase 后重试。
   反例：`LiveQuestion` / `foo_bar` / `-foo` / `a.b` 一律不合法。
4. **起页面**（v1 fallback 升级为 v2 主路径，**不走 `playctl new`**——它只接受内置 6 个探索型模板名）：

   **先确定 `<project_root>` 作为后续所有操作的锚点**：
   - 在 git 仓库内：`git rev-parse --show-toplevel`；
   - 否则向上找 marker：`Cargo.toml` / `package.json` / `pnpm-workspace.yaml` / `pyproject.toml` / `go.mod`，取第一个匹配目录；
   - 都没有：以 cwd 作为 `<project_root>`，并向用户明示"当前以 `<cwd>` 作为项目根"。

   **`<project_root>` 一经确定，后续所有 `playctl` 调用必须带 `--root <project_root>`，所有文件写入必须用绝对路径 `<project_root>/.playgrounds/...`**。原因：`playctl` 各子命令独立向上探测 root，如果 AI 的 cwd 在子目录而文件用相对路径写到 cwd 的 `.playgrounds/`，`playctl reindex` 仍按真正的 root 扫描，文件不会被注册，URL 直接 404（详细 rationale 见 `references/diagnostics.md` 的 *stderr warning 处理* 节）。

   首轮：

   a. **`playctl --root <project_root> start --json`**（幂等：未起则起，已起则空转）。从其 **stdout** 的 JSON 取 `url` 字段作为 base URL。
      **不要假设端口**——默认 4747 可能被占用，实际端口由 playctl 在 4747–4762 中挑选，运行中再次调用 `start` 同样会输出真实 URL。
      **不要加 `--quiet`**：cwd-fallback warning 走 stderr 是有用兜底信号，stdout 仍是纯 JSON，互不干扰。
   b. `mkdir -p <project_root>/.playgrounds/<slug>/`（绝对路径）。
   c. Write 完整 `<project_root>/.playgrounds/<slug>/index.html`。
   d. `playctl --root <project_root> reindex`。
   e. 把 `<base_url 去尾 />/<slug>/` 给用户（拼接规则见下面的 *URL 拼接* 子节）。**不要**自己拼 `http://127.0.0.1:4747/...`，一律读 step a 的 stdout。

   追问：直接 Write **完整覆写** `<project_root>/.playgrounds/<slug>/index.html`（不做 patch / 不做 diff），提示用户刷新；**无需重跑 `start` 或 `reindex`**（已注册过）。若用户报告 404，回到 step a 重取 URL。
5. **写 HTML**：单文件、内联 token、按所选模板的结构写控件 + sticky prompt 区。完整规范见 `references/html-authoring.md` 和 `references/design-tokens.md`。
6. **给 URL + 操作指引**：URL 提示里把"答完点 Copy，把代码块粘回来"**说两遍**——人不点 Copy 而手敲"我选 B"是已知风险。
7. **接 prompt**：解析人粘回的 fenced `ask-result` 块（详见 `references/response-format.md`）；定位 fenced → 解析 header → YAML.parse 主体 → 按 template 校验顶层键。**任何一步失败必须主动追问"请贴 `ask-result` 代码块"，不要默默接受文本答案**。

### URL 拼接

`playctl start --json` 输出的 `url` 自带尾 `/`（形如 `http://127.0.0.1:PORT/`）。拼接 slug URL：先去尾 `/`，再拼 `/<slug>/`，最终形如 `http://127.0.0.1:PORT/<slug>/`，**不出现 `//<slug>/`**。

参考实现：

```js
const baseClean = base_url.replace(/\/+$/, "");
const url = `${baseClean}/${slug}/`;
```

### 多轮提示

- 默认**复用同一 slug**：同任务追问 → 重写 `.playgrounds/<slug>/index.html`，让用户刷新。
- **Round 计数**：每轮把 `round` 数字递增，写入 fenced header 和顶栏。
- **上轮答案折叠区**：每轮 HTML 顶部内联上一轮的 `ask-result` YAML（人类可读，默认折叠）。
- 何时新建 slug：跨任务 / 跨主题 / 需要并行进行 → 新 slug。
- 详细协议见 `references/multi-round-protocol.md`。

## References

- `references/question-templates.md` — 7 种问答模板的写作规范、共同结构、模板对照表。
- `references/response-format.md` — fenced `ask-result` 语法 + YAML 字段约束 + 解析器伪代码。
- `references/multi-round-protocol.md` — slug 复用、Round 计数、上轮折叠区。
- `references/design-tokens.md` — 色板 / 间距 / 字号 / 动画 / 布局 token + 组件状态契约。
- `references/html-authoring.md` — 单文件 / 无外链 / 头部必含四件 / 反模式自检表。
- `references/diagnostics.md` — `playctl` 退出码、cwd-fallback warning、安装诊断。

## Other uses

如果不是问答场景而是连续调参 / 批量批注式探索，仍可用 `playctl print-template <name>`（内置 6 个探索型模板：`design-playground` / `data-explorer` / `concept-map` / `document-critique` / `diff-review` / `code-map`）。不在本 skill 的主路径里详述。
