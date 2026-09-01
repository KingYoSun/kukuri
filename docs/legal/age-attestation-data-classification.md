# Feature Data Classification: 18歳以上の自己申告

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。仕様は ADR 0046。

### Feature Data Classification
- Feature 名: 18歳以上の自己申告(age attestation gate)
- Durable / Transient: Durable
- Canonical Source: ローカル consent ファイル(`<db_path>.app-consent.json` 内の `age_attestation` レコード、ユーザー端末のみ)
- Replicated?: No(複製しない。ネットワークへ送らない)
- Rebuildable From: 再構築不可(ユーザーの申告行為そのもの)。喪失時・新規端末では再申告で再生成。
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要(DB 接続前の起動 gate で読むため、DB とは独立した JSON ファイルに保存)
- 必須 contract: Tauri command `get_app_consent_status` / `accept_app_consents` の payload 形状(`ageAttestation` の状態と `ageAttested` フラグ、startup status の `consent_required` variant を含む)
- 必須 scenario: 起動 gate(未申告 → runtime 非構築 = network 非開始 → 文書同意 + 自己申告 → ready)。frontend は `App.test.tsx`、backend は `src-tauri` のユニットテストで担保。

## 補足
- 自己申告は文書同意(terms / privacy)とは別レコードとして記録し、文書の版更新では失効しない(申告文言の重要変更時のみ `AGE_ATTESTATION_VERSION` を上げて再申告を求める)。
- 生年月日・公的身分証等は収集しない。自己申告は公的な年齢確認ではない。
- 申告が完了するまで `DesktopRuntime` を構築せず、iroh endpoint の bind / discovery を開始しない(fail-closed)。
