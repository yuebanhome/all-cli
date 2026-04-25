# 发布流程

每个 CLI 独立打 tag、独立发布到 GitHub Releases。安装脚本与 `pr-status` 聚合 check 是仓库级共享基础设施，不需要随单 CLI 改动。

## tag 约定

格式：`<cli-name>-v<semver>`

示例：

- `sub2api-image-v0.1.0`（正式版）
- `sub2api-image-v0.2.0-rc.1`（pre-release，自动标识为 pre-release，不会顶替 "Latest" 指针）

若 `<semver>` 自身包含 `-`（即在主版本号之后还有破折号，如 `0.2.0-rc.1`），release workflow 会把它判为 pre-release。

## 发布步骤（以 sub2api-image 为例）

1. 在 main 分支确认 `sub2api-image/Cargo.toml` 中的 `version` 与目标版本一致：

   ```bash
   grep '^version' sub2api-image/Cargo.toml
   ```

2. 打 tag 并推送：

   ```bash
   git tag sub2api-image-v0.1.0
   git push origin sub2api-image-v0.1.0
   ```

3. GitHub Actions `release-sub2api-image` 触发，依次：
   - **`verify`**：解析 tag，校验 tag 名前缀与 `Cargo.toml` 的 `[package].version` 一致；不一致 → 全流程 fail，不构建任何东西
   - **`build` (matrix×6)**：6 个 target triple 各自构建并打包成 `tar.gz` / `zip`
   - **`publish`**：汇总产物，生成 `SHA256SUMS`，调用 `softprops/action-gh-release@v2` 创建 Release 并上传

4. Release 页面应出现 6 个 archive + 一个 `SHA256SUMS`，例如：
   - `sub2api-image-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
   - `sub2api-image-v0.1.0-x86_64-unknown-linux-musl.tar.gz`
   - `sub2api-image-v0.1.0-aarch64-unknown-linux-musl.tar.gz`
   - `sub2api-image-v0.1.0-x86_64-apple-darwin.tar.gz`
   - `sub2api-image-v0.1.0-aarch64-apple-darwin.tar.gz`
   - `sub2api-image-v0.1.0-x86_64-pc-windows-msvc.zip`
   - `SHA256SUMS`

## 版本不一致 / tag 写错的恢复

`verify` job 会因 tag 与 `Cargo.toml` 版本不一致而 fail。处理：

```bash
# 删除本地与远端 tag
git tag -d sub2api-image-v0.1.0
git push origin :refs/tags/sub2api-image-v0.1.0
# 修正 Cargo.toml 版本，commit，重新打 tag
```

## 分支保护配置（main 仅需做一次）

到 GitHub 仓库 **Settings → Branches → Add branch protection rule** 配置 `main`：

1. 勾选 "Require a pull request before merging"
2. 勾选 "Require status checks to pass before merging"
3. 在 required checks 中**只**加入：`pr-status / required`
4. 勾选 "Require branches to be up to date before merging"

> ⚠️ 不要把 `ci-<cli> / fmt`、`ci-<cli> / build-smoke (...)` 这种子 check 直接加为 required。当 PR 未触及该 CLI 的目录时，path filter 会让这些 check 永远不出现，PR 会永远卡在 "Expected" 状态无法合并。
>
> `pr-status / required` 会聚合所有 per-CLI workflow 的状态：触发了的等到结果，未触发的（path-filter mismatch）视为 success。

## 新增 CLI 时

加新 CLI（如 `xtool`，假设也是 Rust）：

1. 在仓库根目录创建 `xtool/`，放源码、`Cargo.toml`、`README.md`、`tests/`。
2. 复制 `.github/workflows/ci-sub2api-image.yml` → `ci-xtool.yml`，把 `paths:` 与 `crate-path:` 中的 `sub2api-image` 改为 `xtool`。
3. 复制 `.github/workflows/release-sub2api-image.yml` → `release-xtool.yml`，把 tag pattern 与 `with` 参数改为 `xtool`。
4. 在根目录 `README.md` 的 CLI 列表里追加一行。
5. tag 用 `xtool-v0.1.0` 即可触发首次发布。
6. 安装脚本不需要改：`bash install.sh --cli xtool` 直接可用。
7. `pr-status.yml` / `.github/scripts/wait-for-ci.sh` 也不需要改：聚合器通过 `gh api` 动态发现所有 `ci-*` workflow。

非 Rust CLI（Go / Node / ...）：新增 `<lang>-ci.yml` / `<lang>-release.yml` 两个 reusable workflow（按 `rust-ci.yml` / `rust-release.yml` 的模板写），per-CLI 触发文件继续保持薄壳。安装脚本完全语言无关，不需要任何改动 —— 只要新 CLI 遵循同样的产物命名约定（`<cli>-v<ver>-<triple>.<ext>` + `SHA256SUMS`）。

## 排错

- **`release-<cli>` workflow 没触发**：检查 tag 是否以 `<cli>-v` 为前缀；分支保护设的 `release-<cli>.yml` paths 不应阻止；tag 是否成功 `git push origin <tag>`。
- **build job 在某个 target fail**：先在本地 `cargo build --release --target <triple>` 复现；常见是 musl 缺少 `musl-tools`（CI 已自动装），ARM 上链接器问题，或 macOS rustls 链接顺序。
- **publish 报 `fail_on_unmatched_files: true`**：6 个 build 中至少一个产物没上传成功。看 `build` job 哪个 matrix 失败。
- **`pr-status` timeout**：30 分钟仍未拿到所有 ci-* 结果。看 wait-for-ci.sh 输出的 `Still pending: ...` 列表，进对应 workflow 看为什么慢。
