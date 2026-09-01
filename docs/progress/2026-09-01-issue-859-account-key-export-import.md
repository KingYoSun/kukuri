# Issue #859 account key export/import and multi-account

## Summary

無償 Preview 公開のブロッカー(#853 Phase A)として、アカウント鍵の暗号化エクスポート・インポートを実装した。仕様は ADR 0047(`docs/adr/0047-account-key-export-import-multi-account.md`)に固定し、ADR 0002 分類を `docs/legal/account-key-export-data-classification.md` に置いた。インポートは再起動での鍵置換ではなく**アカウントの追加**として扱い、複数アカウントの列挙・再起動なしの切替を同時に導入した(ユーザー決定)。ストレージは後方互換を持たず `<app_data>/accounts/<id>/kukuri.db` の単一レイアウトへ統一し、旧 flat レイアウトは初回起動時にフェイルセーフに一括移行する(ユーザー決定: 破壊的変更可)。

意図的にやらなかったこと: 完全な端末移行/バックアップ(#855、UI と docs で範囲差を明示)、鍵の失効・ローテーション(ADR 0047 §5 で制約を文書化)、単一 SQLite DB の複数アカウント共有(スキーマ全面改修になるため per-account ディレクトリを採用)、ネイティブファイルピッカー(`tauri-plugin-dialog`)導入、`.nsec` legacy 読込経路の削除(sunset 条件は据え置き)、`LEGAL_BUNDLE_VERSION` の bump(「鍵は端末にのみ保存」の文言はエクスポートがユーザー明示操作である限り維持される)。

## 実装内容

- エクスポート形式(`crates/core/src/identity_export.rs`): argon2id(新規 workspace 依存)+ XChaCha20-Poly1305。`kukuri-account-key.v1.<base64url(JSON)>` envelope に version / KDF パラメータ / salt / nonce / fingerprint / ciphertext を持ち、version・KDF パラメータ・salt・fingerprint を AEAD の AAD に束縛して改竄・バージョン偽装を認証エラーで拒否する。パスフレーズ 8 文字以上。`preview` はパスフレーズなしで fingerprint のみ返す。インポート時の KDF パラメータには DoS 防止の上限を置く。
- アカウント registry と一括移行(`crates/desktop-runtime/src/accounts.rs`): `<app_data>/accounts.json`(非秘密メタデータのみ、原子的書込)と `accounts/<pubkey 先頭16hex>/kukuri.db` レイアウト。旧 flat レイアウトは「鍵複製→検証→registry 書込(commit point)→ファイル移動・keyring 再登録・optional secret 再キー→旧実体削除」の順で移行し、中断しても次回起動で冪等に再開する。鍵ファイルのない孤児 db は従来どおり新規生成で救済。
- ランタイム API / IPC: `export_account_key`(spawn_blocking、暗号化 envelope のみ返す)/ `preview_account_key_import`(`already_registered` 判定つき)/ `import_account_key`(復号→重複 pubkey 拒否→アカウント追加。既存アカウント無傷)/ `list_accounts` / `switch_account`。request DTO のパスフレーズは `Debug` で `<redacted>`。`cargo xtask ipc-types` 再生成。
- 再起動なしの切替(`apps/desktop/src-tauri`): `DesktopState` を `RwLock<Arc<DesktopRuntime>>` 化し、切替は「新 runtime 構築→registry 更新→スワップ→旧 runtime shutdown」。失敗時は旧 runtime が残る。通知ディスパッチタスクは runtime 入れ替えで再 subscribe し、OS 通知の pubkey キャッシュ / cursor をリセット。アプリ同意・年齢申告は base ディレクトリの端末レベル状態のまま(アカウント毎の再同意なし)。
- UI(`apps/desktop/src/components/settings/AccountKeyPanel.tsx`、設定 → アカウント): アカウント一覧(fingerprint 全桁表示・使用中バッジ・切替)、エクスポート(destructive 警告への明示同意 → パスフレーズ二重入力 → 暗号文表示・コピー)、インポート(preview で fingerprint 確認 → 登録済み鍵は追加不可 → 追加後に切替導線)。#855 との範囲差をパネル注記に明記。切替時は列下書き(localStorage)を破棄して再読み込み。i18n は ja / en / zh-CN の 3 ロケール。
- docs: ADR 0047、ADR 0002 分類、quickstart への鍵管理節の追記。

## 検証

- `cargo xtask ipc-types` 後の `git diff` 差分なし(CI 契約)
- Rust: `crates/core/src/tests/identity_export.rs` 13 件(round-trip、誤パスフレーズ、破損、切詰め、未知バージョン、fingerprint/KDF パラメータ改竄、KDF パラメータ上限、平文非包含、Debug redaction)。`crates/desktop-runtime/src/tests/accounts_migration.rs` 10 件(初回生成の安定性、flat 移行のファイル・identity・optional secret 移設、孤児 db 救済、移行中断の再開、迷子 flat identity の非アクティブ登録、追加/重複拒否/切替、preview の登録済み判定と秘密非包含、request Debug redaction)
- frontend: `AccountKeyPanel.test.tsx` 4 件(一覧と fingerprint、警告同意+パスフレーズ一致まで実行不可+平文非表示、登録済み鍵のインポート不可、追加成功→切替)、`src/i18n/parity.test.ts`、`pnpm lint` / `pnpm typecheck`
- 既知の環境要因: shell-integration 系 vitest の一部はこのリモート環境では HEAD 時点でも timeout する(コンテナ性能起因)。CI を一次結果とする。visual baseline は設定ドロワーのセクション追加分を `kukuri-visual-baseline` workflow で再生成して差し替える。

関連: #853(親)、#855(完全端末移行、Depends on #859)、ADR 0047
