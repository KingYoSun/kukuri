# #1073 CI runner を Namespace へ移し Rust cache を Cache Volume に置く作業記録

## 現在の状態

- リスク区分 A、Scope revision: 2026-09-17、基準 commit: `1c45b232`。
- 2026-09-17 にユーザーが次を承認した: repo を GitHub organization `kukuri-app` へ移管して外部 runner を使う、`kukuri-cn-images` の build / push を外部 runner の VM で動かす、Issue #1073 の Goal / AC を速度優先へ改訂する。Issue 操作・commit・PR 作成・CI 成功後の merge は承認待ちなしで行う。
- 当初は Blacksmith を採用したが、新規 organization の審査でブロックされたため、同日にユーザーの判断で Namespace へ切り替えた。runner profile はユーザーが作成済みで、Linux は `namespace-profile-kukuri`（Ubuntu 24.04）、Windows は `namespace-profile-kukuri-win`（Windows Server 2022）。
- AC / INVAR の正本は [Issue #1073](https://github.com/kukuri-app/kukuri/issues/1073)。旧 Scope（2026-09-16、GitHub-hosted のまま 10 GB に収める）は Issue 本文の「Superseded」節に残し、計測結果は本書「修正前の観測」に引き継いだ。
- repo 移管の副作用（GHCR namespace が `ghcr.io/kingyosun/*` から `ghcr.io/kukuri-app/*` へ変わる、`apps/desktop/src-tauri/tauri.conf.json` の updater endpoint、`crates/cn-operator` の image 既定値、`infra/terraform` の変数、本番 compose の image 参照、docs 内の URL）は別 Issue で扱い、本 Issue の Close 条件に含めない。

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

sccache の compile request 数は job の同値クラスを示す: 1156（xtask + harness のみ: desktop-ui / desktop-browser / smoke / community-node / additional-scenarios / multi-device）、1398（rust-test）、1504（cn-e2e、app-api-slow）、2738（cn: clippy `--all-targets` + test）、4011（static: clippy `--workspace` + `target/desktop-tauri-check` への tauri check）。同じクラスの job は同じ成果物を作る。

### 所要時間（main run 35104874597）

compile 時間は job ログの `Compiling` 行の間隔（120 秒未満）の合計。残りは test 実行、setup、cache 保存。

| job | 所要 | うち compile | 見込み（cache 有効時） |
| --- | --- | --- | --- |
| `windows-fast` | 48.3 分 | 33.4 分 | 約 15 分 |
| `linux-rust-tests` | 18.0 分 | 6.2 分 | 約 9 分 |
| `linux-desktop-browser` | 14.9 分 | 2.3 分 | 約 9 分 |
| `linux-rust-static` | 13.4 分 | 11.3 分 | 約 4 分 |
| `linux-cn` | 12.3 分 | 9.2 分 | 約 4 分 |
| `linux-desktop-ui` | 12.2 分 | 3.3 分 | 約 7 分 |
| `linux-cn-e2e` / `linux-community-node` / `linux-smoke` | 8.6 / 5.5 / 4.7 分 | 6.4 / 3.5 / 3.3 分 | 計 約 6 分 |

## 外部 runner の仕様で設計に効いた点（2026-09-17 取得）

### Blacksmith（不採用）

- organization 限定。cache は `actions/cache` 系を透過的に肩代わりするが、sccache と docker の `type=gha` は GitHub backend に残る。新規 organization の審査でブロックされたため不採用。

### Namespace（採用）

- **Cache Volume**: runner に NVMe の volume を mount し、upload / download が無い。profile で caching を有効にする（最小 20 GB）。`namespacelabs/nscloud-cache-action@v1` の `cache: rust` が Cargo registry、git 依存、build 成果物を volume に置く。
- **並行と commit**: job は最後に commit された版の fork を受け取る。exit 0 で終わった job の状態が次の親になり（last write wins）、失敗した job の変更は捨てられる。容量を超えると volume はリセットされ、次の job は cache miss になる。
- **分離**: volume は workspace / runner profile / repository ごとに分かれ、同じ profile・repo の全 job が 1 つを共有する。`runs-on: <profile>;overrides.cache-tag=<名前>` で別 volume にできる。profile の Branch protection で volume を更新できる branch を制限できる（読み取りは全 branch / PR で可能）。
- **Docker**: Namespace runner では docker build が既定で Remote Builder に向き、layer cache を持つ。`cache-from` / `cache-to` は不要。`docker/build-push-action` をそのまま使う。
- **Windows**: Cache Volume と `nscloud-cache-action` の Windows 対応は docs に記載が無い。

## 対応

| 条件 | 変更 |
| --- | --- |
| AC-1 | Linux job（`kukuri-fast` 8、`kukuri-nightly` 全 job、`kukuri-cn-images`、`kukuri-visual-baseline`）を `namespace-profile-kukuri`、`windows-fast` を `namespace-profile-kukuri-win` へ |
| AC-1 / AC-2 | Linux の `Swatinem/rust-cache` を `namespacelabs/nscloud-cache-action@v1`（`cache: rust`）へ。last write wins で別内容の job が互いの成果を消さないよう、build 内容が同じ job 同士だけで cache-tag を共有する: `kukuri-harness`（desktop-ui、desktop-browser、smoke、community-node、additional-scenarios、multi-device）、`kukuri-rust-tests`、`kukuri-rust-static`、`kukuri-cn`、`kukuri-cn-e2e`、`kukuri-app-api-slow`。fast と nightly の同名 job は同じ tag。cargo を使わない `kukuri-cn-images` / `kukuri-visual-baseline` は専用 tag |
| AC-1 | `windows-fast` は Cache Volume の Windows 対応が未確認のため、#1074 の `Swatinem/rust-cache`（main 限定保存、GitHub backend、2.17 GB）を維持 |
| AC-3 | fast Linux と nightly から sccache を除去（workflow env、`Setup sccache`、`Show sccache stats`、`windows-fast` の無効化 env）。GitHub backend に残るのは Windows の rust-cache と pnpm の cache だけになる |
| AC-3 | `kukuri-cn-images` の `docker/setup-buildx-action` と `cache-from` / `cache-to`（`type=gha`）を削除し、Namespace の既定 builder を使う |
| INVAR-1 | 各 job の実行 step、timeout、artifact 名・path・upload 条件は変更なし（diff で確認）。`Setup sccache` / `Show sccache stats` は cache 基盤 step として #1074 の先例どおり削除 |
| 付随 | 視覚回帰 baseline を比較側と同じ profile で生成するため `kukuri-visual-baseline.yml` も変更し、`docs/runbooks/dev.md` の注記を同期。actionlint 用に `.github/actionlint.yaml` へ profile label を登録 |

profile 側の前提（ユーザー設定）: `namespace-profile-kukuri` の caching を有効にする。Branch protection を main に限定すると、PR run は main の volume を読むだけになり、未 merge の成果物で volume が上書きされない。

検証: `actionlint .github/workflows/kukuri-fast.yml .github/workflows/kukuri-nightly.yml .github/workflows/kukuri-cn-images.yml .github/workflows/kukuri-visual-baseline.yml`、`git diff --check`。

## 試行 PR の計測（AC-1 見込み）

PR run の計測後に追記する。確認項目: 全 job が Namespace runner で成功する、`Cache Rust` が volume を mount する、cn-indexer smoke の `load: true` が Remote Builder で動く、視覚回帰が profile の image で一致する（割れた場合は同じ PR で baseline を再生成する）。

## CI 計測（AC-1〜AC-3）

merge 後に追記する。
