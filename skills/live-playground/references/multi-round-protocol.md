# multi-round-protocol

live-playground v2 多轮问答的协议：同一任务的追问如何复用 slug、Round 数字如何递增、上一轮答案如何在新页面顶部以折叠区呈现、以及在什么情况下应当新开 slug 而不是继续复用。

## slug 复用规则

同一任务的追问**默认复用同一 slug**，不要每轮起新页面。具体动作如下：

- AI 重写 `<project_root>/.playgrounds/<slug>/index.html`，**完整覆写**整个文件，不做局部 patch、不做 diff 合并。新一轮的 HTML 把上一轮的 HTML 整体替换掉。
- 重写后向用户说一句"请刷新页面"。用户在已打开的浏览器标签里手动刷新即可看到新一轮内容，不需要切换 URL，也不需要新开标签页。
- **无需重跑** `playctl --root <project_root> start`：HTTP 服务在首轮已经起来；端口与 URL 不变。
- **无需重跑** `playctl --root <project_root> reindex`：该 slug 已经在 index 里登记过；重写 `index.html` 不会改变注册关系。`reindex` 只在首轮（或新建 slug 时）调用一次。

如果用户报告刷新后仍然 404，那是 playctl 进程异常或被外部 kill，需要回到首轮流程重新跑 `playctl start --json` 取最新 URL（见 SKILL.md 的工作流回路），不是 slug 复用本身的问题。

slug 字面值仍需满足 `^[a-z0-9][a-z0-9-]{0,63}$`（详见 SKILL.md 顶部的 slug 校验规则）。复用一个旧 slug 时不会触发新的合法性检查，但若首轮就写错了，所有后续轮次都会一起失败，重写前自检一次没有坏处。

## 上轮答案折叠区

每轮 HTML 顶部必须**内联上一轮**的 `ask-result` YAML 主体，以人类可读的形式呈现，默认折叠。实现上用原生 `<details>` 元素——不需要 JS，不需要 ARIA role，浏览器自带键盘可达和动画语义。

AI 在重写 HTML 时，把上一轮收到的 fenced `ask-result` 块里的 YAML 主体抽出来（去掉围栏行与 header，只保留主体），原样塞进 `<details>` 内部的 `<pre><code>` 块。示意片段：

```html
<details class="prev-round" open>
  <summary>Round 1 答案</summary>
  <pre><code>choice: option_b
note: 因为 B 的视觉层级最清晰</code></pre>
</details>
```

`<summary>` 文本固定写"Round N 答案"，N 是上一轮的 round 数字（不是当前轮）。`<pre><code>` 内保留 YAML 的换行与缩进，不做 markdown 转义。

`open` 属性是否默认开启由 AI 判断：多数情况下首次重写后默认 `closed`（去掉 `open` 属性即可），让用户的视觉焦点落在新一轮的问题区，不被旧答案分散；但若**上一轮答案对本轮决策有强依赖**（譬如上轮选了某个配色，本轮要在该配色下挑字号），应保持 `open`，方便用户随时回看。再往后的轮次（Round 3+）只显示**上一轮**的折叠区，不要叠加历史所有轮次，否则页面顶部会越积越长。

## Round 计数

首轮 `round=1`。AI 在每次重写 HTML 时把 `round` 数值 **+1**（Round 1 → 2 → 3 → …）。

`round` 数字必须同时出现在两个位置：

a. **`ask-result` fenced header 行**——见 `references/response-format.md` 的 *fenced header 语法* 节，header 4 个必填字段里就有 `round=<N>`。AI 在重写 HTML 模板里的 fenced 块时，把 header 行的 `round=` 值与本轮一致地更新。

b. **页面顶栏的可见文本**——譬如顶栏写 "Round 2 · <任务名>" 或 "Round 2"。用户一眼能看到当前是第几轮，跟折叠区的 "Round 1 答案" 形成对应关系。

约束：

- **不要**复用 round 数字。每一次重写都必须递增；同一 slug 出现两次 `round=2` 会让 AI 解析时无法区分人到底贴的是哪一轮。
- **不要**跳号。Round 1 之后是 Round 2，不是 Round 3。即使中间有一轮被用户 reset 或丢弃，对 AI 而言"重写一次" 就意味着 round +1。
- 跨 slug 不共享 round 计数：新 slug 永远从 `round=1` 起。

## 何时新建 slug

判断准则（spec 末尾原话）：「**如果新问题让人觉得'之前那个还没完'，复用；如果是另一件事，新开**」。换言之，slug 是"同一件任务的轨道"，不是"同一次会话的轨道"——只要还在同一件任务里追问，就继续复用，跨任务才换。

两个具体例子：

- **复用例**：原页面在让用户选 UI 配色方案（`single-choice` 模板，slug `ui-color-scheme`）。用户选完 option_b 后，AI 想接着问"option_b 这套配色下，正文字号偏好 14/15/16 哪一档"——这仍然是"定 UI 视觉风格"的同一件任务的延续，应**复用** slug `ui-color-scheme`，Round 2 重写 HTML，顶部折叠区保留上一轮的 `choice: option_b`。
- **新开例**：用户选完配色后，AI 接下来要问的不是配色相关，而是"给一批新组件起个命名风格"——这是一件完全独立的、跨任务的新决策，与 `ui-color-scheme` 之间没有依赖关系。应**新建** slug（譬如 `component-naming-style`），从 `round=1` 起页面，按首轮流程走 `playctl start` → Write → `reindex`。

新建 slug 时仍须满足 `^[a-z0-9][a-z0-9-]{0,63}$`：小写字母或数字开头、其余可加 `-`、长度上限 64。详细的合法性检查与 fallback 规则见 SKILL.md，本节不重复展开。
