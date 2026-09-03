# Issue #860 Community Node 法務文書

参照: Issue #860（親: #853）  
実施日: 2026-09-02

冒頭の Phase A 記録は当時の実装履歴であり、現行仕様は後段の Phase B と
「Phase B 再監査修正（2026-09-03）」を正とする。特に同一表示 version の変更拒否、
旧 placeholder 同意の破棄、日本語正文限定は、再監査修正後の現行挙動ではない。

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

## Phase B

- capability のデータ分類、処理、利用条件、保持参照、外部送信、請求経路、safety action、効果範囲を型付き descriptor に集約し、生成文書と法務 snapshot が同じ事実を参照するようにした。
- `legal` を公開する構成では全保持期間の明示を必須にし、operator 固有補足と正文 version に固定された revision 付き参考訳を追加した。
- 構造化入力由来の `policy_snapshot_revision` を導入し、operator の選択なしで snapshot 変更時の厳密な再同意と accept 競合拒否を行う。
- `cn_admin.policies` を append-only 正文履歴へ発展させ、参考訳履歴を正文複合 key へ従属させた。現行 catalog のインメモリ複製は廃止し、current・history・consent が同じ DB catalog を参照する。
- desktop の既存暗号化 per-node 同意履歴へ snapshot を後方互換に追加し、参考訳・正文 fallback metadata を同意画面へ渡した。

Phase B でも生成物は法的助言・完全性保証ではない。法的評価や第三者確認を開発 Issue の完了条件にはしない。

## Phase B 再監査修正（2026-09-03）

- `language: ja` 固定を除去し、日本語／英語の正文 renderer を operator が選択できるようにした。対応 renderer がない言語は本文と言語 metadata の不一致を避けるため公開前に拒否する。
- capability descriptor に型付き purpose と削除・訂正・利用停止等の請求経路を追加し、生成規約・privacy・service description・risk guide は `purpose`／`privacy_note`／`terms_note` の自由記述ではなく descriptor を参照する。
- cloud provider／region と manifest version を canonical snapshot に含め、生成文書が参照する hosting 情報の変更を自動再同意へ反映する。
- `cn_admin.policies`、参考訳、server consent の identity を `(slug, 表示version, policy_snapshot_revision)` へ拡張した。同じ表示 version の新 snapshot も旧正文・旧同意を更新／削除せず追記し、前後関係、公開状態、公開・施行・廃止時点と厳密な snapshot 取得 API を公開する。
- `cn-cli rights-requests` と retention command は `COMMUNITY_NODE_OPERATOR_CONFIG` の明示 retention を共有し、実運用の `cn-user-api` 起動でも operator config を必須にした。
- `cn-operator init` sample を現行の report／rights-request／tester-feedback capability と明示応答目標へ同期した。
- legacy seed は required 7文書で共通の安定 snapshot identity に backfill し、移行直後も required bundle を一括同意できるようにした。

### 再監査の検証結果

- failing-first: 非日本語正文 contract は従来の `language: ja` guard で失敗し、hosting 情報の snapshot contract は cloud provider／region が canonical input から欠落して失敗することを確認した。
- `cargo test -p kukuri-cn-operator`（生成・descriptor・日本語／英語 operator matrix を含む）
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`（Rust workspace 758 tests、harness 22 tests、frontend 140 files / 1081 tests）
- `cargo xtask rust-test`（Rust workspace 758 tests、harness 22 tests、doc tests）
- `cargo xtask cn-check`
- `cargo xtask cn-test`（Postgres／Valkey、append-only snapshot 履歴、厳密同意を含む）
- `cargo xtask cn-e2e`（Postgres／Valkey／ArcadeDB full-stack）
- `cargo xtask desktop-ui-check`（Vitest 1081、Playwright browser 58、visual 14）
- `cargo xtask scenario community_node_public_connectivity`（15 steps、`connected=true`）
- `cargo xtask e2e-smoke`（6 steps）
- `cargo xtask oversized-files`

`xtask/oversized-baseline.json` は、今回触れた既存 oversized file に型付き policy renderer と
回帰契約を追加した実測値へ更新した。機能・schema 変更と無関係なファイル分割を同じ PR に
混ぜないための baseline 更新であり、上限を無効化するものではない。
