# question-templates

live-playground v2 的 7 种问答模板写作规范：每种模板的适用场景、控件 HTML 骨架、回流 YAML 顶层键、键盘可达性要点。所有模板共享同一套页面骨架（顶栏 / 问题区 / 控件区 / sticky prompt 预览区），并强制使用 `references/design-tokens.md` 的 *:root token 表* 节声明的 CSS 变量。

## 共同结构（所有模板必须遵守）

每个问答页面都按下面四个分区从上到下布局；模板差异只在 *控件区* 内部实现，其余三区结构固定：

```
┌─ 顶栏 ────────────────────────────────────────────┐
│  任务名 · Round N · "上轮答案" 折叠区（如有）       │
├─ 问题区 ──────────────────────────────────────────┤
│  问题标题（h2，1–2 行）                            │
│  问题正文 / 上下文（如需要）                        │
├─ 控件区 ──────────────────────────────────────────┤
│  按模板类型                                         │
├─ Prompt 预览区（sticky，实时刷新）─────────────────┤
│  <pre>```ask-result …```</pre>  [Copy] [Reset]    │
└────────────────────────────────────────────────────┘
```

四点硬约束：

- **Prompt 预览区必须 sticky**。用 `position: sticky; bottom: 0;` 钉在视口底部，控件多 / 列表长时人也始终能看到「自己当前回答会变成什么」，并随时按 Copy。被父级 `overflow: hidden` 截断或忘了 sticky 会触发 `references/html-authoring.md` *Anti-pattern 自检清单* 节里的 "Prompt 区不 sticky" 反模式。
- **顶栏 Round 计数**：渲染 `Round N · <任务名>`，N 与 fenced header 的 `round=` 必须一致；多轮规则见 `references/multi-round-protocol.md` 的 *Round 计数* 节。
- **上轮答案折叠区**：从 Round 2 起出现，用原生 `<details>`，写法见 `references/multi-round-protocol.md` 的 *上轮答案折叠区* 节。
- **控件区**始终在 Prompt 预览区上方滚动；Prompt 预览区里的 fenced 代码块结构来自 `references/response-format.md` 的 *fenced header 语法* 节，Copy 行为见同文件的 *Copy 按钮行为* 节。

## 模板对照表

| 模板 | 何时用 | 关键控件 | YAML 顶层键 |
|---|---|---|---|
| `single-choice` | 选 1 个（3–8 选项） | 卡片式 `<label>` 包 `<input type=radio>`，整卡可点 | `choice` |
| `multi-choice` | 选 N 个 | 同上 + checkbox；显示已选数；可设上限 | `choices[]` |
| `rating` | 1–5 / 1–10 打分一个或一组项 | `<input type=range>` + 数字读数 + 标签 | `ratings{}` |
| `ranking` | 把若干项排成顺序 | 每项一对"上移 / 下移"按钮（**必做**，键盘可达）；HTML5 drag-and-drop（**可选增强**） | `order[]` |
| `form` | 多字段输入（命名 / 描述 / 数字 / 枚举混合） | 标准 label+input；分组；客户端校验 | `fields{}` |
| `annotate` | 对一批 sections / lines / files 逐项 ✓/✗/💬 | 每条 3 按钮 + 行内 textarea | `items[]` |
| `compare` | 2–4 个候选并排 vs 单独看 | 同屏并排预览 + 单选；可切到大图 | `winner`, `notes?` |

YAML 顶层键的精确字段名与示例见 `references/response-format.md` 的 *主体（按 template）* 节；本文件下面各模板节给出的 mini YAML 与该节逐字一致，**AI 不允许改名**。

---

### single-choice

**何时用**

人需要在 3–8 个互斥选项里选恰好 1 个。典型场景：挑一套配色方案、挑一种命名风格、在多个生成结果里选偏好的版本。低于 3 个选项走纯文本 yes/no（见 SKILL.md 的 *When NOT to use* 节），高于 8 个考虑拆成两轮 single-choice 或换 `compare`。

**关键控件**

原生 `<label class="card">` 包 `<input type="radio">`——卡片本体即 label，整卡可点，**不要**在 `<label>` 或外层 `<div>` 叠 ARIA 的 radio role，会与原生 `<input>` 语义冲突。

```html
<fieldset class="choice-group" style="display:grid; gap:var(--sp-3); border:0; padding:0;">
  <legend style="font-size:var(--fs-sm); color:var(--text-2);">选 1 个</legend>
  <label class="card">
    <input type="radio" name="choice" value="option_a">
    <span class="card-title">Option A</span>
    <span class="card-desc">紧凑布局，行距 1.3</span>
  </label>
  <label class="card">
    <input type="radio" name="choice" value="option_b">
    <span class="card-title">Option B</span>
    <span class="card-desc">舒展布局，行距 1.55</span>
  </label>
  <!-- 其余选项同构 -->
</fieldset>
```

`.card` 的视觉态（default / hover / focus-visible / active / selected）按 `references/design-tokens.md` 的 *组件状态契约* 节里"卡片选项"行落地，色值走 `var(--bg-surface)` / `var(--border-1)` / `var(--accent)` 等 token。

**YAML 顶层键**

顶层键 `choice`（单数），值是选中的 option 字面值；`note` 可选自由文本。

```yaml
choice: option_b
note: 因为 B 的视觉层级最清晰
```

**键盘可达性要点**

- Tab 焦点进入 fieldset 后落在**第一个**或**当前选中**的 radio 上（原生行为，无需额外 JS）。
- 方向键 ↑/↓/←/→ 在同 `name` 的 radio 间移动并同步选中；Space 切换当前焦点 radio 的选中态。
- 整卡可点是鼠标增强；键盘路径仍走原生 radio 焦点，**不要**给 `<label>` 加 `tabindex` 抢焦点。
- 通过 Tab 离开 fieldset 后再 Tab 到 sticky 预览区的 Copy 按钮；Cmd/Ctrl+Enter 全局快捷触发 Copy（约束见 `references/html-authoring.md` 的 *硬约束清单* 节）。

---

### multi-choice

**何时用**

人需要在一组选项里**选 N 个**（N 可为 0、可设上限）。典型场景：勾选要保留的 lint 规则、选要纳入下一版的功能子集、批量勾选要重命名的文件。需要并显示「已选 K / 共 M」计数；若有 `maxSelected` 上限，达到上限后未选项变 disabled 视觉态并 `aria-disabled="true"`。

**关键控件**

原生 `<label class="card">` 包 `<input type="checkbox">`，与 single-choice 同构但允许多选；**不要**给 `<label>` 或外层 `<div>` 叠 ARIA 的 checkbox role。

```html
<fieldset class="choice-group" style="display:grid; gap:var(--sp-3); border:0; padding:0;">
  <legend style="font-size:var(--fs-sm); color:var(--text-2);">
    选若干 · <span id="picked-count" aria-live="polite">0 of 5</span>
  </legend>
  <label class="card">
    <input type="checkbox" name="choices" value="option_a">
    <span class="card-title">Option A</span>
  </label>
  <label class="card">
    <input type="checkbox" name="choices" value="option_c">
    <span class="card-title">Option C</span>
  </label>
  <!-- 其余选项同构；JS 监听 change 事件实时刷新 #picked-count 与 sticky 预览区 -->
</fieldset>
```

未选时空态显式渲染"0 of N"（不要留空 DOM），对应 `references/html-authoring.md` 的 *硬约束清单* 节"空态显式渲染"。

**YAML 顶层键**

顶层键 `choices`（复数，数组）；`note` 可选。

```yaml
choices: [option_a, option_c, option_e]
note: A 和 E 必选；C 在边缘但保留
```

**键盘可达性要点**

- Tab 在每个 checkbox 之间依次穿过（与 radio 不同：checkbox 每个独立可达，方向键不切换组内焦点）。
- Space 切换当前焦点 checkbox 的勾选态。
- 计数显示区用 `aria-live="polite"`，屏幕阅读器在勾选变化时朗读"3 of 5"。
- 达到 `maxSelected` 上限时，未勾选项设 `disabled` 并 `aria-disabled="true"`，焦点跳过；已勾选项仍可 Space 反选。

---

### rating

**何时用**

对**一个或一组**项目做 1–5 或 1–10 打分。典型场景：评估三个维度（密度 / 对比度 / 动效）的偏好强度、给一段文案 1–10 打分。单维度也用 rating，不用退化成 single-choice——`<input type=range>` 的 slider 比 radio 更适合「程度」语义。

**关键控件**

每个维度一行：label + `<input type="range">` + 实时数字读数（`aria-live="polite"`，mono 字体 + tabular-nums，见 `references/design-tokens.md` 的 *排版与节奏* 节）。

```html
<div class="rating-row" style="display:grid; grid-template-columns:1fr auto; gap:var(--sp-3); align-items:center;">
  <label for="r-density" style="font-size:var(--fs-sm); color:var(--text-2);">密度</label>
  <output id="r-density-out" for="r-density"
          style="font-family:var(--font-mono); font-variant-numeric:tabular-nums; color:var(--text-1);"
          aria-live="polite">3</output>
  <input id="r-density" type="range" min="1" max="5" step="1" value="3"
         name="density" style="grid-column:1 / -1;">
</div>
<!-- 其余维度同构，min/max 视 1–5 或 1–10 调整 -->
```

range 的视觉态（track / thumb / focus）走 `references/design-tokens.md` 的 *组件状态契约* 节"range"行；track 用 `var(--border-1)`，filled 用 `var(--accent)`。

**YAML 顶层键**

顶层键 `ratings`（map），key 是维度名，value 是整数；无单独 `note` 字段（如需评论加一个 `form` 模板的 textarea 字段或留到下一轮）。

```yaml
ratings:
  density: 4
  contrast: 5
  motion: 2
```

**键盘可达性要点**

- Tab 在每个 range 之间穿过。
- ←/→ 按 `step` 减/增 1；↑/↓ 同效；Home/End 跳到 min/max；PageUp/PageDown 大步进（原生行为）。
- 数字读数 `aria-live="polite"`，每次按键朗读新值，便于盲键操作下确认。
- 单维 rating 不用键盘以外的拖拽假设——所有读数都必须可纯键盘达到。

---

### ranking

**何时用**

把若干项排成**顺序**。典型场景：给一组提案排优先级、给一组选项排展示顺序。3 项以下退化成 single-choice 或 compare；超过 ~8 项考虑拆轮，单页过长会让人疲劳。

**关键控件**

列表 + 每项一对"上移 / 下移"按钮（**必做**，键盘可达；按一次顺序变化一格）；HTML5 drag-and-drop 是**可选增强**，鼠标加速用，**不能替代键盘按钮**。列表项可用 ARIA role（`role="listitem"` 或更精细的自定义 role）配合自实现的键盘行为，是 `references/html-authoring.md` 的 *硬约束清单* 节里"控件语义优先原生，不要叠 ARIA role"规则的**允许例外**——因为 ranking 本就是完全自定义控件。

```html
<ol id="rank-list" style="list-style:none; padding:0; display:grid; gap:var(--sp-2);">
  <li data-id="item_1" draggable="true"
      style="display:grid; grid-template-columns:1fr auto auto; gap:var(--sp-2);
             align-items:center; background:var(--bg-surface);
             border:1px solid var(--border-1); border-radius:var(--r-2);
             padding:var(--sp-3);">
    <span class="rank-label">Item 1</span>
    <button type="button" class="rank-up"   aria-label="上移 Item 1"
            style="min-width:var(--touch-target-min); min-height:var(--touch-target-min);">↑</button>
    <button type="button" class="rank-down" aria-label="下移 Item 1"
            style="min-width:var(--touch-target-min); min-height:var(--touch-target-min);">↓</button>
  </li>
  <!-- 其余项同构 -->
</ol>
```

JS 监听按钮 click 与全局 keydown：按钮 Tab 可达，Enter/Space 触发；触发后交换 `<li>` 在 DOM 中的位置并刷新 sticky 预览区 `order[]`。drag-and-drop 可在 `dragstart` / `dragover` / `drop` 上额外绑定，但必须确保**不依赖 drag-and-drop**：仅靠原生按钮 + JS 也能完整完成排序。

**YAML 顶层键**

顶层键 `order`（数组），元素是 item 的字面 id，按当前顺序从上到下排列。

```yaml
order: [item_3, item_1, item_4, item_2]
```

**键盘可达性要点**

- **上移 / 下移按钮必做，键盘可达**：原生 `<button>`，Tab 可达，Enter/Space 触发；每按一次顺序变化一格。
- 拖拽是**可选增强**，不能替代键盘——AI 不允许只实现拖拽不实现按钮；纯键盘走完 ranking 路径（Tab 到列表项 → Tab/Enter 到"上移"或"下移"按钮 → 触发至少一次移动 → Tab 到 Copy → Cmd/Ctrl+Enter，剪贴板 `order[]` 顺序与操作匹配）是本模板的**必过验证项**——任何 ranking 实现若做不到这一点都不算完成。
- 每个按钮 `aria-label` 必须包含被移动项的可读名（"上移 Item 1"），屏幕阅读器朗读时能区分。
- 按钮触控目标走 `var(--touch-target-min)` (44×44)；按钮间距 ≥ `var(--sp-2)` 避免误触。
- 移动后焦点**留在刚按的按钮上**（不要重置到列表头），让用户连按多次时不打断节奏；列表顺序变化通过 sticky 预览区的 `aria-live` 读数提示。

---

### form

**何时用**

需要**多字段输入**——命名 / 描述 / 数字 / 枚举的混合。典型场景：起一个组件并填名字、变体、最大行数、备注；批量参数调一组数值。比 single/multi-choice 重，比 annotate 短；当问题"无法靠点选回答"时用 form。

**关键控件**

标准 `<label>` + `<input>` / `<select>` / `<textarea>` 组合，按字段分组用 `<fieldset>` + `<legend>`；客户端校验靠原生 `required` / `pattern` / `min` / `max` 属性 + JS 在 Copy 前做最后一次 `form.checkValidity()`。

```html
<form id="answer-form" style="display:grid; gap:var(--sp-4);">
  <fieldset style="border:1px solid var(--border-1); border-radius:var(--r-2); padding:var(--sp-4);">
    <legend style="font-size:var(--fs-sm); color:var(--text-2);">命名</legend>
    <label for="f-name" style="display:block; font-size:var(--fs-sm); color:var(--text-2);">组件名</label>
    <input id="f-name" name="name" type="text" required pattern="[A-Z][A-Za-z0-9]+"
           style="width:100%; background:var(--bg-input);
                  border:1px solid var(--border-1); border-radius:var(--r-1);
                  padding:var(--sp-2) var(--sp-3); color:var(--text-1);
                  font-size:var(--fs-md);">

    <label for="f-variant" style="display:block; margin-top:var(--sp-3);
                                   font-size:var(--fs-sm); color:var(--text-2);">变体</label>
    <select id="f-variant" name="variant"
            style="background:var(--bg-input); border:1px solid var(--border-1);
                   border-radius:var(--r-1); padding:var(--sp-2) var(--sp-3);
                   color:var(--text-1); font-size:var(--fs-md);">
      <option value="primary">primary</option>
      <option value="secondary">secondary</option>
    </select>
  </fieldset>
</form>
```

input / textarea / select 视觉态走 `references/design-tokens.md` 的 *组件状态契约* 节"input/textarea"行。

**YAML 顶层键**

顶层键 `fields`（map），key 与表单 `name` 一致。

```yaml
fields:
  name: "MyComponent"
  variant: primary
  max_lines: 3
  notes: "..."
```

**键盘可达性要点**

- Tab 顺序按 DOM 顺序穿过 fieldset 内所有可交互元素；不要 `tabindex` 重排，让顺序与视觉一致。
- 每个 `<input>` / `<select>` / `<textarea>` 必须关联 `<label>`（`for`/`id` 或包裹）；占位 `placeholder` 不替代 label。
- `required` / `pattern` 触发的浏览器原生校验气泡也通过键盘可达；Copy 前 JS 调 `form.checkValidity()`，失败时阻止 Copy 并把焦点送回第一个非法字段。
- 数字字段用 `type="number"` + `min/max/step`，方向键和滚轮调值；不要自己造数字 stepper。

---

### annotate

**何时用**

对一**批 sections / lines / files** 逐项 ✓ / ✗ / 💬。典型场景：review 一份提案的 N 个段落、对一批 diff 行决定 approve/reject/comment、批量审一组文件并记录单项备注。**每项**独立判定 + 独立 textarea，**不允许**全局只放一个总评 textarea（触发 `references/html-authoring.md` 的 *Anti-pattern 自检清单* 节"单 textarea 收所有意见"反模式）。

**关键控件**

列表，每项 3 按钮（approve / reject / comment）+ 行内 textarea；选 approve / reject 隐藏 textarea，选 comment 展开。三按钮配 ✓ / ✗ / 💬 字符 + 文字，不依赖纯色。

```html
<ul id="annotate-list" style="list-style:none; padding:0; display:grid; gap:var(--sp-3);">
  <li data-id="§1" class="annot-item"
      style="background:var(--bg-surface); border:1px solid var(--border-1);
             border-radius:var(--r-2); padding:var(--sp-3); display:grid; gap:var(--sp-2);">
    <div class="annot-head" style="display:flex; gap:var(--sp-2); align-items:center;">
      <span class="annot-id" style="font-family:var(--font-mono); color:var(--text-2);">§1</span>
      <span class="annot-title" style="color:var(--text-1);">提案标题摘要</span>
    </div>
    <div class="annot-actions" aria-label="对 §1 的判定"
         style="display:flex; gap:var(--sp-2);" data-selected="">
      <button type="button" data-verdict="approve" aria-pressed="false">✓ 通过</button>
      <button type="button" data-verdict="reject"  aria-pressed="false">✗ 否决</button>
      <button type="button" data-verdict="comment" aria-pressed="false">💬 评论</button>
    </div>
    <textarea class="annot-note" hidden rows="2" placeholder="补充说明…"
              style="background:var(--bg-input); border:1px solid var(--border-1);
                     border-radius:var(--r-1); padding:var(--sp-2);
                     color:var(--text-1); font-size:var(--fs-md);
                     min-height:calc(var(--sp-8) + var(--sp-4));"></textarea>
  </li>
  <!-- 其余项同构 -->
</ul>
```

JS 单击按钮：组内三个按钮互斥更新 `aria-pressed`（同一项内永远只有一个按钮为 `true`，其余为 `false`），并把当前 verdict 写到父级 `data-selected`；选 comment 时移除 textarea 的 `hidden`、focus 进去；选 approve/reject 时再加回 `hidden` 并清空。空态（一项未操作）在 sticky 预览区显式提示"未对 §K 给出判定"。

**YAML 顶层键**

顶层键 `items`（数组），每元素含 `id` + `verdict`，`verdict=comment` 时额外含 `note`。

```yaml
items:
  - id: §1
    verdict: approve
  - id: §2
    verdict: reject
    note: "示例代码错了"
  - id: §3
    verdict: comment
    note: "再补一段动机"
```

**键盘可达性要点**

- 每项的三按钮组使用互斥 toggle buttons：子按钮是原生 `<button>` + `aria-pressed`，**不要**给父级加 `role="radiogroup"`，也不要把 `aria-pressed` 与 radio 语义混用。Tab 进入第一个按钮后，可用方向键在组三个按钮间移动并触发选择（自实现），Enter/Space 也可触发当前焦点按钮。
- 选 comment 后焦点**自动跳到 textarea**，省一次 Tab；离开 textarea 后 Shift+Tab 回按钮组。
- 列表项之间 Tab 穿过按钮组 → textarea（若展开）→ 下一项按钮组，顺序与视觉一致。
- 三按钮文字（✓ 通过 / ✗ 否决 / 💬 评论）+ 图标双重表达态，颜色不是唯一线索。

---

### compare

**何时用**

**2–4 个候选**并排看 vs 单独看，最后选一个赢家。典型场景：两版 UI 截图选偏好的一版、三段文案对比措辞、四个布局并排比较密度。和 single-choice 的区别：compare 强调"看到差异"——必须**同屏并排预览**，候选数 ≥ 5 时降级回 single-choice，密度高了反而看不清。

**关键控件**

并排栅格（CSS grid，`grid-template-columns: repeat(N, 1fr)`，N 是候选数）+ 每列底部一个原生 radio（互斥单选，与 single-choice 同结构，不叠 ARIA role）+ 可选"切到大图"切换按钮（同屏放大单个候选）+ 可选 `notes` textarea。窄屏（视口 < 720px，对应 `--bp-single-column`）降为可滑动横向栏（见 `references/design-tokens.md` 的 *响应式 & 触摸* 节）。

```html
<fieldset class="compare-grid"
          style="display:grid; gap:var(--sp-4);
                 grid-template-columns:repeat(3, minmax(0, 1fr));
                 border:0; padding:0;">
  <legend style="font-size:var(--fs-sm); color:var(--text-2);">挑赢家</legend>

  <label class="card compare-card">
    <div class="preview" style="background:var(--bg-elevated); border-radius:var(--r-2);
                                 padding:var(--sp-3); min-height:calc(var(--sp-12) * 4);">
      <!-- 候选 A 的内联预览：SVG / 文字 / 缩略 DOM -->
    </div>
    <input type="radio" name="winner" value="variant_a">
    <span class="card-title">Variant A</span>
  </label>

  <label class="card compare-card">
    <div class="preview" style="background:var(--bg-elevated); border-radius:var(--r-2);
                                 padding:var(--sp-3); min-height:calc(var(--sp-12) * 4);">
      <!-- 候选 B -->
    </div>
    <input type="radio" name="winner" value="variant_b">
    <span class="card-title">Variant B</span>
  </label>

  <!-- Variant C 同构 -->
</fieldset>

<label for="cmp-notes" style="display:block; margin-top:var(--sp-4);
                               font-size:var(--fs-sm); color:var(--text-2);">备注（可选）</label>
<textarea id="cmp-notes" name="notes" rows="2"
          style="width:100%; background:var(--bg-input);
                 border:1px solid var(--border-1); border-radius:var(--r-1);
                 padding:var(--sp-2); color:var(--text-1); font-size:var(--fs-md);"></textarea>
```

预览区色值走 `var(--bg-elevated)` 与卡片本体的 `var(--bg-surface)` 形成层级；选中态走"卡片选项"的 `accent border + inner ring`（见 `references/design-tokens.md` 的 *组件状态契约* 节）。

**YAML 顶层键**

顶层键 `winner`（单数，单选值），可选附 `notes` 自由文本。

```yaml
winner: variant_b
notes: "B 更紧凑；A 的字号偏大"
```

**键盘可达性要点**

- 原生 radio 行为：Tab 进入 fieldset 后落在第一个 / 当前选中 radio；方向键在同 `name` 内切换；Space 切换选中态。
- 整卡可点是鼠标增强；键盘焦点环必须落在 radio 上而非卡片外层（`:focus-visible` 走 `var(--shadow-focus)` 体现在卡片视觉态）。
- "切到大图"切换按钮可选；若实现，按钮 Tab 可达、Enter/Space 触发，切回并排时焦点回到上一次选中的 radio。
- `notes` textarea 通过 Tab 在 radio 组之后到达；Cmd/Ctrl+Enter 在 textarea 内仍触发全局 Copy（在 `keydown` 监听里不要因为 target 是 textarea 就吞掉事件）。
