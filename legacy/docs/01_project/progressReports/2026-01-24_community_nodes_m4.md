# Community Nodes M4 完了
日付: 2026年01月24日

## 概要
- moderation v1 で report/label を処理し、ルール運用と監査を通す最小ラインを実装。
- LLM v2 へ差し替え可能な抽象を追加し、disabled を既定にした。

## 対応内容
- cn_moderation schema（rules/jobs/labels）と migration を追加し、default config を seed に反映。
- cn-moderation の outbox 追従/ルール評価/label 発行と LLMProvider 抽象（disabled）を実装。
- User API に report 受理（39005）と label 参照（39006）を追加し、署名検証を実装。
- Admin API/Console にルール CRUD/監査ログ/通報・ラベル一覧/手動ラベルを追加。
- admin-console の Moderation 画面と docker-compose/.env の moderation サービス設定を追加。
- moderation の unit test（rule 判定/label イベント構築）を追加。

## 検証
- `docker run --rm -v "<workspace>:/workspace" -w /workspace/kukuri-community-node rust:1.88-bookworm cargo test --workspace --all-features`（警告: `dead_code` など）
- `./scripts/test-docker.ps1 rust`（警告: `dead_code` など）
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`、`useRouter must be used inside a <RouterProvider>`）

## 補足
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` は初回タイムアウトのため再実行し成功。
