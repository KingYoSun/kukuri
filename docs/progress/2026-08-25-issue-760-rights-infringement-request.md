# Issue #760: Community Node への権利侵害申出

## 実装した境界

- `rights_request_endpoint` を一般通報から独立した opt-in capability とした。
- manifest に専用申出画面、説明文書、初回応答の運用目標を追加した。
- 申出フォームより先に node-local な可能・不可能な措置を提示し、版付き `scope_revision` への明示同意を受付条件にした。
- server が manifest capability と node-local 対象 record を照合し、`verified_scope`、`unverified_scope`、`out_of_scope` を決めるようにした。
- 申出 PII と権利主張を一般通報から分離した `cn_legal` schema に保存し、証拠ファイル upload は持たない。
- accountless な追跡 ID + 一度だけ表示する secret を導入し、secret は hash のみ保存する。
- 状態遷移を append-only event と redacted operator audit に記録し、`actioned` と既存の送信防止を同じ transaction で確定する。
- IAP operator UI と `cn-cli rights-requests` から一覧、詳細、状態遷移、措置を操作できるようにした。
- desktop の一般通報で権利侵害を選んだ場合は一般通報へ POST せず、専用 scope-first 画面へ移動するようにした。

## 対応しない範囲

他 node・第三者端末・投稿正本の削除、Direct P2P の遮断、暗号化 relay packet の検査、既取得データの回収、自動 SMTP、証拠ファイル保管、権利侵害の自動認定は実装していない。

## 検証

- `cargo xtask check`
- `cargo xtask test`（workspace 592件、harness 18件、desktop unit 860件）
- `cargo xtask desktop-ui-check`（lint、typecheck、unit、Storybook、browser 35件、visual 14件）
- `cargo xtask cn-check`
- `cargo xtask cn-test`（実 Postgres／Valkey を含む）
- `cargo xtask ipc-types`
- `cargo xtask oversized-files`（既存 baseline 3件のみ）
