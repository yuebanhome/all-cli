# html-authoring

live-playground v2 问答页面的 HTML 写作硬约束：单文件、无外链、原生语义、可达、可控体积。设计 token（色值 / 间距 / 字号 / 动画 / 布点）一律走 `references/design-tokens.md`，本文件不重复其内容。

## 硬约束清单

下列每一条都是**必达**项。AI 在产出 `index.html` 前应逐条核对；本文件末尾的"Anti-pattern 自检清单"是这套约束的反向校验表。

- **单文件 `index.html`**：内联 CSS、内联 JS、内联 SVG icon。**零外链、零 CDN、零 web font**。`<link rel="stylesheet" href="…">`、`<script src="…">`、`@import url(…)` 一律禁止。
- **无 JS 框架**：只用原生 ES2020、`addEventListener`、`querySelector`、`URLSearchParams` 等 DOM API。不引入 React / Vue / Alpine / htmx / jQuery。
- **无 build step**：浏览器直接打开就能跑。不依赖打包器、不依赖 TypeScript / Sass / PostCSS。
- **可达性**：
  - 每个 `<input>` 必关联 `<label>`（卡片式选项推荐 `<label class="card"><input …>…</label>` 包裹法，无需 `for`/`id` 对齐）。
  - 每个 `<button>` 必须有可读 accessible name（按钮文字或 `aria-label`，纯图标按钮必加 `aria-label`）。
  - **状态不能仅靠颜色**：红 / 绿等成败提示必配 ✓ / ✗ 图标或文字，色弱用户可辨。
  - `aria-live="polite"` 用在 Copy 反馈区和 slider 旁的实时数字读数。
- **键盘**：
  - 所有交互元素 Tab 可达；不要 `tabindex="-1"`、`outline: none` 屏蔽焦点。
  - 全局快捷键 **Cmd/Ctrl+Enter 触发 Copy**（在 `document` 上 `addEventListener('keydown', …)`，匹配 `e.key === 'Enter' && (e.metaKey || e.ctrlKey)`）。
- **控件语义优先原生，不要叠 ARIA role**：
  - single-choice / multi-choice 的卡片**就是** `<label class="card"><input type="radio|checkbox">…</label>`——原生 `<input>` 自带语义和键盘行为（Space 切换、方向键在同 `name` 的 radio 间移动、focus ring）。**不要**再给 `<label>` 或外层 `<div>` 叠 `role="radio"` / `role="checkbox"`，会与原生语义重复或冲突。
  - 只有当必须做**完全自定义控件**（譬如 ranking 列表项、自定义颜色选择器）时，才使用 ARIA role，**并必须自实现键盘行为**（Tab 进入、Space/Enter 触发、方向键移动），不能仅依赖鼠标事件。
- **对比度**：WCAG AA（正文 4.5:1，大字 ≥ 18px 或 ≥ 14px bold 时 3:1）；dark 与 light 两种主题都必须达标。设计 token 已按此校准（见 `references/design-tokens.md` 的 *:root token 表* 节），但若新增局部色值/混合 alpha，必须重新核算。
- **空态显式渲染**：任何可能为 0 结果的列表（如 multi-choice 一项未选、annotate 一项未操作）显式渲染 "0 of N 匹配"、"未选任何项"、"暂无评论"等文案——不要留空 DOM 让用户误以为页面坏了。
- **不持久化**：不写 `localStorage` / `sessionStorage` / `IndexedDB`；不发任何外部请求（无 `fetch`、`XMLHttpRequest`、`navigator.sendBeacon`、`<img src="http…">`）。所有状态只活在当前页面 DOM 内，刷新即清零。
- **HTML 头部必含四件**（缺一即视为不合格，见下节"头部模板"）：
  - `<meta name="viewport" …>`
  - `<meta name="color-scheme" content="dark light">`
  - `<title>…</title>` —— 1–2 行能概括本轮问题的标题（**不是** slug，**不是** "Untitled"）。`playctl reindex` 从这里取标题（见 `playctl/src/index.rs:80–81`），缺失会导致 `playctl list` 退化为 slug + 空描述。
  - `<meta name="description" content="…">` —— 一句话描述本轮要问什么。同上由 `reindex` 读取。
- **体积上限**：单文件 ≤ **200 KB**（gzip 前的源文件字节数）。超出时**不要**继续塞内容、不要削减可达性或对比度——拆问题、走多轮（见 `references/multi-round-protocol.md`）。

## 头部模板

下面是最小可行 `index.html` 骨架。`{{…}}` 是必须由 AI 填入的占位符；其余结构和顺序不要动。

```html
<!doctype html>
<html lang="zh-Hans">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="color-scheme" content="dark light">
  <title>{{ROUND_TITLE}}</title>
  <meta name="description" content="{{ROUND_DESCRIPTION}}">
  <style>{{INLINE_CSS_INCLUDING_DESIGN_TOKENS}}</style>
</head>
<body>
  <!-- §3.1 共同结构布局：顶栏 / 问题区 / 控件区 / 粘性 prompt 区 -->
  <script>{{INLINE_JS}}</script>
</body>
</html>
```

占位符含义：

- **`{{ROUND_TITLE}}`**：1–2 行能概括本轮问题的标题。`playctl reindex` 读取这里写入 `index.json`，给 `playctl list` 和 home 页用。**不要**用 slug 代替（`test-pick-color` 不是标题），**不要**写 "Untitled" / "Playground"。
- **`{{ROUND_DESCRIPTION}}`**：一句话描述本轮要问什么（题干摘要即可）。同样被 `reindex` 抓走。
- **`{{INLINE_CSS_INCLUDING_DESIGN_TOKENS}}`**：内联 CSS。**必须**先声明 `references/design-tokens.md` 的 *:root token 表* 节中的 `:root { … }` token 集，以及 `@media (prefers-color-scheme: light)` 与 `@media (prefers-reduced-motion: reduce)` 覆写块；随后才是各控件的状态契约落地（见 *组件状态契约* 节：五态 + 禁用态）和本页面的具体规则。色值 / 间距 / 字号 / 动画时长 / 触控尺寸**全部走 `var(--token)`**，硬编码裸值会在自检清单第 1–2 行被打回。
- **`{{INLINE_JS}}`**：内联 ES2020。原生 DOM API，无依赖。负责：渲染上轮答案、绑定控件 → 实时刷新 sticky prompt 区、Copy 按钮 + Cmd/Ctrl+Enter 快捷键、aria-live 反馈。

## Anti-pattern 自检清单

写完 HTML 后逐行扫一遍下表；任一行命中即整改，**不要**带瑕疵交付。

| 反模式 | 触发条件 |
|---|---|
| 硬编码色值 | CSS 中出现 `#xxxxxx` / `rgb(…)` / `hsl(…)`，且**不在** `:root { … }` token 声明块内。所有色值必须走 `var(--bg-canvas)` / `var(--accent)` 等。 |
| 硬编码间距 / 时长 / 触控尺寸 | CSS 规则属性值出现裸 `px` / `ms` 数值用于间距、动画时长、触控目标尺寸，违反 `design-tokens.md` token 化原则。**允许的裸值**：`0`、百分比（`%`）、网格单位（`fr`）、视口单位（`vh` / `vw`）、`calc()` / `min()` / `max()` 表达式、`1px` 描边、SVG 内部坐标系。**特别豁免**：`@media (min-width: …px)` / `@media (max-width: …px)` 条件里的断点数值——CSS 不允许在 `@media` 条件里求值 `var()`（自定义属性只在属性值里求值），断点必须直接硬编码，并与 `--bp-single-column` 等 token 数值**人工保持同步**。 |
| 引入 CDN / 外字体 | 出现 `<link rel="stylesheet" href="http…">`、`<script src="http…">`、`@import url(http…)`、`@font-face { src: url(http…) }`。本地引用同样禁止——单文件意味着所有资源都内联。 |
| 仅靠颜色表达态 | 红 / 绿按钮或徽章只染色，无文字也无图标（缺 ✓ / ✗ / "通过" / "失败" 等可读标识）。 |
| 缺 focus ring | 某交互元素 `:focus-visible` 未声明可见样式，或 `outline: none;` 抹掉了默认焦点框却没补 `box-shadow: var(--shadow-focus);`。 |
| 控件无 label | `<input>` 没有被 `<label>` 包裹，也没有用 `for`/`id` 关联到任何 `<label>`。占位文字 `placeholder` **不是** label。 |
| 单 textarea 收所有意见 | annotate 模板里只放一个总评 textarea，让用户把对所有 section 的反馈塞到一段散文里。必须**每项**单独 ✓ / ✗ / 💬 + 行内 textarea。 |
| Prompt 区不 sticky | sticky prompt 预览缺 `position: sticky; bottom: 0;` 或被父级 `overflow: hidden` 截断，用户必须滚到页面底部才能看到 Copy 按钮。 |
| 屏蔽 prefers-reduced-motion | 动画时长不被 `@media (prefers-reduced-motion: reduce)` 缩短到 0（即 `--t-fast` / `--t-base` / `--t-slow` 三个 token 没在该 media query 内被覆写为 `0ms`）。 |
