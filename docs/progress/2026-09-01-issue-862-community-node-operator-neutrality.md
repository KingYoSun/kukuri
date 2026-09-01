# Issue #862 Community Node operator neutrality

## Summary

Community Node の汎用実装、配備資材、desktop runtime、Release 画面から特定運営者の暗黙既定を除去した。管理 actor は operator config の明示値だけを Terraform / Compose へ渡し、未設定時の admin mutation は `503 Service Unavailable` で fail-closed する。初回 Node 候補は汎用 runtime ではなく Tauri の配布設定へ移し、利用者が削除・置換した設定は再起動後も復元しない。

Node 固有の利用規約、プライバシー、外部送信、通報、保持方針は各 Node の公開 manifest だけから Release 画面へ表示する。manifest の欠落・取得失敗時に特定 Node へ fallback せず、user-added HTTPS Node は CSP で利用できる。汎用 source への `ops@kukuri.app`、`api.kukuri.app`、`iroh-relay.kukuri.app` の再混入を検出する `operator-neutrality-check` を通常の `cargo xtask check` に組み込んだ。

稼働中 default Node は、ローカル `operator-config.yaml` / `terraform.tfvars` と repository variable `CN_LOW_COST_TFVARS_B64` に管理 actor を明示し、汎用既定を空にしても既存の write 運用を維持するよう移行した。秘密値は変更・記録していない。

## 実装内容

- operator actor: `DeployConfig.admin_actor` を追加し、未指定は空として tfvars を生成する。Terraform env/module と Compose fallback も空にし、example は `ops@example.com` を明示設定例として扱う。admin preview/apply は actor 未設定時に write を拒否し、明示 actor だけを append-only audit に渡す。
- 配布設定: `apps/desktop/src-tauri/distribution/community-nodes.json` に default onboarding Node を隔離した。`DesktopRuntime` は配布側から任意設定を受け取り、保存済み `community-node.json` が存在する場合は空一覧を含めて常に優先する。
- manifest / Release UI: manifest に `data_retention_url` を追加し、client の slim 型では全開示 URL を後方互換な任意 field とした。Release 画面は設定済み Node ごとに取得済み manifest の URL だけを表示し、固定 URL fallback を削除した。`node_role` は由来表示以外の権限・機能判定に使わない。
- CSP / 再混入防止: Tauri `connect-src` を任意 HTTPS operator 対応にし、運営者固有 domain 列挙を除去した。`cargo xtask operator-neutrality-check` は汎用 infra、Compose、VPS script、runtime、製品 UI、self-host runbook を検査し、配布設定、test/fixture、default Node 専用 runbook を完全一致または用途別 allowlist とする。
- runbook / examples: self-host と GCP の資料・補助 script を `example.com` 系へ変更し、container image 配布元と Node 運営主体を区別した。region、machine type、deployment profile は変更可能な開始値であり、管理 actor 未設定時は read-only になることを明記した。
- contract: example operator の domain/contact/actor から tfvars、manifest、開示文書が一貫して生成され、特定運営者値を含まない横断 test を追加した。初回配布設定、削除、置換、manifest round-trip、固定 URL fallback 不在、actor fail-closed も契約化した。

## 検証

- 修正前 failing contract: `cargo test -p kukuri-cn-operator --test deploy generate_tfvars_` は `admin_actor` 不在を検出して失敗し、実装後は deploy test 31件が成功。
- `cargo xtask check` / `cargo xtask test`（Rust 708件、直列 harness 22件、doc test、frontend 137 files / 1065件、すべて完走）
- `cargo xtask cn-check` / `cargo xtask cn-test`（PostgreSQL を使う Community Node integration を含め完走）
- `cargo xtask desktop-ui-check`（lint、typecheck、Vitest 1065件、Storybook build、browser E2E 58件、visual 14件）
- `cargo xtask tauri-check` / `cargo xtask e2e-smoke`
- `cargo xtask operator-neutrality-check`
- `docker compose --env-file .env.community-node.example -f docker-compose.community-node.yml config --quiet`
- `terraform fmt -check -recursive infra/terraform` と `ha` / `low-cost` / `managed-db` 各 root の `terraform init -backend=false` + `terraform validate`
- `bash scripts/vps/setup-community-node-edge.sh --help`

関連: #862、#860、`docs/architecture/default-community-node-dependency-reduction.md`
