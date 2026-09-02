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
- 同意は文書 slug 単位のレコード(#857)で管理する。現状は全文書が `LEGAL_BUNDLE_VERSION`（単調増加の整数、初期値 1、現在値 4）と同じ版番号を共有している。
- 文書ごとに `accepted_version < current_version` の場合に再同意を要求する。
- version 2 は、投稿コンテンツの権利帰属、権利保有の表明、共有範囲と Community Node capability に限定した技術的利用許諾を追加する重要変更である。
- version 3 は、利用資格（18歳以上）と成人向け表現の既定非表示に関する記載を追加する重要変更である(#858、ADR 0046)。18歳以上の自己申告の分類は `docs/legal/age-attestation-data-classification.md` を参照。
- version 4 は、管理主体、実データフロー、外部送信、診断情報、P2P copy の削除限界、日本語正文と参考訳を明記する重要変更である(#854)。外部送信の突合は `docs/legal/app-data-flow-inventory.md` と `docs/legal/external-transmission-notice.md` を参照。
- 同意するまで `DesktopRuntime` を構築せず、iroh endpoint の bind / discovery を開始しない（fail-closed = IP 取得前に同意）。
