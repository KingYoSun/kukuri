# Feature Data Classification: 成人向け表現の表示設定

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。仕様は ADR 0046。

### Feature Data Classification
- Feature 名: 成人向け表現の表示設定(adult content display preference)と取得ゲート
- Durable / Transient: Durable
- Canonical Source: ローカル設定ファイル(`<db_path>.content-display.json`、ユーザー端末のみ)。frontend は Rust 側の値の mirror。
- Replicated?: No(複製しない。ネットワークへ送らない)
- Rebuildable From: 再構築不可(ユーザーの設定行為そのもの)。喪失時・新規端末では既定 OFF に戻る。
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要(設定 OFF 中は成人向けラベル付き添付の blob 取得自体を行わない。ON 中の取得は ephemeral fetch で永続化しない)
- SQLite projection 必要有無: 必要(成人向けラベルの hash 逆引き `adult_media_hashes`、object projection と object-backed notification projection の `content_labels`。取得・表示ゲートの判定に使う)
- 必須 contract: Tauri command `get_content_display_settings` / `set_adult_content_display_enabled` の payload 形状。`blob_media_payload` が「成人向けラベル付き hash かつ設定 OFF」で blob 取得を行わないこと。object-backed 通知が署名済み envelope 由来の `content_labels` を保持し、未解決の既存通知を設定 OFF で fail-closed に扱うこと。
- 必須 scenario: 取得ゲート(既定 OFF → 成人向けラベル付き添付の blob 取得・プリフェッチが発生しない → ON で ephemeral 取得 → OFF へ戻すと以後の取得停止 + 表示破棄)。取得の起点にはタイムライン系に加えて「見つける」の解決済み投稿(#1052)を含み、表示中の結果に限る一時状態として扱う(永続 projection にしない)。表示ゲート(タイムライン・引用/埋め込み・返信プレビュー・Community Index の canonical 解決待ち/失敗/成功と解決済み投稿の添付メディア・in-app/OS 通知で raw text を露出しない)。frontend は `DesktopShellPage` / `CommunityIndexWorkspace` の vitest、backend は `crates/app-api` / Tauri のユニットテストで担保。

## 補足
- 表示設定は 18 歳以上の自己申告とは別の状態であり、自己申告だけでは ON にならない。既定 OFF。
- 成人向けラベルは投稿者自己申告(署名済み envelope の `content_labels`)であり、真正性は検証できない。ラベルなしコンテンツの安全は保証しない(ADR 0046)。

## 2026-09-15 改訂（#1051、ADR 0046 §6）: Community Node content advisory の合成
- ラベル源に、設定済み / 購読 Community Node が発行した `content_advisories`（ADR 0028 §8.6。`label = adult` / `sensitive`、issuer_node_id / category / confidence / signal_id / basis 付き）を第 2 の源として加える。node-local な advisory であり canonical でも署名対象でもない。`content_labels` へ書き戻さない。
- Blob: 設定 OFF 中は advisory 付き添付の blob 取得も行わない。ON 中は ephemeral fetch で永続化しない（self-label と同一ゲート）。
- SQLite projection: advisory 付き blob hash の集合を取得ゲート判定に使う。永続 projection にするか in-memory にするかは実装（#1051 child C3 / C4）で決定し、本節へ追記する。
- 追加 contract: `advisory_labeled_media_respects_adult_display_gate`（`blob_media_payload` が「advisory 付き hash かつ設定 OFF」で blob 取得を行わない）、`content_advisories_are_separate_from_signed_content_labels`、`advisory_lookup_returns_only_configured_node_signals`（一括照会は設定済み node 自身の advisory のみ返す）。
- 追加 scenario: 表示ゲート（見つけるの `content_advisories`、タイムライン向け一括照会の応答）で self-label と同じプレースホルダーになり、発行 node / category / confidence と異議申し立て導線を説明できる。設定 OFF 中に advisory 付き media の bytes 取得が 0 であることを frontend vitest と `crates/app-api` の test で担保する。
- 利用規約 第3条 4 項の文言改訂と `LEGAL_BUNDLE_VERSION` 更新（再同意）は C4 で行う。それまで advisory の合成は有効化しない。
