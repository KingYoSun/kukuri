# Feature Data Classification: アカウント鍵のエクスポート・インポートと複数アカウント

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。仕様は ADR 0047。

### Feature Data Classification: アカウント registry
- Feature 名: 複数アカウントの列挙とアクティブ選択(accounts registry)
- Durable / Transient: Durable
- Canonical Source: ローカル registry ファイル(`<app_data>/accounts.json`、ユーザー端末のみ)
- Replicated?: No(複製しない。ネットワークへ送らない)
- Rebuildable From: 部分的に再構築可(`accounts/<id>/` ディレクトリ走査で id と鍵は回復できるが、label / timestamps は失われる)
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要(runtime 構築前に読む起動時状態のため、DB とは独立した JSON ファイル)
- 必須 contract: registry に秘密情報(秘密鍵・パスフレーズ)を含まないこと。`list_accounts` / `switch_account` の payload 形状。
- 必須 scenario: flat レイアウトからの一括移行(中断・再開含む)で鍵が失われないこと(`crates/desktop-runtime/src/tests/accounts_migration.rs`)。

### Feature Data Classification: アカウント鍵エクスポート
- Feature 名: アカウント鍵の暗号化エクスポート・インポート(account key export)
- Durable / Transient: Transient(アプリは成果物を保存しない。保存はユーザーの明示操作)
- Canonical Source: エクスポート元アカウントの identity storage(keyring / `<db>.identity-key`)
- Replicated?: No(アプリがネットワークへ送ることはない。持ち出しはユーザー自身の操作のみ)
- Rebuildable From: 同一鍵の identity storage から何度でも再生成可。パスフレーズ喪失時、当該エクスポートは復元不能。
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要
- 必須 contract: envelope 形式(version / KDF パラメータ / salt / fingerprint の AAD 束縛、誤パスフレーズ・破損・未知バージョン・改竄の拒否、`crates/core/src/tests/identity_export.rs`)。平文秘密鍵が IPC / ログ / Debug 出力 / preview に現れないこと。
- 必須 scenario: export → preview(fingerprint 確認)→ import(アカウント追加)→ 切替後に同一 pubkey となる round-trip。同一 pubkey 重複インポートの拒否と既存アカウント無傷。

## 補足
- 本機能の対象は本人性を表すアカウント鍵のみ。投稿 DB・設定・添付・非公開チャネル秘密等の完全バックアップは対象外(#855)であり、UI(設定 → アカウント の注記)と本文書で範囲を明示する。
- エクスポートの漏えいはなりすましに直結し、運営者は鍵の失効も復旧もできない。エクスポート UI はこの警告への明示同意を必須とする。
- パスフレーズは argon2id で鍵導出され、平文でどこにも保存されない。request DTO の `Debug` は `<redacted>`。
