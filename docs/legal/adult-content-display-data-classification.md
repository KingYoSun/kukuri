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
- SQLite projection 必要有無: 必要(成人向けラベルの hash 逆引き `adult_media_hashes` と object projection の `content_labels`。取得ゲートの判定に使う)
- 必須 contract: Tauri command `get_content_display_settings` / `set_adult_content_display_enabled` の payload 形状。`blob_media_payload` が「成人向けラベル付き hash かつ設定 OFF」で blob 取得を行わないこと。
- 必須 scenario: 取得ゲート(既定 OFF → 成人向けラベル付き添付の blob 取得・プリフェッチが発生しない → ON で ephemeral 取得 → OFF へ戻すと以後の取得停止 + 表示破棄)。frontend は `DesktopShellPage` の vitest、backend は `crates/app-api` のユニットテストで担保。

## 補足
- 表示設定は 18 歳以上の自己申告とは別の状態であり、自己申告だけでは ON にならない。既定 OFF。
- 成人向けラベルは投稿者自己申告(署名済み envelope の `content_labels`)であり、真正性は検証できない。ラベルなしコンテンツの安全は保証しない(ADR 0046)。
