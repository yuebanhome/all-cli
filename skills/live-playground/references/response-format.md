# response-format

live-playground v2 问答页面回流给 AI 的唯一格式定义：`ask-result` fenced 代码块的 header 语法、按模板分类的 YAML 主体、字段约束、解析失败处理、最小解析器伪代码，以及 Copy 按钮行为。

## fenced header 语法

AI 解析的唯一来源是下面这种 fenced 代码块（**不接受**自由文本、JSON、或缺围栏的 YAML）：

````
```ask-result slug=<slug> round=<N> template=<name> ts=<ISO8601>
<YAML 主体>
```
````

header 行 4 个字段全部**必填**：

- `slug`：任务标识，与 `.playgrounds/<slug>/` 目录名一致。
- `round`：轮次序号（从 1 起的整数）。
- `template`：模板名，取值见下一节（`single-choice` / `multi-choice` / `rating` / `ranking` / `form` / `annotate` / `compare`）。
- `ts`：ISO8601 时间戳，由页面在 Copy 时写入。

主体使用 **YAML**（不是自由 markdown，不是 JSON），统一好读好写。

## 主体（按 template）

下面 7 段示例来自 spec §4.2，逐字给出；顶层键名固定，不允许 AI 改名。

```yaml
# template=single-choice
choice: option_b
note: 因为 B 的视觉层级最清晰

# template=multi-choice
choices: [option_a, option_c, option_e]
note: A 和 E 必选；C 在边缘但保留

# template=rating
ratings:
  density: 4
  contrast: 5
  motion: 2

# template=ranking
order: [item_3, item_1, item_4, item_2]

# template=form
fields:
  name: "MyComponent"
  variant: primary
  max_lines: 3
  notes: "..."

# template=annotate
items:
  - id: §1
    verdict: approve
  - id: §2
    verdict: reject
    note: "示例代码错了"
  - id: §3
    verdict: comment
    note: "再补一段动机"

# template=compare
winner: variant_b
notes: "B 更紧凑；A 的字号偏大"
```

## 字段约束

1. header 行 4 字段（`slug` / `round` / `template` / `ts`）全部必填；AI 据此识别。
2. YAML 主体顶层键由模板决定，名称固定（见上一节 *主体（按 template）* 的逐模板示例），不允许 AI 改名。
3. `note` / `notes` 永远是可选自由文本，给人留补充话语的位置。
4. 人粘回的内容可能含前后文，AI 只抓**第一个** `ask-result` fenced 块。
5. 解析顺序固定：定位 fenced → 解析 header → `YAML.parse` 主体 → 按 template 名校验顶层键。

## 解析失败处理

上述解析顺序中任一步失败，AI 的正确动作只有一个：

- **主动追问** "请贴 `ask-result` 代码块"。
- **不要**默默接受文本答案；**不要**"差不多就行"地从自然语言里推断 `choice` / `choices` / `ratings` 等结构化字段。

这条规则与 spec §8.3 的"人粘的 YAML 主体格式错 → 明确报错，不要默默接受"一致。

## 解析器伪代码

最小可行解析器（约 15 行 JS），用任意 YAML 解析器（例如 `js-yaml`）填充 `parseYaml`：

```js
function parseAskResult(text) {
  const m = text.match(/```ask-result\s+([^\n]+)\n([\s\S]*?)```/);
  if (!m) throw new Error("missing ask-result fenced block");
  const header = Object.fromEntries(
    m[1].trim().split(/\s+/).map(kv => kv.split("="))
  );
  for (const k of ["slug", "round", "template", "ts"]) {
    if (!header[k]) throw new Error(`missing header field: ${k}`);
  }
  const body = parseYaml(m[2]);  // 用任意 YAML parser
  return { header, body };
}
```

说明：

- 正则 `/```ask-result\s+([^\n]+)\n([\s\S]*?)```/` 默认非贪婪，自然只抓**第一个** `ask-result` fenced 块，匹配本文 *字段约束* 节第 4 条。
- `parseYaml` 假定由调用方注入；建议直接用社区成熟实现（譬如浏览器侧 `js-yaml`），不要自己写。
- header 必填校验失败立即抛错；调用方捕获后应触发本文 *解析失败处理* 节描述的主动追问行为。
- 按 template 名校验顶层键（例如 `single-choice` 必有 `choice`、`rating` 必有 `ratings`）放在 `parseAskResult` 之外做，保持解析器本身只负责"取出结构化数据"。

## Copy 按钮行为

页面上每个问答模板都必须提供 Copy 按钮，行为契约如下：

- 写入剪贴板的是**整段 fenced 块**（含开头的 ```` ```ask-result ... ```` 行和结尾的 ```` ``` ````围栏），不只是 YAML 主体。
- Copy 完成后 1.5 秒内必须给出可见的"copied"反馈（譬如按钮文本切换、或紧邻按钮的状态文本）。
- 反馈区域使用 `aria-live="polite"`，与 `references/html-authoring.md` 的可达性硬约束一致。
