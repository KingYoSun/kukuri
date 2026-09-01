# Issue #860 Community Node 法務文書 Phase A

参照: Issue #860（親: #853）  
実施日: 2026-09-02

## 完了内容

- `operator-config.yaml` schema に `legal` を追加し、7文書の kind / slug / version / 施行日 / 言語 / required と、氏名・住所の請求方法を検証する。
- 現行 KingYoSun Node の設定値として operator `KingYoSun`、連絡先 `ops@kukuri.app`、日本語版 v1（施行日 2026-09-02）を設定できる最小例と契約テストを追加した。
- 生成文書から「ドラフト」を外し、kukuri client と当該 Node の適用範囲、Node が扱う capability 別データ、P2P 全体からの削除を保証しない境界を明記した。
- public manifest に全法務文書の metadata / URL を追加し、認証不要の `GET /v1/policies` は operator config 由来の required 文書本文だけを返すようにした。
- `cn_admin.policies` に施行日・言語を追加し、起動時に operator config と同期する。同一版の本文・metadata 差し替えと version rollback は fail-closed で拒否する。旧固定英語 placeholder は一度だけ置換し、その同意履歴を破棄する。
- desktop の per-node 同意画面へ施行日・言語を追加し、既存の slug/version 比較による更新検知・再同意を維持した。
- data classification と operator runbook を更新した。

## 境界

- Phase A は現行 KingYoSun Node の Preview 公開に必要な実態整合までを対象とする。
- 第三者 Node の汎用テンプレート、重要変更フラグ、翻訳版、専門家レビューは Issue #860 の将来対応として残す。
- app-level 規約同意と per-node 同意は別記録であり、Node 同意が無い状態では公開文書以外の Node 通信を開始しない。

## 検証

- failing-first: `phase_a_legal_documents_publish_operator_and_versioned_identity` が連絡先未出力で失敗することを確認後、実装して成功。
- `cargo test -p kukuri-cn-operator`
- `cargo test -p kukuri-desktop-runtime community_node::manifest_support::tests::server_manifest_output_round_trips_into_slim_type`
- `cargo test -p kukuri-cn-user-api --tests`
- `cargo run -p kukuri-cn-operator --bin cn-operator -- validate-config --config operator-config.yaml`
- `cargo xtask cn-test`（Postgres / Valkey 統合を含む）
- `cargo xtask desktop-lint`
- `cargo xtask desktop-test`（138 files / 1069 tests）
- `cargo xtask rust-test`（workspace 732 tests + harness 22 tests）
- `cargo xtask ipc-types`

DB を使う `policy_sync` 契約は `KUKURI_CN_RUN_INTEGRATION_TESTS=1` の `cn-test` で実行し、旧 placeholder 同意の破棄、同一版変更拒否、version rollback 拒否を検証する。
