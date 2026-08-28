# Issue #802: テスターフィードバックレポートの収集・集約

## 実装した境界

- `tester_feedback` を opt-in capability として追加した(`features: tester_feedback: true`。manifest の `capability_scope.available_enabled` に載る)。無効な node への送信は 404 `TESTER_FEEDBACK_NOT_CONFIGURED` で fail-closed。
- `POST /v1/tester-feedback` を bearer 認証 + 必須同意承認済み user のみ受け付ける endpoint として追加した。送信者の identity(pubkey)はレポート record に保存しない。
- 入力は「やろうとしたこと」「何が起きたか」「何が変だと思ったか」の 3 つの自由記述のみ。各 2000 字は両側とも Unicode コードポイント数で判定する(ADR 0039)。
- client version(`CARGO_PKG_VERSION`)と OS(`std::env::consts::OS`)は desktop-runtime が wire request 組み立て時に自動付与し、UI に入力させない。
- レポートは `cn_admin.tester_feedback` に plain TEXT で保存し、`retention.tester_feedback_days`(既定 180 日)の `expires_at` を持たせて既存の retention 再適用・期限切れ削除に組み込んだ。
- IAP admin dashboard に read-only の一覧ページ(`/tester-feedback`。3 項目全文を escape 済みで表示)、`cn-cli tester-feedback list/show` を追加した。
- desktop は Control Center トリガー隣の「フィードバックを送る」ボタンからモーダルを開き、`eligibleCommunityNodes(..., ['tester_feedback'])` で適格 node のみを送信先セレクトに出す。成功・失敗は Notice で表示する。

## 対応しない範囲

カテゴリ分類、重要度・優先度、対応ステータス管理、添付ファイル、クライアントログ自動添付、GitHub Issue 連携、AI 分析、通知、高度な検索・集計は実装していない(Issue #802 のスコープ外)。新規 harness シナリオは追加せず、各境界の contract test で固定した(ADR 0039)。

## 検証

- `cargo test -p kukuri-cn-protocol`(wire 契約 4件)
- `cargo xtask check` / `cargo xtask test`
- cn lane: cn packages の `cargo test`(実 Postgres 16 / Redis。storage 2件、HTTP contract 3件、cn-cli help 更新、cn-operator golden 更新を含む)+ cn packages clippy
- desktop-runtime: mock node に対する client contract 3件(version/OS 自動付与、入力検証、401 再認証)
- `cargo xtask desktop-ui-check`
- `cargo xtask ipc-types --check` / `cargo xtask oversized-files`
