# #1073 CI runner を Blacksmith へ移し cache を成果物 class 単位に統合する作業記録

## 現在の状態

- リスク区分 A、Scope revision: 2026-09-17、基準 commit: `1c45b232`。
- 2026-09-17 にユーザーが次を承認した: repo を GitHub organization へ移管して Blacksmith を使う、`kukuri-cn-images` の build / push を Blacksmith の VM で動かす、Issue #1073 の Goal / AC を速度優先へ改訂する。Issue 操作・commit・PR 作成・CI 成功後の merge は承認待ちなしで行う。
- AC / INVAR の正本は [Issue #1073](https://github.com/KingYoSun/kukuri/issues/1073)。旧 Scope（2026-09-16、GitHub-hosted のまま 10 GB に収める）は Issue 本文の「Superseded」節に残し、計測結果は本書「修正前の観測」に引き継いだ。
- repo 移管の副作用（GHCR namespace `ghcr.io/kingyosun/*`、`apps/desktop/src-tauri/tauri.conf.json` の updater endpoint、`crates/cn-operator` の image 既定値、`infra/terraform` の変数、本番 compose の image 参照、docs 内 124 file の URL）は別 Issue で扱い、本 Issue の Close 条件に含めない。

## 修正前の観測（2026-09-16 UTC、GitHub-hosted）

### cache の実態

| 観測 | 値 | 根拠 |
| --- | --- | --- |
| Actions cache usage API | 19.39 GB / 1735 entry → 30 分後 15.83 GB / 1548 entry | `gh api repos/KingYoSun/kukuri/actions/cache/usage` |
| `actions/caches` 一覧の合計 | 10.2 GB / 1014 entry（rust-cache 8.85 GB / 6、sccache 0.96 GB / 1001、node 0.37 GB / 3、buildkit blob 約 1.6 GB / 18） | 全 page を取得して集計 |
| job ごとの rust-cache 保存サイズ | `linux-rust-static` 2.14 GB、`linux-cn` 1.92 GB、`linux-cn-e2e` 1.63 GB、`linux-app-api-slow` 1.63 GB、`linux-rust-tests` 1.41 GB、xtask + harness だけを build する job 各 1.06 GB、`windows-fast` 2.17 GB | 各 job の `Post Cache Rust`（tar 送信量）、run 35104874597 / 35073909006 / 35022170102 / 35112821418 |
| 1 日の main scope 書き込み量 | `Kukuri Fast` Linux 8 job 11.3 GB × main push 回数 + `Kukuri Nightly` Linux 10 key 13.1 GB + Windows 2.17 GB | 上記サイズの合算 |
| main の `Cache Rust` restore | 3 run + nightly の Linux 26 job すべて「No cache found」。保存は run あたり 3〜5 job が「Unable to reserve cache … another job may be creating this cache」で失敗（同時刻に PR run が同名 key を保存） | 同 run のログ |
| sccache（Linux） | hit 率 5〜51%（中央値 約 25%）、write error 535〜3093 件 / job | `Show sccache stats` |

sccache の compile request 数は job の同値クラスを示す: 1156（xtask + harness のみ: desktop-ui / desktop-browser / smoke / community-node / additional-scenarios / multi-device）、1398（rust-test = 1156 + workspace test、cn crate 除外）、1504（cn-e2e、app-api-slow）、2738（cn: clippy `--all-targets` + test）、4011（static: clippy `--workspace` + `target/desktop-tauri-check` への tauri check）。同じクラスの job は同じ成果物を別 key に重複保存していた。

### 所要時間（main run 35104874597）

| job | 所要 | 内訳 |
| --- | --- | --- |
| `windows-fast` | 48 分 | Doctor 10 分、Windows Tauri compile check 7 分、Windows package 24 分、`Post Cache Rust` 5 分、その他 2 分 |
| `linux-rust-tests` | 18 分 | cache なし |
| `linux-desktop-browser` | 15 分 | cache なし |
| `linux-rust-static` | 13.5 分 | cache なし |

### Blacksmith の仕様で設計に効いた点（公式 docs、2026-09-17 取得）

- organization 限定（personal repository 不可）。`KingYoSun/kukuri` の owner type は `User` だったため移管が前提。
- cache は `actions/cache` 系（`Swatinem/rust-cache`、`actions/setup-node`）を透過的に Blacksmith backend へ向ける。無料枝 25 GB / repo / 週、7 日 LRU、branch scope は GitHub と同じ。job 単位 key のままでは 1 日 24 GB 以上を書くため 25 GB でも溢れる。
- 「Currently, the Rust `sccache` and the GitHub Actions cache option in the `docker/build-push-action` are still redirected to GitHub's backend.」→ sccache は除去、docker は `useblacksmith/setup-docker-builder@v2` + `useblacksmith/build-push-action@v2` へ。
- runner label は `blacksmith-{2,4,8,16,32}vcpu-ubuntu-2404` / `blacksmith-{2..32}vcpu-windows-2025`。Windows は full Visual Studio なし（VS Build Tools 2022 あり）。同時実行数の上限なし。
- GitHub App は secrets / code の権限を要求しないが、job が使う secrets は Blacksmith の VM 上に展開される。署名鍵を使う `kukuri-release` / `kukuri-linux-package` は移さない（INVAR-2）。
- sticky disk（tar なしの永続 disk、LWW、Branch Protection 可、$0.50/GB/月）は Windows 対応が docs に無いため、本 Issue では使わず後続の候補にする。

## 対応

| 条件 | 変更 |
| --- | --- |
| AC-1 | `kukuri-fast.yml` / `kukuri-nightly.yml` の runner を Blacksmith へ。compile 重 job（rust-static、rust-tests、cn、cn-e2e、app-api-slow）は `blacksmith-8vcpu-ubuntu-2404`、xtask のみの job（desktop-ui、desktop-browser、smoke、community-node、additional-scenarios、multi-device）は `blacksmith-4vcpu-ubuntu-2404`、`windows-fast` は `blacksmith-8vcpu-windows-2025` |
| AC-1 / AC-2 | Linux rust-cache の key を job 単位から class 単位へ: `kukuri-linux-dev`（保存: fast `linux-rust-tests`）、`kukuri-linux-static`（保存: fast `linux-rust-static`）、`kukuri-linux-cn`（保存: fast `linux-cn`）。保存は `save-if: main` + `cache-on-failure: true`、他 job と nightly 全 job は `save-if: false` で restore のみ。`windows-fast` は #1074 の設定（main 限定保存）を維持 |
| AC-3 | fast Linux と nightly から sccache を除去（workflow env、`Setup sccache`、`Show sccache stats`、`windows-fast` の無効化 env） |
| AC-3 | `kukuri-cn-images.yml` の `docker/setup-buildx-action@v3` を `useblacksmith/setup-docker-builder@v2`（`cache-key: docker/cn/Dockerfile-<image>`）へ、`docker/build-push-action@v6` 2 箇所を `useblacksmith/build-push-action@v2` へ置換し `cache-from` / `cache-to`（`type=gha`）を削除 |
| INVAR-1 | 各 job の実行 step、timeout、artifact 名・path・upload 条件は変更なし（diff で確認）。`Setup sccache` / `Show sccache stats` は cache 基盤 step として #1074 の先例どおり削除 |
| 付随 | 視覚回帰 baseline を比較側と同じ runner image で生成するため `kukuri-visual-baseline.yml` も `blacksmith-4vcpu-ubuntu-2404` へ。`docs/runbooks/dev.md` の baseline 注記を同期。actionlint 用に `.github/actionlint.yaml` へ Blacksmith label を登録 |

検証: `actionlint .github/workflows/kukuri-fast.yml .github/workflows/kukuri-nightly.yml .github/workflows/kukuri-cn-images.yml .github/workflows/kukuri-visual-baseline.yml`、`git diff --check`。

## 試行 PR の計測（AC-1 見込み）

PR run の計測後に追記する。

## CI 計測（AC-1〜AC-3）

merge 後に追記する。
