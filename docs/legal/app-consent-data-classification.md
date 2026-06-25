# Feature Data Classification: App-level legal consent

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。

### Feature Data Classification
- Feature 名: App-level 利用規約 / プライバシーポリシー 同意（app legal consent gate）
- Durable / Transient: Durable
- Canonical Source: ローカル consent ファイル（`<db_path>.app-consent.json`、ユーザー端末のみ）
- Replicated?: No（複製しない。ネットワークへ送らない）
- Rebuildable From: 再構築不可（ユーザーの同意行為そのもの）。喪失時は再同意で再生成。
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要（DB 接続前の起動 gate で読むため、DB とは独立した JSON ファイルに保存）
- 必須 contract: Tauri command `get_app_consent_status` / `accept_app_consents` の payload 形状（startup status の `consent_required` variant を含む）
- 必須 scenario: 起動 gate（未同意 → runtime 非構築 = network 非開始 → 同意 → ready）。frontend は `App.test.tsx`、backend は `src-tauri` のユニットテストで担保。

## 補足
- 同意は単一 legal bundle（TOS + PP）を `legal_bundle_version`（単調増加の整数、初期値 1）で管理し、一括同意する。
- `accepted_bundle_version < current_bundle_version` の場合に再同意を要求する。
- 同意するまで `DesktopRuntime` を構築せず、iroh endpoint の bind / discovery を開始しない（fail-closed = IP 取得前に同意）。
