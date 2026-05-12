# design-tokens

live-playground v2 问答页面的 CSS 设计 token 规范：所有色值、间距、字号、动画时长、布局阈值（断点 / 触控尺寸）**必须**通过 `var(--token)` 引用，不在 CSS 规则里出现裸值。例外仅限 `references/html-authoring.md` 的 *Anti-pattern 自检清单* 节末尾列出的"允许的裸值"（`0`、`%`、`fr`、`vh/vw`、`calc()`、border 描边等）。

## :root token 表

```css
:root {
  color-scheme: dark light;

  /* 颜色 */
  --bg-canvas:    #0b0d10;
  --bg-surface:   #14181d;
  --bg-elevated:  #1c2128;
  --bg-input:     #0f1216;
  --border-1:     #2a313a;
  --border-2:     #3b4452;
  --text-1:       #e6e9ef;
  --text-2:       #aab1bd;
  --text-3:       #6e7681;
  --accent:       #6ea8fe;
  --accent-fg:    #0b0d10;
  --success:      #4ec9b0;
  --warn:         #f0c674;
  --danger:       #e06c75;
  --focus-ring:   #6ea8fe;

  /* 字体 */
  --font-stack:   ui-sans-serif, system-ui, -apple-system,
                  "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  --font-mono:    ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  --fs-xs: 12px; --fs-sm: 13px; --fs-md: 15px;
  --fs-lg: 17px; --fs-xl: 20px; --fs-2xl: 24px; --fs-3xl: 30px;
  --lh-tight: 1.3; --lh-body: 1.55; --lh-loose: 1.75;
  --fw-regular: 400; --fw-medium: 500; --fw-semibold: 600; --fw-bold: 700;

  /* 间距（4px 基线） */
  --sp-0: 0; --sp-1: 4px; --sp-2: 8px; --sp-3: 12px; --sp-4: 16px;
  --sp-5: 20px; --sp-6: 24px; --sp-8: 32px; --sp-10: 40px; --sp-12: 48px;

  /* 圆角 */
  --r-1: 4px; --r-2: 8px; --r-3: 12px; --r-pill: 999px;
  --ring-selected: 2px;

  /* 阴影 */
  --shadow-1: 0 1px 2px rgba(0,0,0,.4);
  --shadow-2: 0 4px 12px rgba(0,0,0,.45);
  --shadow-focus: 0 0 0 4px rgba(110,168,254,.35);

  /* 动画 */
  --t-fast: 120ms; --t-base: 180ms; --t-slow: 260ms;
  --ease-out: cubic-bezier(.22,.61,.36,1);

  /* z-index */
  --z-sticky: 10; --z-toast: 20; --z-overlay: 30;

  /* 布局阈值（也走 token，避免 Anti-pattern 自检清单误判） */
  --bp-single-column: 720px;   /* 视口 < 此值时强制单列 */
  --touch-target-min: 44px;    /* 触控目标最小边长 */
  --content-max-ch: 70ch;      /* 正文行宽上限 */
}

@media (prefers-color-scheme: light) {
  :root {
    --bg-canvas: #fafbfc; --bg-surface: #ffffff; --bg-elevated: #f3f5f8;
    --bg-input: #ffffff; --border-1: #d9dee5; --border-2: #b9c1cc;
    --text-1: #11151b; --text-2: #4a5260; --text-3: #6b7280;
    --accent: #2563eb; --accent-fg: #ffffff;
    --shadow-1: 0 1px 2px rgba(11,13,16,.08);
    --shadow-2: 0 8px 24px rgba(11,13,16,.12);
    --shadow-focus: 0 0 0 4px rgba(37,99,235,.25);
  }
}

@media (prefers-reduced-motion: reduce) {
  :root { --t-fast: 0ms; --t-base: 0ms; --t-slow: 0ms; }
}
```

## 组件状态契约

每种交互控件**必须**实现 5 个视觉态：`default / hover / focus-visible / active / selected(or checked)`，外加禁用态。

| 控件 | default | hover | focus-visible | active | selected |
|---|---|---|---|---|---|
| 卡片选项 | bg-surface, border-1 | border-2 | + shadow-focus | bg-elevated | accent border + var(--ring-selected) inner ring |
| 按钮主 | accent / accent-fg | `filter: brightness(1.05)` | + shadow-focus | `brightness(.95)` | n/a |
| 按钮次 | bg-surface, border-1 | border-2 | + shadow-focus | bg-elevated | n/a |
| range | track 用 border-1，filled 用 accent | thumb 放大 | thumb shadow-focus | — | — |
| input/textarea | bg-input, border-1 | border-2 | accent border + shadow-focus | — | — |

禁用态：`opacity: .55; cursor: not-allowed;` 并清除 hover / focus 影响。

## 排版与节奏

- body 默认 `font-size: var(--fs-md)` (15px)，`line-height: var(--lh-body)`。
- 题干标题用 `--fs-xl`，正文不超过 70ch。
- 控件标签 `--fs-sm`，但 ≥ 13px。
- 数字读数（slider 旁边）用 mono 字体 + `font-variant-numeric: tabular-nums`。

## 响应式 & 触摸

- 单列断点：`@media (max-width: 720px)` 全部纵向堆叠；`compare` 模板降为可滑动栏。

  **技术说明**：CSS `@media` 条件里**不能**用 `var()`（自定义属性只在属性值里求值），所以断点数值必须直接写硬编码。`--bp-single-column: 720px` 在 `:root` 声明的目的是作为单一来源的文档锚点，并供 JS 通过 `getComputedStyle` 读取——**该数值必须与 `:root` 的 `--bp-single-column` 同步**，@media 数值与 token 数值必须人工保持一致。

- 触控目标 `min-width: var(--touch-target-min); min-height: var(--touch-target-min);`（44×44，提升自 v1 的 40）。
- 控件之间留白 ≥ `--sp-3`。
- 正文行宽 ≤ `var(--content-max-ch)`（例如 `max-inline-size: var(--content-max-ch);`）。
